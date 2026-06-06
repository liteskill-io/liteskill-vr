use rusqlite::params;

use super::error::{DbError, Result};
use super::models::EvidenceLink;
use super::{new_id, now, Database};

/// Input for [`Database::evidence_link`].
///
/// Either `source_entity_*` (a link to an existing entity) or `external_locator`
/// (a free-text reference like a Ghidra symbol, address, or pcap packet) must be
/// present.
pub struct NewEvidence<'a> {
    pub target_type: &'a str,
    pub target_id: &'a str,
    pub source_entity_type: Option<&'a str>,
    pub source_entity_id: Option<&'a str>,
    pub external_locator: Option<&'a str>,
    pub external_kind: Option<&'a str>,
    pub evidence_type: &'a str,
    pub strength: &'a str,
    pub excerpt: Option<&'a str>,
    pub author: &'a str,
}

impl Database {
    pub fn evidence_link(&self, e: &NewEvidence<'_>) -> Result<EvidenceLink> {
        self.validate_evidence_target(e.target_type, e.target_id)?;

        if e.source_entity_id.is_none() && e.external_locator.is_none() {
            return Err(DbError::InvalidReference {
                entity: "evidence source".to_string(),
                id: "must provide a source entity or an external locator".to_string(),
            });
        }
        if let (Some(st), Some(sid)) = (e.source_entity_type, e.source_entity_id) {
            self.validate_source_entity(st, sid)?;
        }

        let id = new_id();
        let ts = now();
        self.conn.execute(
            "INSERT INTO evidence_links (id, target_type, target_id, source_entity_type,
                source_entity_id, external_locator, external_kind, evidence_type, strength,
                excerpt, author, author_type, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'agent', ?12)",
            params![
                id,
                e.target_type,
                e.target_id,
                e.source_entity_type,
                e.source_entity_id,
                e.external_locator,
                e.external_kind,
                e.evidence_type,
                e.strength,
                e.excerpt,
                e.author,
                ts
            ],
        )?;

        Ok(EvidenceLink {
            id,
            target_type: e.target_type.to_string(),
            target_id: e.target_id.to_string(),
            source_entity_type: e.source_entity_type.map(String::from),
            source_entity_id: e.source_entity_id.map(String::from),
            external_locator: e.external_locator.map(String::from),
            external_kind: e.external_kind.map(String::from),
            evidence_type: e.evidence_type.to_string(),
            strength: e.strength.to_string(),
            excerpt: e.excerpt.map(String::from),
            author: e.author.to_string(),
            author_type: "agent".to_string(),
            created_at: ts,
        })
    }

    fn validate_evidence_target(&self, target_type: &str, target_id: &str) -> Result<()> {
        let table = match target_type {
            "explanation" => "explanations",
            "claim" => "claims",
            "finding" => "items_of_interest",
            _ => {
                return Err(DbError::InvalidReference {
                    entity: format!("evidence target type '{target_type}'"),
                    id: target_id.to_string(),
                })
            }
        };
        self.row_exists(table, target_id, target_type)
    }

    fn validate_source_entity(&self, entity_type: &str, id: &str) -> Result<()> {
        let table = match entity_type {
            "item" => "items",
            "item_of_interest" | "finding" => "items_of_interest",
            "note" => "notes",
            "connection" => "connections",
            "explanation" => "explanations",
            _ => {
                return Err(DbError::InvalidReference {
                    entity: format!("evidence source type '{entity_type}'"),
                    id: id.to_string(),
                })
            }
        };
        self.row_exists(table, id, entity_type)
    }

    fn row_exists(&self, table: &str, id: &str, entity_label: &str) -> Result<()> {
        let exists: bool = self.conn.query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?1)"),
            params![id],
            |row| row.get(0),
        )?;
        if exists {
            Ok(())
        } else {
            Err(DbError::InvalidReference {
                entity: entity_label.to_string(),
                id: id.to_string(),
            })
        }
    }
}
