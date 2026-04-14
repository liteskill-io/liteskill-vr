use std::fmt::Write;

use rusqlite::params;

use super::error::{DbError, Result};
use super::models::SearchResult;
use super::Database;

impl Database {
    pub fn search(&self, query: &str, entity_type: Option<&str>) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        if entity_type.is_none() || entity_type == Some("item") {
            results.extend(self.search_items(query)?);
        }
        if entity_type.is_none() || entity_type == Some("note") {
            results.extend(self.search_notes(query)?);
        }
        if entity_type.is_none() || entity_type == Some("item_of_interest") {
            results.extend(self.search_iois(query)?);
        }
        if entity_type.is_none() || entity_type == Some("connection") {
            results.extend(self.search_connections_fts(query)?);
        }
        Ok(results)
    }

    pub fn filter_ioi(
        &self,
        item_id: Option<&str>,
        severity: Option<&str>,
        tags: Option<&[String]>,
        author_type: Option<&str>,
    ) -> Result<Vec<super::models::IoiWithTags>> {
        let mut sql = String::from(
            "SELECT i.id, i.item_id, i.title, i.description, i.location, i.severity, i.author, i.author_type, i.created_at, i.updated_at
             FROM items_of_interest i WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(item_id) = item_id {
            param_values.push(Box::new(item_id.to_string()));
            let _ = write!(sql, " AND i.item_id = ?{}", param_values.len());
        }
        if let Some(sev) = severity {
            param_values.push(Box::new(sev.to_string()));
            let _ = write!(sql, " AND i.severity = ?{}", param_values.len());
        }
        if let Some(at) = author_type {
            param_values.push(Box::new(at.to_string()));
            let _ = write!(sql, " AND i.author_type = ?{}", param_values.len());
        }
        if let Some(tags) = tags {
            for tag in tags {
                param_values.push(Box::new(tag.clone()));
                let _ = write!(
                    sql,
                    " AND EXISTS(SELECT 1 FROM ioi_tags WHERE ioi_id = i.id AND tag_name = ?{})",
                    param_values.len()
                );
            }
        }
        sql.push_str(" ORDER BY i.created_at");

        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let iois = stmt
            .query_map(params_ref.as_slice(), |row| {
                Ok(super::models::ItemOfInterest {
                    id: row.get(0)?,
                    item_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    location: row.get(4)?,
                    severity: row.get(5)?,
                    author: row.get(6)?,
                    author_type: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut result = Vec::with_capacity(iois.len());
        for ioi in iois {
            let tags = self.get_ioi_tags(&ioi.id)?;
            result.push(super::models::IoiWithTags { ioi, tags });
        }
        Ok(result)
    }

    pub fn bulk_delete(
        &self,
        author: Option<&str>,
        since: Option<&str>,
        entity_type: Option<&str>,
    ) -> Result<u64> {
        if author.is_none() && since.is_none() && entity_type.is_none() {
            return Err(DbError::BulkDeleteNoFilter);
        }

        let mut total = 0u64;
        let tables: &[(&str, &str)] = match entity_type {
            Some("note") => &[("notes", "created_at")],
            Some("item_of_interest") => &[("items_of_interest", "created_at")],
            Some("connection") => &[("connections", "created_at")],
            Some("item") => &[("items", "created_at")],
            None => &[
                ("connections", "created_at"),
                ("items_of_interest", "created_at"),
                ("notes", "created_at"),
            ],
            Some(_) => return Ok(0),
        };

        for (table, _ts_col) in tables {
            let mut sql = format!("DELETE FROM {table} WHERE 1=1");
            let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

            if let Some(author) = author {
                param_values.push(Box::new(author.to_string()));
                let _ = write!(sql, " AND author = ?{}", param_values.len());
            }
            if let Some(since) = since {
                param_values.push(Box::new(since.to_string()));
                let _ = write!(sql, " AND created_at >= ?{}", param_values.len());
            }

            let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values
                .iter()
                .map(std::convert::AsRef::as_ref)
                .collect();
            let changes = self.conn.execute(&sql, params_ref.as_slice())?;
            total += changes as u64;
        }
        Ok(total)
    }

    fn search_items(&self, query: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, snippet(fts_items, 2, '<b>', '</b>', '...', 32)
             FROM fts_items f WHERE fts_items MATCH ?1",
        )?;
        let results = stmt
            .query_map(params![query], |row| {
                let id: String = row.get(0)?;
                let snippet: String = row.get(1)?;
                Ok(SearchResult {
                    entity_type: "item".to_string(),
                    entity_id: id,
                    parent_item_id: None,
                    parent_item_name: None,
                    title: String::new(),
                    snippet,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut enriched = Vec::with_capacity(results.len());
        for mut r in results {
            if let Ok(item) = self.get_item_by_id(&r.entity_id) {
                r.title = item.name;
            }
            enriched.push(r);
        }
        Ok(enriched)
    }

    fn search_notes(&self, query: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.item_id, snippet(fts_notes, 3, '<b>', '</b>', '...', 32)
             FROM fts_notes f WHERE fts_notes MATCH ?1",
        )?;
        let results = stmt
            .query_map(params![query], |row| {
                let id: String = row.get(0)?;
                let item_id: String = row.get(1)?;
                let snippet: String = row.get(2)?;
                Ok(SearchResult {
                    entity_type: "note".to_string(),
                    entity_id: id,
                    parent_item_id: Some(item_id),
                    parent_item_name: None,
                    title: String::new(),
                    snippet,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut enriched = Vec::with_capacity(results.len());
        for mut r in results {
            if let Ok(note) = self.get_note_by_id(&r.entity_id) {
                r.title = note.title;
            }
            if let Some(ref item_id) = r.parent_item_id {
                if let Ok(item) = self.get_item_by_id(item_id) {
                    r.parent_item_name = Some(item.name);
                }
            }
            enriched.push(r);
        }
        Ok(enriched)
    }

    fn search_iois(&self, query: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.item_id, snippet(fts_ioi, 3, '<b>', '</b>', '...', 32)
             FROM fts_ioi f WHERE fts_ioi MATCH ?1",
        )?;
        let results = stmt
            .query_map(params![query], |row| {
                let id: String = row.get(0)?;
                let item_id: String = row.get(1)?;
                let snippet: String = row.get(2)?;
                Ok(SearchResult {
                    entity_type: "item_of_interest".to_string(),
                    entity_id: id,
                    parent_item_id: Some(item_id),
                    parent_item_name: None,
                    title: String::new(),
                    snippet,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut enriched = Vec::with_capacity(results.len());
        for mut r in results {
            if let Ok(ioi) = self.get_ioi_by_id(&r.entity_id) {
                r.title = ioi.title;
            }
            if let Some(ref item_id) = r.parent_item_id {
                if let Ok(item) = self.get_item_by_id(item_id) {
                    r.parent_item_name = Some(item.name);
                }
            }
            enriched.push(r);
        }
        Ok(enriched)
    }

    fn search_connections_fts(&self, query: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, snippet(fts_connections, 1, '<b>', '</b>', '...', 32)
             FROM fts_connections f WHERE fts_connections MATCH ?1",
        )?;
        let results = stmt
            .query_map(params![query], |row| {
                Ok(SearchResult {
                    entity_type: "connection".to_string(),
                    entity_id: row.get(0)?,
                    parent_item_id: None,
                    parent_item_name: None,
                    title: String::new(),
                    snippet: row.get(1)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{NewConnection, NewIoi};

    fn test_db() -> Database {
        Database::in_memory("test").unwrap()
    }

    fn setup_data(db: &Database) -> (String, String) {
        let item = db
            .item_create("httpd", "elf", None, None, "Main web server binary", &[])
            .unwrap();
        let item_id = item.item.id;
        db.note_create(
            &item_id,
            "Analysis notes",
            "Found buffer overflow in parse_header",
            "claude",
            "agent",
            &[],
        )
        .unwrap();
        db.ioi_create(&NewIoi {
            item_id: &item_id,
            title: "parse_header()",
            description: "Stack buffer overflow",
            location: Some("0x08041234"),
            severity: Some("critical"),
            author: "claude",
            author_type: "agent",
            tags: &["memory-corruption".to_string()],
        })
        .unwrap();

        let item2 = db
            .item_create(
                "libfoo.so",
                "shared_object",
                None,
                None,
                "Shared library",
                &[],
            )
            .unwrap();
        db.connection_create(&NewConnection {
            source_id: &item_id,
            source_type: "item",
            target_id: &item2.item.id,
            target_type: "item",
            connection_type: "links",
            description: "httpd dynamically links libfoo",
            author: "user",
            author_type: "human",
        })
        .unwrap();
        (item_id, item2.item.id)
    }

    #[test]
    fn search_across_all_entities() {
        let db = test_db();
        setup_data(&db);
        let results = db.search("buffer overflow", None).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn search_filtered_by_entity_type() {
        let db = test_db();
        setup_data(&db);
        let results = db.search("httpd", Some("item")).unwrap();
        assert!(results.iter().all(|r| r.entity_type == "item"));
    }

    #[test]
    fn filter_ioi_by_severity() {
        let db = test_db();
        let (item_id, _) = setup_data(&db);
        let results = db.filter_ioi(None, Some("critical"), None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].ioi.title, "parse_header()");

        let results = db
            .filter_ioi(Some(&item_id), Some("low"), None, None)
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn filter_ioi_by_tag() {
        let db = test_db();
        setup_data(&db);
        let results = db
            .filter_ioi(None, None, Some(&["memory-corruption".to_string()]), None)
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn bulk_delete_by_author() {
        let db = test_db();
        setup_data(&db);
        let deleted = db.bulk_delete(Some("claude"), None, None).unwrap();
        assert!(deleted >= 2);
    }

    #[test]
    fn bulk_delete_requires_filter() {
        let db = test_db();
        let result = db.bulk_delete(None, None, None);
        assert!(matches!(result, Err(DbError::BulkDeleteNoFilter)));
    }

    #[test]
    fn bulk_delete_by_entity_type() {
        let db = test_db();
        setup_data(&db);
        let deleted = db.bulk_delete(None, None, Some("note")).unwrap();
        assert_eq!(deleted, 1);
    }
}
