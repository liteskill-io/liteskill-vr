//! Dev/test fixture loader.
//!
//! Applies a JSON project document (see `fixtures/demo-project.json`) to a
//! [`Database`] through the real db layer, so seed data is always schema- and
//! validation-correct. Used by the `seed` dev binary (`task dev:seed`) and tests.
//!
//! Cross-references use local string keys: items have a `key`, items of interest
//! an optional `key`, claims a `key`. Connection/evidence/scope references
//! resolve those keys to the real generated ids. Evidence `target` is one of
//! `self`, `claim:<key>`, or `finding:<ioi-key>`.

use std::collections::HashMap;

use serde_json::Value;

use crate::db::{
    ClaimInput, Database, ExplanationInput, NewConnection, NewEvidence, NewIoi, QuestionInput,
    StateInput, TransitionInput,
};

#[derive(Debug, Default)]
pub struct SeedStats {
    pub tags: usize,
    pub connection_types: usize,
    pub items: usize,
    pub notes: usize,
    pub iois: usize,
    pub connections: usize,
    pub explanations: usize,
    pub claims: usize,
    pub open_questions: usize,
    pub states: usize,
    pub transitions: usize,
    pub evidence: usize,
}

fn str_field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

fn array<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key)
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

fn tag_list(v: &Value) -> Vec<String> {
    array(v, "tags")
        .iter()
        .filter_map(Value::as_str)
        .map(String::from)
        .collect()
}

// Author identity for a fixture entity; defaults to an agent so seed data looks
// like agent output unless explicitly marked human.
fn author(v: &Value) -> (&str, &str) {
    (
        str_field(v, "author").unwrap_or("demo-agent"),
        str_field(v, "author_type").unwrap_or("agent"),
    )
}

