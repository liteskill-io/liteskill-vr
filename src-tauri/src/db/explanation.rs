use rusqlite::{params, OptionalExtension};

use super::error::{DbError, Result};
use super::models::{
    Claim, EvidenceLink, Explanation, ExplanationDetail, ExplanationSummary, OpenQuestion,
};
use super::{new_id, now, parse_tag_list, Database};

/// A claim supplied to [`Database::explanation_upsert`].
pub struct ClaimInput {
    pub stable_key: String,
    pub text: String,
    pub claim_type: Option<String>,
    pub status: Option<String>,
    pub confidence: Option<String>,
}

/// An open question supplied to [`Database::explanation_upsert`].
pub struct QuestionInput {
    pub stable_key: String,
    pub question: String,
    pub priority: Option<String>,
    pub status: Option<String>,
}

/// The full nested input for an idempotent explanation upsert.
pub struct ExplanationInput {
    pub stable_key: String,
    pub title: String,
    pub explanation_type: String,
    pub summary: String,
    pub status: Option<String>,
    pub confidence: Option<String>,
    pub tags: Vec<String>,
    pub scope_item_ids: Vec<String>,
    pub claims: Vec<ClaimInput>,
    pub open_questions: Vec<QuestionInput>,
    pub author: String,
}

/// Result of an upsert.
///
/// Carries the resulting detail plus non-fatal advisory warnings (e.g. the
/// "prose dump" smell — a long summary with no structured claims).
pub struct UpsertResult {
    pub detail: ExplanationDetail,
    pub warnings: Vec<String>,
}

// Summaries longer than this with no claims are flagged as a prose dump.
const PROSE_DUMP_SUMMARY_CHARS: usize = 600;

