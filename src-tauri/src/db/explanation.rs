use rusqlite::{params, OptionalExtension};

use super::error::{DbError, Result};
use super::models::{
    Claim, EvidenceLink, Explanation, ExplanationDetail, ExplanationSummary, OpenQuestion, State,
    Transition,
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

/// A state machine state supplied to [`Database::explanation_upsert`].
pub struct StateInput {
    pub stable_key: String,
    pub name: String,
    pub description: Option<String>,
    pub is_initial: bool,
    pub is_terminal: bool,
}

/// A state machine transition supplied to [`Database::explanation_upsert`].
pub struct TransitionInput {
    pub stable_key: String,
    pub from_state: String,
    pub to_state: String,
    pub event: Option<String>,
    pub guard: Option<String>,
    pub action: Option<String>,
    pub description: Option<String>,
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
    pub states: Vec<StateInput>,
    pub transitions: Vec<TransitionInput>,
    pub author: String,
    pub author_type: String,
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
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?10, ?9, ?9)",
                params![
                    id,
                    input.stable_key,
                    input.title,
                    input.explanation_type,
                    input.summary,
                    status,
                    confidence,
                    input.author,
                    ts,
                    input.author_type
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

        self.upsert_claims(
            &expl_id,
            &input.claims,
            &input.author,
            &input.author_type,
            &ts,
        )?;
        self.upsert_questions(
            &expl_id,
            &input.open_questions,
            &input.author,
            &input.author_type,
            &ts,
        )?;
        self.upsert_scope_links(
            &expl_id,
            &input.scope_item_ids,
            &input.author,
            &input.author_type,
            &ts,
        )?;
        self.upsert_states(
            &expl_id,
            &input.states,
            &input.author,
            &input.author_type,
            &ts,
        )?;
        self.upsert_transitions(
            &expl_id,
            &input.transitions,
            &input.author,
            &input.author_type,
            &ts,
        )?;
        Ok(expl_id)
    }

    fn upsert_states(
        &self,
        expl_id: &str,
        states: &[StateInput],
        author: &str,
        author_type: &str,
        ts: &str,
    ) -> Result<()> {
        for s in states {
            let existing: Option<String> = self
                .conn
                .query_row(
                    "SELECT id FROM states WHERE explanation_id = ?1 AND stable_key = ?2",
                    params![expl_id, s.stable_key],
                    |row| row.get(0),
                )
                .optional()?;
            let desc = s.description.as_deref().unwrap_or("");
            if let Some(id) = existing {
                self.conn.execute(
                    "UPDATE states SET name = ?2, description = ?3, is_initial = ?4,
                        is_terminal = ?5, updated_at = ?6 WHERE id = ?1",
                    params![id, s.name, desc, s.is_initial, s.is_terminal, ts],
                )?;
            } else {
                self.conn.execute(
                    "INSERT INTO states (id, explanation_id, stable_key, name, description,
                        is_initial, is_terminal, author, author_type, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                    params![
                        new_id(),
                        expl_id,
                        s.stable_key,
                        s.name,
                        desc,
                        s.is_initial,
                        s.is_terminal,
                        author,
                        author_type,
                        ts
                    ],
                )?;
            }
        }
        Ok(())
    }

    fn upsert_transitions(
        &self,
        expl_id: &str,
        transitions: &[TransitionInput],
        author: &str,
        author_type: &str,
        ts: &str,
    ) -> Result<()> {
        for t in transitions {
            let existing: Option<String> = self
                .conn
                .query_row(
                    "SELECT id FROM transitions WHERE explanation_id = ?1 AND stable_key = ?2",
                    params![expl_id, t.stable_key],
                    |row| row.get(0),
                )
                .optional()?;
            let event = t.event.as_deref().unwrap_or("");
            let desc = t.description.as_deref().unwrap_or("");
            if let Some(id) = existing {
                self.conn.execute(
                    "UPDATE transitions SET from_state = ?2, to_state = ?3, event = ?4,
                        guard = ?5, action = ?6, description = ?7, updated_at = ?8 WHERE id = ?1",
                    params![
                        id,
                        t.from_state,
                        t.to_state,
                        event,
                        t.guard,
                        t.action,
                        desc,
                        ts
                    ],
                )?;
            } else {
                self.conn.execute(
                    "INSERT INTO transitions (id, explanation_id, stable_key, from_state, to_state,
                        event, guard, action, description, author, author_type, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
                    params![
                        new_id(),
                        expl_id,
                        t.stable_key,
                        t.from_state,
                        t.to_state,
                        event,
                        t.guard,
                        t.action,
                        desc,
                        author,
                        author_type,
                        ts
                    ],
                )?;
            }
        }
        Ok(())
    }

    fn upsert_claims(
        &self,
        expl_id: &str,
        claims: &[ClaimInput],
        author: &str,
        author_type: &str,
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
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?10, ?9, ?9)",
                    params![
                        new_id(),
                        expl_id,
                        c.stable_key,
                        c.text,
                        claim_type,
                        cstatus,
                        cconf,
                        author,
                        ts,
                        author_type
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
        author_type: &str,
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
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?9, ?8, ?8)",
                    params![
                        new_id(),
                        expl_id,
                        q.stable_key,
                        q.question,
                        priority,
                        qstatus,
                        author,
                        ts,
                        author_type
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
        author_type: &str,
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
                     VALUES (?1, ?2, 'explanation', ?3, 'item', 'explains', '', ?4, ?6, ?5)",
                    params![new_id(), expl_id, item_id, author, ts, author_type],
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
        let states = self.get_states_for_explanation(id)?;
        let transitions = self.get_transitions_for_explanation(id)?;
        // Text diagram is generated on the fly for agents — never stored.
        let diagram_text = if states.is_empty() && transitions.is_empty() {
            None
        } else {
            Some(state_machine_text(
                &explanation.title,
                &states,
                &transitions,
            ))
        };
        Ok(ExplanationDetail {
            explanation,
            tags,
            scope_item_ids,
            claims,
            open_questions,
            evidence,
            states,
            transitions,
            diagram_text,
        })
    }

    fn get_states_for_explanation(&self, id: &str) -> Result<Vec<State>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, explanation_id, stable_key, name, description, is_initial, is_terminal,
                    author, author_type, created_at, updated_at
             FROM states WHERE explanation_id = ?1 ORDER BY is_initial DESC, created_at",
        )?;
        let rows = stmt
            .query_map(params![id], row_to_state)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn get_transitions_for_explanation(&self, id: &str) -> Result<Vec<Transition>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, explanation_id, stable_key, from_state, to_state, event, guard, action,
                    description, author, author_type, created_at, updated_at
             FROM transitions WHERE explanation_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt
            .query_map(params![id], row_to_transition)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
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

    /// Update envelope fields only (children have their own CRUD). `COALESCE`
    /// keeps any field passed as `None`.
    pub fn explanation_update(
        &self,
        id: &str,
        title: Option<&str>,
        explanation_type: Option<&str>,
        summary: Option<&str>,
        status: Option<&str>,
        confidence: Option<&str>,
    ) -> Result<Explanation> {
        self.get_explanation_by_id(id)?;
        self.conn.execute(
            "UPDATE explanations SET title = COALESCE(?2, title),
                explanation_type = COALESCE(?3, explanation_type),
                summary = COALESCE(?4, summary), status = COALESCE(?5, status),
                confidence = COALESCE(?6, confidence), updated_at = ?7 WHERE id = ?1",
            params![
                id,
                title,
                explanation_type,
                summary,
                status,
                confidence,
                now()
            ],
        )?;
        self.get_explanation_by_id(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn claim_create(
        &self,
        explanation_id: &str,
        text: &str,
        claim_type: Option<&str>,
        status: Option<&str>,
        confidence: Option<&str>,
        author: &str,
        author_type: &str,
    ) -> Result<Claim> {
        self.get_explanation_by_id(explanation_id)?;
        let id = new_id();
        let ts = now();
        self.conn.execute(
            "INSERT INTO claims (id, explanation_id, stable_key, text, claim_type, status,
                confidence, author, author_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                id,
                explanation_id,
                new_id(),
                text,
                claim_type.unwrap_or("behavior"),
                status.unwrap_or("hypothesis"),
                confidence.unwrap_or("medium"),
                author,
                author_type,
                ts
            ],
        )?;
        self.claim_by_id(&id)
    }

    pub fn claim_update(
        &self,
        id: &str,
        text: Option<&str>,
        claim_type: Option<&str>,
        status: Option<&str>,
        confidence: Option<&str>,
    ) -> Result<Claim> {
        let changes = self.conn.execute(
            "UPDATE claims SET text = COALESCE(?2, text), claim_type = COALESCE(?3, claim_type),
                status = COALESCE(?4, status), confidence = COALESCE(?5, confidence),
                updated_at = ?6 WHERE id = ?1",
            params![id, text, claim_type, status, confidence, now()],
        )?;
        if changes == 0 {
            return Err(DbError::NotFound {
                entity: "claim".to_string(),
                id: id.to_string(),
            });
        }
        self.claim_by_id(id)
    }

    pub fn claim_delete(&self, id: &str) -> Result<()> {
        self.delete_row("claims", "claim", id)
    }

    pub fn open_question_create(
        &self,
        explanation_id: &str,
        question: &str,
        priority: Option<&str>,
        status: Option<&str>,
        author: &str,
        author_type: &str,
    ) -> Result<OpenQuestion> {
        self.get_explanation_by_id(explanation_id)?;
        let id = new_id();
        let ts = now();
        self.conn.execute(
            "INSERT INTO open_questions (id, explanation_id, stable_key, question, priority,
                status, answer_claim_id, author, author_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?9)",
            params![
                id,
                explanation_id,
                new_id(),
                question,
                priority.unwrap_or("medium"),
                status.unwrap_or("open"),
                author,
                author_type,
                ts
            ],
        )?;
        self.question_by_id(&id)
    }

    pub fn open_question_update(
        &self,
        id: &str,
        question: Option<&str>,
        priority: Option<&str>,
        status: Option<&str>,
    ) -> Result<OpenQuestion> {
        let changes = self.conn.execute(
            "UPDATE open_questions SET question = COALESCE(?2, question),
                priority = COALESCE(?3, priority), status = COALESCE(?4, status),
                updated_at = ?5 WHERE id = ?1",
            params![id, question, priority, status, now()],
        )?;
        if changes == 0 {
            return Err(DbError::NotFound {
                entity: "open_question".to_string(),
                id: id.to_string(),
            });
        }
        self.question_by_id(id)
    }

    pub fn open_question_delete(&self, id: &str) -> Result<()> {
        self.delete_row("open_questions", "open_question", id)
    }

    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub fn state_create(
        &self,
        explanation_id: &str,
        name: &str,
        description: Option<&str>,
        is_initial: bool,
        is_terminal: bool,
        author: &str,
        author_type: &str,
    ) -> Result<State> {
        self.get_explanation_by_id(explanation_id)?;
        let id = new_id();
        let ts = now();
        self.conn.execute(
            "INSERT INTO states (id, explanation_id, stable_key, name, description, is_initial,
                is_terminal, author, author_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                id,
                explanation_id,
                new_id(),
                name,
                description.unwrap_or(""),
                is_initial,
                is_terminal,
                author,
                author_type,
                ts
            ],
        )?;
        self.state_by_id(&id)
    }

    pub fn state_update(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        is_initial: Option<bool>,
        is_terminal: Option<bool>,
    ) -> Result<State> {
        let changes = self.conn.execute(
            "UPDATE states SET name = COALESCE(?2, name), description = COALESCE(?3, description),
                is_initial = COALESCE(?4, is_initial), is_terminal = COALESCE(?5, is_terminal),
                updated_at = ?6 WHERE id = ?1",
            params![id, name, description, is_initial, is_terminal, now()],
        )?;
        if changes == 0 {
            return Err(DbError::NotFound {
                entity: "state".to_string(),
                id: id.to_string(),
            });
        }
        self.state_by_id(id)
    }

    /// Delete a state and any transitions that reference it (by `stable_key`
    /// within the same explanation), so the graph never dangles.
    pub fn state_delete(&self, id: &str) -> Result<()> {
        let row: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT explanation_id, stable_key FROM states WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((explanation_id, stable_key)) = row else {
            return Err(DbError::NotFound {
                entity: "state".to_string(),
                id: id.to_string(),
            });
        };
        self.conn.execute(
            "DELETE FROM transitions WHERE explanation_id = ?1 AND (from_state = ?2 OR to_state = ?2)",
            params![explanation_id, stable_key],
        )?;
        self.conn
            .execute("DELETE FROM states WHERE id = ?1", params![id])?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn transition_create(
        &self,
        explanation_id: &str,
        from_state: &str,
        to_state: &str,
        event: Option<&str>,
        guard: Option<&str>,
        action: Option<&str>,
        description: Option<&str>,
        author: &str,
        author_type: &str,
    ) -> Result<Transition> {
        self.get_explanation_by_id(explanation_id)?;
        self.require_state(explanation_id, from_state)?;
        self.require_state(explanation_id, to_state)?;
        let id = new_id();
        let ts = now();
        self.conn.execute(
            "INSERT INTO transitions (id, explanation_id, stable_key, from_state, to_state, event,
                guard, action, description, author, author_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
            params![
                id,
                explanation_id,
                new_id(),
                from_state,
                to_state,
                event.unwrap_or(""),
                guard,
                action,
                description.unwrap_or(""),
                author,
                author_type,
                ts
            ],
        )?;
        self.transition_by_id(&id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn transition_update(
        &self,
        id: &str,
        from_state: Option<&str>,
        to_state: Option<&str>,
        event: Option<&str>,
        guard: Option<&str>,
        action: Option<&str>,
        description: Option<&str>,
    ) -> Result<Transition> {
        let changes = self.conn.execute(
            "UPDATE transitions SET from_state = COALESCE(?2, from_state),
                to_state = COALESCE(?3, to_state), event = COALESCE(?4, event),
                guard = COALESCE(?5, guard), action = COALESCE(?6, action),
                description = COALESCE(?7, description), updated_at = ?8 WHERE id = ?1",
            params![
                id,
                from_state,
                to_state,
                event,
                guard,
                action,
                description,
                now()
            ],
        )?;
        if changes == 0 {
            return Err(DbError::NotFound {
                entity: "transition".to_string(),
                id: id.to_string(),
            });
        }
        self.transition_by_id(id)
    }

    pub fn transition_delete(&self, id: &str) -> Result<()> {
        self.delete_row("transitions", "transition", id)
    }

    fn require_state(&self, explanation_id: &str, stable_key: &str) -> Result<()> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM states WHERE explanation_id = ?1 AND stable_key = ?2)",
            params![explanation_id, stable_key],
            |r| r.get(0),
        )?;
        if exists {
            Ok(())
        } else {
            Err(DbError::InvalidReference {
                entity: "state".to_string(),
                id: stable_key.to_string(),
            })
        }
    }

    fn state_by_id(&self, id: &str) -> Result<State> {
        self.conn
            .query_row(
                "SELECT id, explanation_id, stable_key, name, description, is_initial, is_terminal,
                        author, author_type, created_at, updated_at FROM states WHERE id = ?1",
                params![id],
                row_to_state,
            )
            .map_err(map_not_found("state", id))
    }

    fn transition_by_id(&self, id: &str) -> Result<Transition> {
        self.conn
            .query_row(
                "SELECT id, explanation_id, stable_key, from_state, to_state, event, guard, action,
                        description, author, author_type, created_at, updated_at
                 FROM transitions WHERE id = ?1",
                params![id],
                row_to_transition,
            )
            .map_err(map_not_found("transition", id))
    }

    fn claim_by_id(&self, id: &str) -> Result<Claim> {
        self.conn
            .query_row(
                "SELECT id, explanation_id, stable_key, text, claim_type, status, confidence,
                        author, author_type, created_at, updated_at FROM claims WHERE id = ?1",
                params![id],
                row_to_claim,
            )
            .map_err(map_not_found("claim", id))
    }

    fn question_by_id(&self, id: &str) -> Result<OpenQuestion> {
        self.conn
            .query_row(
                "SELECT id, explanation_id, stable_key, question, priority, status, answer_claim_id,
                        author, author_type, created_at, updated_at FROM open_questions WHERE id = ?1",
                params![id],
                row_to_question,
            )
            .map_err(map_not_found("open_question", id))
    }

    fn delete_row(&self, table: &str, entity: &str, id: &str) -> Result<()> {
        let changes = self
            .conn
            .execute(&format!("DELETE FROM {table} WHERE id = ?1"), params![id])?;
        if changes == 0 {
            return Err(DbError::NotFound {
                entity: entity.to_string(),
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

fn map_not_found<'a>(entity: &'a str, id: &'a str) -> impl Fn(rusqlite::Error) -> DbError + 'a {
    move |e| match e {
        rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
            entity: entity.to_string(),
            id: id.to_string(),
        },
        other => DbError::Sqlite(other),
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

fn row_to_state(row: &rusqlite::Row) -> rusqlite::Result<State> {
    Ok(State {
        id: row.get(0)?,
        explanation_id: row.get(1)?,
        stable_key: row.get(2)?,
        name: row.get(3)?,
        description: row.get(4)?,
        is_initial: row.get(5)?,
        is_terminal: row.get(6)?,
        author: row.get(7)?,
        author_type: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn row_to_transition(row: &rusqlite::Row) -> rusqlite::Result<Transition> {
    Ok(Transition {
        id: row.get(0)?,
        explanation_id: row.get(1)?,
        stable_key: row.get(2)?,
        from_state: row.get(3)?,
        to_state: row.get(4)?,
        event: row.get(5)?,
        guard: row.get(6)?,
        action: row.get(7)?,
        description: row.get(8)?,
        author: row.get(9)?,
        author_type: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

/// Generate a compact text rendering of a state machine for agents. Generated on
/// the fly from the editable rows; never stored. State keys resolve to names.
fn state_machine_text(title: &str, states: &[State], transitions: &[Transition]) -> String {
    use std::fmt::Write;
    let name_of = |key: &str| {
        states
            .iter()
            .find(|s| s.stable_key == key)
            .map_or_else(|| key.to_string(), |s| s.name.clone())
    };
    let mut out = format!("State machine: {title}\n");
    let initial: Vec<&str> = states
        .iter()
        .filter(|s| s.is_initial)
        .map(|s| s.name.as_str())
        .collect();
    if !initial.is_empty() {
        let _ = writeln!(out, "Initial: {}", initial.join(", "));
    }
    let _ = writeln!(out, "States ({}):", states.len());
    for s in states {
        let mut marks = Vec::new();
        if s.is_initial {
            marks.push("initial");
        }
        if s.is_terminal {
            marks.push("terminal");
        }
        let suffix = if marks.is_empty() {
            String::new()
        } else {
            format!(" [{}]", marks.join(", "))
        };
        let _ = writeln!(out, "  - {}{suffix}", s.name);
    }
    let _ = writeln!(out, "Transitions ({}):", transitions.len());
    for t in transitions {
        let guard = t
            .guard
            .as_deref()
            .filter(|g| !g.is_empty())
            .map_or_else(String::new, |g| format!(" [{g}]"));
        let action = t
            .action
            .as_deref()
            .filter(|a| !a.is_empty())
            .map_or_else(String::new, |a| format!(" / {a}"));
        let _ = writeln!(
            out,
            "  {} --{}{guard}{action}--> {}",
            name_of(&t.from_state),
            t.event,
            name_of(&t.to_state),
        );
    }
    out
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
            states: Vec::new(),
            transitions: Vec::new(),
            author: "claude".to_string(),
            author_type: "agent".to_string(),
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
            author_type: "human",
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
            author_type: "human",
        });
        assert!(matches!(err, Err(DbError::InvalidReference { .. })));
    }
}