/// Apply a fixture document to `db`. Tags / connection types that already exist
/// are skipped (idempotent for vocabularies); everything else is created fresh.
#[allow(clippy::too_many_lines)]
pub fn apply(db: &Database, doc: &Value) -> Result<SeedStats, String> {
    use crate::db::error::DbError;
    let mut stats = SeedStats::default();
    let emap = |e: DbError| e.to_string();

    for t in array(doc, "tags") {
        let name = str_field(t, "name").ok_or("tag missing 'name'")?;
        match db.tag_create(
            name,
            str_field(t, "description").unwrap_or(""),
            str_field(t, "color"),
        ) {
            Ok(_) => stats.tags += 1,
            Err(DbError::DuplicateName { .. }) => {}
            Err(e) => return Err(emap(e)),
        }
    }

    for c in array(doc, "connection_types") {
        let name = str_field(c, "name").ok_or("connection_type missing 'name'")?;
        match db.connection_type_create(name, str_field(c, "description").unwrap_or("")) {
            Ok(_) => stats.connection_types += 1,
            Err(DbError::DuplicateName { .. }) => {}
            Err(e) => return Err(emap(e)),
        }
    }

    // key -> generated id maps for cross-references.
    let mut item_ids: HashMap<String, String> = HashMap::new();
    let mut ioi_ids: HashMap<String, String> = HashMap::new();

    for it in array(doc, "items") {
        let name = str_field(it, "name").ok_or("item missing 'name'")?;
        let key = str_field(it, "key").unwrap_or(name);
        let created = db
            .item_create(
                name,
                str_field(it, "item_type").unwrap_or("unknown"),
                str_field(it, "path"),
                str_field(it, "architecture"),
                str_field(it, "description").unwrap_or(""),
                &tag_list(it),
            )
            .map_err(emap)?;
        let item_id = created.item.id.clone();
        if let Some(status) = str_field(it, "analysis_status") {
            if status != "untouched" {
                db.item_update(&item_id, None, None, Some(status), None)
                    .map_err(emap)?;
            }
        }
        item_ids.insert(key.to_string(), item_id.clone());
        stats.items += 1;

        for n in array(it, "notes") {
            let (au, at) = author(n);
            db.note_create(
                Some(&item_id),
                str_field(n, "title").unwrap_or("note"),
                str_field(n, "content").unwrap_or(""),
                au,
                at,
                &tag_list(n),
            )
            .map_err(emap)?;
            stats.notes += 1;
        }

        for o in array(it, "iois") {
            let (au, at) = author(o);
            let (ioi, _) = db
                .ioi_create(&NewIoi {
                    item_id: &item_id,
                    title: str_field(o, "title").unwrap_or("finding"),
                    description: str_field(o, "description").unwrap_or(""),
                    location: str_field(o, "location"),
                    severity: str_field(o, "severity"),
                    status: str_field(o, "status"),
                    author: au,
                    author_type: at,
                    tags: &tag_list(o),
                })
                .map_err(emap)?;
            if let Some(k) = str_field(o, "key") {
                ioi_ids.insert(k.to_string(), ioi.ioi.id.clone());
            }
            stats.iois += 1;
        }
    }

    let resolve = |key: &str| -> Result<String, String> {
        item_ids
            .get(key)
            .or_else(|| ioi_ids.get(key))
            .cloned()
            .ok_or_else(|| format!("unknown entity ref '{key}'"))
    };

    for c in array(doc, "connections") {
        let (au, at) = author(c);
        let source = resolve(str_field(c, "source").ok_or("connection missing 'source'")?)?;
        let target = resolve(str_field(c, "target").ok_or("connection missing 'target'")?)?;
        db.connection_create(&NewConnection {
            source_id: &source,
            source_type: str_field(c, "source_type").unwrap_or("item"),
            target_id: &target,
            target_type: str_field(c, "target_type").unwrap_or("item"),
            connection_type: str_field(c, "connection_type")
                .ok_or("connection missing 'connection_type'")?,
            description: str_field(c, "description").unwrap_or(""),
            author: au,
            author_type: at,
        })
        .map_err(emap)?;
        stats.connections += 1;
    }

    for e in array(doc, "explanations") {
        let (au, at) = author(e);
        let scope: Vec<String> = array(e, "scope")
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|k| item_ids.get(k).cloned())
            .collect();
        let claims: Vec<ClaimInput> = array(e, "claims")
            .iter()
            .map(|c| ClaimInput {
                stable_key: str_field(c, "key").unwrap_or("claim").to_string(),
                text: str_field(c, "text").unwrap_or("").to_string(),
                claim_type: str_field(c, "claim_type").map(String::from),
                status: str_field(c, "status").map(String::from),
                confidence: str_field(c, "confidence").map(String::from),
            })
            .collect();
        let open_questions: Vec<QuestionInput> = array(e, "open_questions")
            .iter()
            .enumerate()
            .map(|(i, q)| QuestionInput {
                stable_key: str_field(q, "key").map_or_else(|| format!("q.{i}"), String::from),
                question: str_field(q, "question").unwrap_or("").to_string(),
                priority: str_field(q, "priority").map(String::from),
                status: str_field(q, "status").map(String::from),
            })
            .collect();
        let state_inputs: Vec<StateInput> = array(e, "states")
            .iter()
            .map(|s| StateInput {
                stable_key: str_field(s, "key").unwrap_or("state").to_string(),
                name: str_field(s, "name").unwrap_or("").to_string(),
                description: str_field(s, "description").map(String::from),
                is_initial: s
                    .get("is_initial")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                is_terminal: s
                    .get("is_terminal")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
            .collect();
        let transition_inputs: Vec<TransitionInput> = array(e, "transitions")
            .iter()
            .map(|t| TransitionInput {
                stable_key: str_field(t, "key").unwrap_or("transition").to_string(),
                from_state: str_field(t, "from").unwrap_or("").to_string(),
                to_state: str_field(t, "to").unwrap_or("").to_string(),
                event: str_field(t, "event").map(String::from),
                guard: str_field(t, "guard").map(String::from),
                action: str_field(t, "action").map(String::from),
                description: str_field(t, "description").map(String::from),
            })
            .collect();

        let result = db
            .explanation_upsert(&ExplanationInput {
                stable_key: str_field(e, "stable_key")
                    .ok_or("explanation missing 'stable_key'")?
                    .to_string(),
                title: str_field(e, "title").unwrap_or("Explanation").to_string(),
                explanation_type: str_field(e, "explanation_type")
                    .unwrap_or("custom")
                    .to_string(),
                summary: str_field(e, "summary").unwrap_or("").to_string(),
                status: str_field(e, "status").map(String::from),
                confidence: str_field(e, "confidence").map(String::from),
                tags: tag_list(e),
                scope_item_ids: scope,
                claims,
                open_questions,
                states: state_inputs,
                transitions: transition_inputs,
                author: au.to_string(),
                author_type: at.to_string(),
            })
            .map_err(emap)?;
        stats.explanations += 1;
        stats.claims += result.detail.claims.len();
        stats.open_questions += result.detail.open_questions.len();
        stats.states += result.detail.states.len();
        stats.transitions += result.detail.transitions.len();

        for ev in array(e, "evidence") {
            let target = str_field(ev, "target").unwrap_or("self");
            let (target_type, target_id) = if target == "self" {
                ("explanation", result.detail.explanation.id.clone())
            } else if let Some(k) = target.strip_prefix("claim:") {
                let id = result
                    .detail
                    .claims
                    .iter()
                    .find(|c| c.stable_key == k)
                    .map(|c| c.id.clone())
                    .ok_or_else(|| format!("evidence target: unknown claim '{k}'"))?;
                ("claim", id)
            } else if let Some(k) = target.strip_prefix("finding:") {
                let id = ioi_ids
                    .get(k)
                    .cloned()
                    .ok_or_else(|| format!("evidence target: unknown finding '{k}'"))?;
                ("finding", id)
            } else {
                return Err(format!("evidence target: bad reference '{target}'"));
            };
            db.evidence_link(&NewEvidence {
                target_type,
                target_id: &target_id,
                source_entity_type: str_field(ev, "source_entity_type"),
                source_entity_id: str_field(ev, "source_entity_id"),
                external_locator: str_field(ev, "external_locator"),
                external_kind: str_field(ev, "external_kind"),
                evidence_type: str_field(ev, "evidence_type").unwrap_or("agent_inference"),
                strength: str_field(ev, "strength").unwrap_or("moderate"),
                excerpt: str_field(ev, "excerpt"),
                author: au,
                author_type: at,
            })
            .map_err(emap)?;
            stats.evidence += 1;
        }
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Keeps the committed fixture valid: if someone edits demo-project.json into
    // something the db layer rejects, this fails.
    #[test]
    fn bundled_fixture_applies_cleanly() {
        let doc: Value =
            serde_json::from_str(include_str!("../../fixtures/demo-project.json")).unwrap();
        let db = Database::in_memory("fixture-test").unwrap();
        let stats = apply(&db, &doc).unwrap();

        assert_eq!(stats.items, 4);
        assert_eq!(stats.explanations, 11, "one representative entry per type");
        assert!(stats.iois >= 3);
        assert!(stats.connections >= 3);
        assert!(stats.evidence >= 3);
        assert_eq!(stats.states, 4);
        assert_eq!(stats.transitions, 4);

        // Sanity: it round-trips through the read path the UI uses, and the state
        // machine's text diagram is generated on the fly.
        assert_eq!(db.item_list(None, None, None).unwrap().len(), 4);
        let explanations = db.explanation_list(None, None).unwrap();
        assert_eq!(explanations.len(), 11);
        let sm = explanations
            .iter()
            .find(|e| e.explanation.explanation_type == "state_machine")
            .unwrap();
        let detail = db.explanation_get(&sm.explanation.id).unwrap();
        assert_eq!(detail.states.len(), 4);
        assert_eq!(detail.transitions.len(), 4);
        let text = detail.diagram_text.unwrap();
        assert!(text.contains("UNAUTHENTICATED"));
        assert!(
            text.contains("--SIGNED_NONCE [signature valid] / derive session key--> AUTHENTICATED")
        );
    }
}