impl Database {
    /// Create or update an explanation (by `stable_key`) together with its
    /// claims, open questions, and scope links — all-or-nothing.
    pub fn explanation_upsert(&self, input: &ExplanationInput) -> Result<UpsertResult> {
        self.validate_tags(&input.tags)?;
        for item_id in &input.scope_item_ids {
            let exists: bool = self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM items WHERE id = ?1)",
                params![item_id],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(DbError::InvalidReference {
                    entity: "item".to_string(),
                    id: item_id.clone(),
                });
            }
        }

        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match self.explanation_upsert_inner(input) {
            Ok(id) => {
                self.conn.execute_batch("COMMIT")?;
                let detail = self.explanation_get(&id)?;
                let warnings = explanation_warnings(&detail);
                Ok(UpsertResult { detail, warnings })
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn explanation_upsert_inner(&self, input: &ExplanationInput) -> Result<String> {
        let ts = now();
        let status = input.status.as_deref().unwrap_or("draft");
        let confidence = input.confidence.as_deref().unwrap_or("medium");

        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM explanations WHERE stable_key = ?1",
                params![input.stable_key],
                |row| row.get(0),
            )
            .optional()?;

        let expl_id = if let Some(id) = existing {
            self.conn.execute(
                "UPDATE explanations SET title = ?2, explanation_type = ?3, summary = ?4,
                    status = ?5, confidence = ?6, updated_at = ?7 WHERE id = ?1",
                params![
                    id,
                    input.title,
                    input.explanation_type,
                    input.summary,
                    status,
                    confidence,
                    ts
                ],
            )?;
            id
        } else {
            let id = new_id();
            self.conn.execute(
                "INSERT INTO explanations (id, stable_key, title, explanation_type, summary,
                    status, confidence, author, author_type, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'agent', ?9, ?9)",
                params![
                    id,
                    input.stable_key,
                    input.title,
                    input.explanation_type,
                    input.summary,
                    status,
                    confidence,
                    input.author,
                    ts
                ],
            )?;
            id
        };

        self.conn.execute(
            "DELETE FROM explanation_tags WHERE explanation_id = ?1",
            params![expl_id],
        )?;
        for tag in &input.tags {
            self.conn.execute(
                "INSERT INTO explanation_tags (explanation_id, tag_name) VALUES (?1, ?2)",
                params![expl_id, tag],
            )?;
        }

        self.upsert_claims(&expl_id, &input.claims, &input.author, &ts)?;
        self.upsert_questions(&expl_id, &input.open_questions, &input.author, &ts)?;
        self.upsert_scope_links(&expl_id, &input.scope_item_ids, &input.author, &ts)?;
        Ok(expl_id)
    }

    fn upsert_claims(
        &self,
        expl_id: &str,
        claims: &[ClaimInput],
        author: &str,
        ts: &str,
    ) -> Result<()> {
        for c in claims {
            let existing: Option<String> = self
                .conn
                .query_row(
                    "SELECT id FROM claims WHERE explanation_id = ?1 AND stable_key = ?2",
                    params![expl_id, c.stable_key],
                    |row| row.get(0),
                )
                .optional()?;
            let claim_type = c.claim_type.as_deref().unwrap_or("behavior");
            let cstatus = c.status.as_deref().unwrap_or("hypothesis");
            let cconf = c.confidence.as_deref().unwrap_or("medium");
            if let Some(cid) = existing {
                self.conn.execute(
                    "UPDATE claims SET text = ?2, claim_type = ?3, status = ?4,
                        confidence = ?5, updated_at = ?6 WHERE id = ?1",
                    params![cid, c.text, claim_type, cstatus, cconf, ts],
                )?;
            } else {
                self.conn.execute(
                    "INSERT INTO claims (id, explanation_id, stable_key, text, claim_type,
                        status, confidence, author, author_type, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'agent', ?9, ?9)",
                    params![
                        new_id(),
                        expl_id,
                        c.stable_key,
                        c.text,
                        claim_type,
                        cstatus,
                        cconf,
                        author,
                        ts
                    ],
                )?;
            }
        }
        Ok(())
    }

    fn upsert_questions(
        &self,
        expl_id: &str,
        questions: &[QuestionInput],
        author: &str,
        ts: &str,
    ) -> Result<()> {
        for q in questions {
            let existing: Option<String> = self
                .conn
                .query_row(
                    "SELECT id FROM open_questions WHERE explanation_id = ?1 AND stable_key = ?2",
                    params![expl_id, q.stable_key],
                    |row| row.get(0),
                )
                .optional()?;
            let priority = q.priority.as_deref().unwrap_or("medium");
            let qstatus = q.status.as_deref().unwrap_or("open");
            if let Some(qid) = existing {
                self.conn.execute(
                    "UPDATE open_questions SET question = ?2, priority = ?3, status = ?4,
                        updated_at = ?5 WHERE id = ?1",
                    params![qid, q.question, priority, qstatus, ts],
                )?;
            } else {
                self.conn.execute(
                    "INSERT INTO open_questions (id, explanation_id, stable_key, question,
                        priority, status, answer_claim_id, author, author_type, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, 'agent', ?8, ?8)",
                    params![
                        new_id(),
                        expl_id,
                        q.stable_key,
                        q.question,
                        priority,
                        qstatus,
                        author,
                        ts
                    ],
                )?;
            }
        }
        Ok(())
    }

    // Scope links reuse the connections table (explanation --explains--> item).
    fn upsert_scope_links(
        &self,
        expl_id: &str,
        item_ids: &[String],
        author: &str,
        ts: &str,
    ) -> Result<()> {
        for item_id in item_ids {
            let exists: bool = self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM connections WHERE source_id = ?1
                    AND source_type = 'explanation' AND target_id = ?2
                    AND target_type = 'item' AND connection_type = 'explains')",
                params![expl_id, item_id],
                |row| row.get(0),
            )?;
            if !exists {
                self.conn.execute(
                    "INSERT INTO connections (id, source_id, source_type, target_id, target_type,
                        connection_type, description, author, author_type, created_at)
                     VALUES (?1, ?2, 'explanation', ?3, 'item', 'explains', '', ?4, 'agent', ?5)",
                    params![new_id(), expl_id, item_id, author, ts],
                )?;
            }
        }
        Ok(())
    }

    pub fn explanation_get(&self, id: &str) -> Result<ExplanationDetail> {
        let explanation = self.get_explanation_by_id(id)?;
        let tags = self.get_explanation_tags(id)?;
        let scope_item_ids = self.get_explanation_scope(id)?;
        let claims = self.get_claims_for_explanation(id)?;
        let open_questions = self.get_questions_for_explanation(id)?;
        let evidence = self.get_evidence_for_explanation(id)?;
        Ok(ExplanationDetail {
            explanation,
            tags,
            scope_item_ids,
            claims,
            open_questions,
            evidence,
        })
    }

    pub fn explanation_list(
        &self,
        explanation_type: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<ExplanationSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.stable_key, e.title, e.explanation_type, e.summary, e.status,
                    e.confidence, e.author, e.author_type, e.created_at, e.updated_at,
                    (SELECT COUNT(*) FROM claims WHERE explanation_id = e.id),
                    (SELECT COUNT(*) FROM open_questions WHERE explanation_id = e.id AND status = 'open'),
                    (SELECT GROUP_CONCAT(tag_name, char(31)) FROM
                        (SELECT tag_name FROM explanation_tags WHERE explanation_id = e.id ORDER BY tag_name))
             FROM explanations e
             WHERE (?1 IS NULL OR e.explanation_type = ?1)
               AND (?2 IS NULL OR e.status = ?2)
             ORDER BY e.updated_at DESC",
        )?;
        let rows = stmt
            .query_map(params![explanation_type, status], |row| {
                Ok((
                    row_to_explanation(row),
                    parse_tag_list(row.get(13)?),
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut out = Vec::with_capacity(rows.len());
        for (explanation, tags, claim_count, open_question_count) in rows {
            let scope_item_ids = self.get_explanation_scope(&explanation.id)?;
            let evidence_count = self.evidence_count_for_explanation(&explanation.id)?;
            out.push(ExplanationSummary {
                explanation,
                tags,
                scope_item_ids,
                claim_count,
                open_question_count,
                evidence_count,
            });
        }
        Ok(out)
    }

    /// Open questions across the project, newest first — backs `filter`.
    pub fn open_questions_list(
        &self,
        priority: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<OpenQuestion>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, explanation_id, stable_key, question, priority, status, answer_claim_id,
                    author, author_type, created_at, updated_at
             FROM open_questions
             WHERE (?1 IS NULL OR priority = ?1) AND (?2 IS NULL OR status = ?2)
             ORDER BY updated_at DESC",
        )?;
        let qs = stmt
            .query_map(params![priority, status], row_to_question)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(qs)
    }

    pub fn explanation_delete(&self, id: &str) -> Result<()> {
        let changes = self
            .conn
            .execute("DELETE FROM explanations WHERE id = ?1", params![id])?;
        if changes == 0 {
            return Err(DbError::NotFound {
                entity: "explanation".to_string(),
                id: id.to_string(),
            });
        }
        Ok(())
    }

    fn get_explanation_by_id(&self, id: &str) -> Result<Explanation> {
        self.conn
            .query_row(
                "SELECT id, stable_key, title, explanation_type, summary, status, confidence,
                        author, author_type, created_at, updated_at
                 FROM explanations WHERE id = ?1",
                params![id],
                |row| Ok(row_to_explanation(row)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                    entity: "explanation".to_string(),
                    id: id.to_string(),
                },
                other => DbError::Sqlite(other),
            })
    }

    fn get_explanation_tags(&self, id: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT tag_name FROM explanation_tags WHERE explanation_id = ?1 ORDER BY tag_name",
        )?;
        let tags = stmt
            .query_map(params![id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(tags)
    }

    fn get_explanation_scope(&self, id: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT target_id FROM connections WHERE source_id = ?1
                AND source_type = 'explanation' AND target_type = 'item'
                AND connection_type = 'explains' ORDER BY target_id",
        )?;
        let ids = stmt
            .query_map(params![id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(ids)
    }

    fn get_claims_for_explanation(&self, id: &str) -> Result<Vec<Claim>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, explanation_id, stable_key, text, claim_type, status, confidence,
                    author, author_type, created_at, updated_at
             FROM claims WHERE explanation_id = ?1 ORDER BY created_at",
        )?;
        let claims = stmt
            .query_map(params![id], row_to_claim)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(claims)
    }

    fn get_questions_for_explanation(&self, id: &str) -> Result<Vec<OpenQuestion>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, explanation_id, stable_key, question, priority, status, answer_claim_id,
                    author, author_type, created_at, updated_at
             FROM open_questions WHERE explanation_id = ?1 ORDER BY created_at",
        )?;
        let qs = stmt
            .query_map(params![id], row_to_question)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(qs)
    }

    fn get_evidence_for_explanation(&self, id: &str) -> Result<Vec<EvidenceLink>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, target_type, target_id, source_entity_type, source_entity_id,
                    external_locator, external_kind, evidence_type, strength, excerpt,
                    author, author_type, created_at
             FROM evidence_links
             WHERE (target_type = 'explanation' AND target_id = ?1)
                OR (target_type = 'claim' AND target_id IN
                    (SELECT id FROM claims WHERE explanation_id = ?1))
             ORDER BY created_at",
        )?;
        let ev = stmt
            .query_map(params![id], row_to_evidence)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(ev)
    }

    fn evidence_count_for_explanation(&self, id: &str) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM evidence_links
             WHERE (target_type = 'explanation' AND target_id = ?1)
                OR (target_type = 'claim' AND target_id IN
                    (SELECT id FROM claims WHERE explanation_id = ?1))",
            params![id],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

fn row_to_explanation(row: &rusqlite::Row) -> Explanation {
    Explanation {
        id: row.get_unwrap(0),
        stable_key: row.get_unwrap(1),
        title: row.get_unwrap(2),
        explanation_type: row.get_unwrap(3),
        summary: row.get_unwrap(4),
        status: row.get_unwrap(5),
        confidence: row.get_unwrap(6),
        author: row.get_unwrap(7),
        author_type: row.get_unwrap(8),
        created_at: row.get_unwrap(9),
        updated_at: row.get_unwrap(10),
    }
}

fn row_to_claim(row: &rusqlite::Row) -> rusqlite::Result<Claim> {
    Ok(Claim {
        id: row.get(0)?,
        explanation_id: row.get(1)?,
        stable_key: row.get(2)?,
        text: row.get(3)?,
        claim_type: row.get(4)?,
        status: row.get(5)?,
        confidence: row.get(6)?,
        author: row.get(7)?,
        author_type: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn row_to_question(row: &rusqlite::Row) -> rusqlite::Result<OpenQuestion> {
    Ok(OpenQuestion {
        id: row.get(0)?,
        explanation_id: row.get(1)?,
        stable_key: row.get(2)?,
        question: row.get(3)?,
        priority: row.get(4)?,
        status: row.get(5)?,
        answer_claim_id: row.get(6)?,
        author: row.get(7)?,
        author_type: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn row_to_evidence(row: &rusqlite::Row) -> rusqlite::Result<EvidenceLink> {
    Ok(EvidenceLink {
        id: row.get(0)?,
        target_type: row.get(1)?,
        target_id: row.get(2)?,
        source_entity_type: row.get(3)?,
        source_entity_id: row.get(4)?,
        external_locator: row.get(5)?,
        external_kind: row.get(6)?,
        evidence_type: row.get(7)?,
        strength: row.get(8)?,
        excerpt: row.get(9)?,
        author: row.get(10)?,
        author_type: row.get(11)?,
        created_at: row.get(12)?,
    })
}

// The anti-wiki guardrail: surface advisory warnings, never block the write.
fn explanation_warnings(d: &ExplanationDetail) -> Vec<String> {
    let mut warnings = Vec::new();
    if d.explanation.summary.chars().count() > PROSE_DUMP_SUMMARY_CHARS && d.claims.is_empty() {
        warnings.push(
            "Long summary with no claims — prefer structured, evidence-backed claims over a prose dump.".to_string(),
        );
    }
    let evidenced: std::collections::HashSet<&str> = d
        .evidence
        .iter()
        .filter(|e| e.target_type == "claim")
        .map(|e| e.target_id.as_str())
        .collect();
    let unbacked = d
        .claims
        .iter()
        .filter(|c| !evidenced.contains(c.id.as_str()))
        .count();
    if unbacked > 0 {
        warnings.push(format!(
            "{unbacked} claim(s) have no linked evidence — attach evidence or lower confidence."
        ));
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NewEvidence;

    fn test_db() -> Database {
        Database::in_memory("test").unwrap()
    }

    fn claim(key: &str, text: &str) -> ClaimInput {
        ClaimInput {
            stable_key: key.to_string(),
            text: text.to_string(),
            claim_type: None,
            status: None,
            confidence: None,
        }
    }

    fn base_input(stable_key: &str, title: &str) -> ExplanationInput {
        ExplanationInput {
            stable_key: stable_key.to_string(),
            title: title.to_string(),
            explanation_type: "architecture".to_string(),
            summary: "short tldr".to_string(),
            status: None,
            confidence: None,
            tags: Vec::new(),
            scope_item_ids: Vec::new(),
            claims: Vec::new(),
            open_questions: Vec::new(),
            author: "claude".to_string(),
        }
    }

    #[test]
    fn upsert_is_idempotent_by_stable_key() {
        let db = test_db();
        let mut input = base_input("explanation.auth", "Auth flow");
        input.claims = vec![claim("claim.uses_rsa", "Auth uses RSA")];
        input.open_questions = vec![QuestionInput {
            stable_key: "q.len_bound".to_string(),
            question: "Is length bounded?".to_string(),
            priority: Some("high".to_string()),
            status: None,
        }];

        let first = db.explanation_upsert(&input).unwrap();
        let first_id = first.detail.explanation.id.clone();
        assert_eq!(first.detail.claims.len(), 1);
        assert_eq!(first.detail.open_questions.len(), 1);

        // Re-run with an updated claim text + a new claim: same explanation row,
        // claim updated in place, no duplicates.
        input.title = "Authentication flow".to_string();
        input.claims = vec![
            claim("claim.uses_rsa", "Auth uses RSA-2048"),
            claim("claim.nonce", "Server issues a nonce"),
        ];
        let second = db.explanation_upsert(&input).unwrap();
        assert_eq!(second.detail.explanation.id, first_id, "same row reused");
        assert_eq!(second.detail.explanation.title, "Authentication flow");
        assert_eq!(second.detail.claims.len(), 2, "no duplicate claim");
        let rsa = second
            .detail
            .claims
            .iter()
            .find(|c| c.stable_key == "claim.uses_rsa")
            .unwrap();
        assert_eq!(rsa.text, "Auth uses RSA-2048");

        assert_eq!(db.explanation_list(None, None).unwrap().len(), 1);
    }

    #[test]
    fn scope_links_dedupe_across_runs() {
        let db = test_db();
        let item = db.item_create("httpd", "elf", None, None, "", &[]).unwrap();
        let mut input = base_input("explanation.x", "X");
        input.scope_item_ids = vec![item.item.id.clone()];

        let first = db.explanation_upsert(&input).unwrap();
        assert_eq!(first.detail.scope_item_ids, vec![item.item.id.clone()]);
        // Re-run: scope connection must not be duplicated.
        let second = db.explanation_upsert(&input).unwrap();
        assert_eq!(second.detail.scope_item_ids.len(), 1);
        let conns = db.connection_list(&item.item.id, Some("explains")).unwrap();
        assert_eq!(conns.len(), 1);
    }

    #[test]
    fn invalid_scope_item_rolls_back() {
        let db = test_db();
        let mut input = base_input("explanation.x", "X");
        input.scope_item_ids = vec!["does-not-exist".to_string()];
        let result = db.explanation_upsert(&input);
        assert!(matches!(result, Err(DbError::InvalidReference { .. })));
        // Nothing persisted.
        assert!(db.explanation_list(None, None).unwrap().is_empty());
    }

    #[test]
    fn prose_dump_and_unbacked_claim_warnings() {
        let db = test_db();
        let mut input = base_input("explanation.blob", "Blob");
        input.summary = "x".repeat(700);
        let res = db.explanation_upsert(&input).unwrap();
        assert!(res.warnings.iter().any(|w| w.contains("Long summary")));

        // Add a claim → now the unbacked-claim warning fires, prose-dump clears.
        input.claims = vec![claim("claim.a", "Some claim")];
        let res = db.explanation_upsert(&input).unwrap();
        assert!(res
            .warnings
            .iter()
            .any(|w| w.contains("no linked evidence")));
        assert!(!res.warnings.iter().any(|w| w.contains("Long summary")));
    }

    #[test]
    fn evidence_clears_unbacked_warning_and_shows_in_detail() {
        let db = test_db();
        let mut input = base_input("explanation.e", "E");
        input.summary = "short".to_string();
        input.claims = vec![claim("claim.a", "A claim")];
        let res = db.explanation_upsert(&input).unwrap();
        let claim_id = res.detail.claims[0].id.clone();

        db.evidence_link(&NewEvidence {
            target_type: "claim",
            target_id: &claim_id,
            source_entity_type: None,
            source_entity_id: None,
            external_locator: Some("FUN_00401000+0x14"),
            external_kind: Some("ghidra"),
            evidence_type: "decompilation",
            strength: "strong",
            excerpt: None,
            author: "claude",
        })
        .unwrap();

        let res = db.explanation_upsert(&input).unwrap();
        assert!(!res
            .warnings
            .iter()
            .any(|w| w.contains("no linked evidence")));
        assert_eq!(res.detail.evidence.len(), 1);
        assert_eq!(
            res.detail.evidence[0].external_locator.as_deref(),
            Some("FUN_00401000+0x14")
        );
    }

    #[test]
    fn evidence_requires_a_source() {
        let db = test_db();
        let res = db
            .explanation_upsert(&base_input("explanation.e", "E"))
            .unwrap();
        let id = res.detail.explanation.id;
        let err = db.evidence_link(&NewEvidence {
            target_type: "explanation",
            target_id: &id,
            source_entity_type: None,
            source_entity_id: None,
            external_locator: None,
            external_kind: None,
            evidence_type: "agent_inference",
            strength: "weak",
            excerpt: None,
            author: "claude",
        });
        assert!(matches!(err, Err(DbError::InvalidReference { .. })));
    }
}
