use std::fmt::Write;

use super::error::{DbError, Result};
use super::models::SearchResult;
use super::{parse_tag_list, Database, TAG_SEP};

type Params = Vec<Box<dyn rusqlite::types::ToSql>>;

/// Optional filters that narrow a full-text [`Database::search`].
///
/// A filter that does not apply to an entity kind (e.g. `severity` on items)
/// excludes that kind from the results entirely, so a severity filter makes
/// `search` return only items of interest.
#[derive(Default)]
pub struct SearchFilters<'a> {
    pub tags: Option<&'a [String]>,
    pub severity: Option<&'a str>,
    pub connection_type: Option<&'a str>,
    pub author_type: Option<&'a str>,
}

// Append ` AND <col> = ?n` for an optional equality filter.
fn push_eq(sql: &mut String, pv: &mut Params, col: &str, val: Option<&str>) {
    if let Some(v) = val {
        pv.push(Box::new(v.to_string()));
        let _ = write!(sql, " AND {col} = ?{}", pv.len());
    }
}

// Append an `EXISTS` clause per tag against a `<join>(<fk>, tag_name)` table.
fn push_tags(
    sql: &mut String,
    pv: &mut Params,
    join: &str,
    fk: &str,
    id_expr: &str,
    tags: Option<&[String]>,
) {
    if let Some(tags) = tags {
        for tag in tags {
            pv.push(Box::new(tag.clone()));
            let _ = write!(
                sql,
                " AND EXISTS(SELECT 1 FROM {join} WHERE {fk} = {id_expr} AND tag_name = ?{})",
                pv.len()
            );
        }
    }
}

fn params_ref(pv: &Params) -> Vec<&dyn rusqlite::types::ToSql> {
    pv.iter().map(std::convert::AsRef::as_ref).collect()
}

impl Database {
    pub fn search(
        &self,
        query: &str,
        entity_type: Option<&str>,
        filters: &SearchFilters<'_>,
    ) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        let want = |et: &str| entity_type.is_none() || entity_type == Some(et);

        // Each entity kind only participates when no filter that is inapplicable
        // to it has been set.
        if want("item")
            && filters.severity.is_none()
            && filters.connection_type.is_none()
            && filters.author_type.is_none()
        {
            results.extend(self.search_items(query, filters.tags)?);
        }
        if want("note") && filters.severity.is_none() && filters.connection_type.is_none() {
            results.extend(self.search_notes(query, filters.tags, filters.author_type)?);
        }
        if want("item_of_interest") && filters.connection_type.is_none() {
            results.extend(self.search_iois(
                query,
                filters.tags,
                filters.severity,
                filters.author_type,
            )?);
        }
        if want("connection") && filters.tags.is_none() && filters.severity.is_none() {
            results.extend(self.search_connections_fts(
                query,
                filters.connection_type,
                filters.author_type,
            )?);
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
        let mut sql = format!(
            "SELECT i.id, i.item_id, i.title, i.description, i.location, i.severity,
                    i.status, i.author, i.author_type, i.created_at, i.updated_at,
                    (SELECT GROUP_CONCAT(tag_name, char({sep})) FROM
                        (SELECT tag_name FROM ioi_tags WHERE ioi_id = i.id ORDER BY tag_name)
                    ) AS tag_list
             FROM items_of_interest i WHERE 1=1",
            sep = TAG_SEP as u32,
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
                Ok(super::models::IoiWithTags {
                    ioi: super::models::ItemOfInterest {
                        id: row.get(0)?,
                        item_id: row.get(1)?,
                        title: row.get(2)?,
                        description: row.get(3)?,
                        location: row.get(4)?,
                        severity: row.get(5)?,
                        status: row.get(6)?,
                        author: row.get(7)?,
                        author_type: row.get(8)?,
                        created_at: row.get(9)?,
                        updated_at: row.get(10)?,
                    },
                    tags: parse_tag_list(row.get(11)?),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(iois)
    }

    pub fn filter_notes(
        &self,
        item_id: Option<&str>,
        tags: Option<&[String]>,
        author_type: Option<&str>,
    ) -> Result<Vec<super::models::NoteWithTags>> {
        let mut sql = format!(
            "SELECT n.id, n.item_id, n.title, n.content, n.author, n.author_type,
                    n.created_at, n.updated_at,
                    (SELECT GROUP_CONCAT(tag_name, char({sep})) FROM
                        (SELECT tag_name FROM note_tags WHERE note_id = n.id ORDER BY tag_name)
                    ) AS tag_list
             FROM notes n WHERE 1=1",
            sep = TAG_SEP as u32,
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(item_id) = item_id {
            param_values.push(Box::new(item_id.to_string()));
            let _ = write!(sql, " AND n.item_id = ?{}", param_values.len());
        }
        if let Some(at) = author_type {
            param_values.push(Box::new(at.to_string()));
            let _ = write!(sql, " AND n.author_type = ?{}", param_values.len());
        }
        if let Some(tags) = tags {
            for tag in tags {
                param_values.push(Box::new(tag.clone()));
                let _ = write!(
                    sql,
                    " AND EXISTS(SELECT 1 FROM note_tags WHERE note_id = n.id AND tag_name = ?{})",
                    param_values.len()
                );
            }
        }
        sql.push_str(" ORDER BY n.created_at");

        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let notes = stmt
            .query_map(params_ref.as_slice(), |row| {
                Ok(super::models::NoteWithTags {
                    note: super::models::Note {
                        id: row.get(0)?,
                        item_id: row.get(1)?,
                        title: row.get(2)?,
                        content: row.get(3)?,
                        author: row.get(4)?,
                        author_type: row.get(5)?,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                    },
                    tags: parse_tag_list(row.get(8)?),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(notes)
    }

    pub fn filter_connections(
        &self,
        connection_type: Option<&str>,
        author_type: Option<&str>,
    ) -> Result<Vec<super::models::Connection>> {
        let mut sql = String::from(
            "SELECT id, source_id, source_type, target_id, target_type, connection_type, description, author, author_type, created_at
             FROM connections WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ct) = connection_type {
            param_values.push(Box::new(ct.to_string()));
            let _ = write!(sql, " AND connection_type = ?{}", param_values.len());
        }
        if let Some(at) = author_type {
            param_values.push(Box::new(at.to_string()));
            let _ = write!(sql, " AND author_type = ?{}", param_values.len());
        }
        sql.push_str(" ORDER BY created_at");

        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let conns = stmt
            .query_map(params_ref.as_slice(), |row| {
                Ok(super::models::Connection {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    source_type: row.get(2)?,
                    target_id: row.get(3)?,
                    target_type: row.get(4)?,
                    connection_type: row.get(5)?,
                    description: row.get(6)?,
                    author: row.get(7)?,
                    author_type: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(conns)
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
        let tables: &[&str] = match entity_type {
            Some("note") => &["notes"],
            Some("item_of_interest") => &["items_of_interest"],
            Some("connection") => &["connections"],
            Some("item") => &["items"],
            None => &["connections", "items_of_interest", "notes"],
            Some(_) => return Ok(0),
        };

        for table in tables {
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

    fn search_items(&self, query: &str, tags: Option<&[String]>) -> Result<Vec<SearchResult>> {
        let mut sql = String::from(
            "SELECT f.id, i.name, snippet(fts_items, 2, '<b>', '</b>', '...', 32)
             FROM fts_items f JOIN items i ON i.id = f.id
             WHERE fts_items MATCH ?1",
        );
        let mut pv: Params = vec![Box::new(query.to_string())];
        push_tags(&mut sql, &mut pv, "item_tags", "item_id", "i.id", tags);

        let mut stmt = self.conn.prepare(&sql)?;
        let results = stmt
            .query_map(params_ref(&pv).as_slice(), |row| {
                Ok(SearchResult {
                    entity_type: "item".to_string(),
                    entity_id: row.get(0)?,
                    parent_item_id: None,
                    parent_item_name: None,
                    title: row.get(1)?,
                    snippet: row.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(results)
    }

    fn search_notes(
        &self,
        query: &str,
        tags: Option<&[String]>,
        author_type: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let mut sql = String::from(
            "SELECT f.id, f.item_id, n.title, i.name, snippet(fts_notes, 3, '<b>', '</b>', '...', 32)
             FROM fts_notes f
             JOIN notes n ON n.id = f.id
             LEFT JOIN items i ON i.id = f.item_id
             WHERE fts_notes MATCH ?1",
        );
        let mut pv: Params = vec![Box::new(query.to_string())];
        push_eq(&mut sql, &mut pv, "n.author_type", author_type);
        push_tags(&mut sql, &mut pv, "note_tags", "note_id", "n.id", tags);

        let mut stmt = self.conn.prepare(&sql)?;
        let results = stmt
            .query_map(params_ref(&pv).as_slice(), |row| {
                Ok(SearchResult {
                    entity_type: "note".to_string(),
                    entity_id: row.get(0)?,
                    parent_item_id: row.get(1)?,
                    parent_item_name: row.get(3)?,
                    title: row.get(2)?,
                    snippet: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(results)
    }

    fn search_iois(
        &self,
        query: &str,
        tags: Option<&[String]>,
        severity: Option<&str>,
        author_type: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let mut sql = String::from(
            "SELECT f.id, f.item_id, o.title, i.name, snippet(fts_ioi, 3, '<b>', '</b>', '...', 32)
             FROM fts_ioi f
             JOIN items_of_interest o ON o.id = f.id
             JOIN items i ON i.id = f.item_id
             WHERE fts_ioi MATCH ?1",
        );
        let mut pv: Params = vec![Box::new(query.to_string())];
        push_eq(&mut sql, &mut pv, "o.severity", severity);
        push_eq(&mut sql, &mut pv, "o.author_type", author_type);
        push_tags(&mut sql, &mut pv, "ioi_tags", "ioi_id", "o.id", tags);

        let mut stmt = self.conn.prepare(&sql)?;
        let results = stmt
            .query_map(params_ref(&pv).as_slice(), |row| {
                Ok(SearchResult {
                    entity_type: "item_of_interest".to_string(),
                    entity_id: row.get(0)?,
                    parent_item_id: row.get(1)?,
                    parent_item_name: row.get(3)?,
                    title: row.get(2)?,
                    snippet: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(results)
    }

    fn search_connections_fts(
        &self,
        query: &str,
        connection_type: Option<&str>,
        author_type: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let mut sql = String::from(
            "SELECT f.id, snippet(fts_connections, 1, '<b>', '</b>', '...', 32)
             FROM fts_connections f
             JOIN connections c ON c.id = f.id
             WHERE fts_connections MATCH ?1",
        );
        let mut pv: Params = vec![Box::new(query.to_string())];
        push_eq(&mut sql, &mut pv, "c.connection_type", connection_type);
        push_eq(&mut sql, &mut pv, "c.author_type", author_type);

        let mut stmt = self.conn.prepare(&sql)?;
        let results = stmt
            .query_map(params_ref(&pv).as_slice(), |row| {
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
            Some(&item_id),
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
            status: None,
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
        let results = db
            .search("buffer overflow", None, &SearchFilters::default())
            .unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn search_filtered_by_entity_type() {
        let db = test_db();
        setup_data(&db);
        let results = db
            .search("httpd", Some("item"), &SearchFilters::default())
            .unwrap();
        assert!(results.iter().all(|r| r.entity_type == "item"));
    }

    #[test]
    fn search_severity_filter_restricts_to_iois() {
        let db = test_db();
        setup_data(&db);
        // Baseline: unfiltered, "buffer" matches both the note ("buffer overflow")
        // and the IOI ("Stack buffer overflow"). Establish that the note IS in the
        // unfiltered results, so the filter assertion below proves real dropping.
        let baseline = db
            .search("buffer", None, &SearchFilters::default())
            .unwrap();
        assert!(baseline.iter().any(|r| r.entity_type == "note"));
        assert!(baseline.iter().any(|r| r.entity_type == "item_of_interest"));

        // A severity filter must drop the note (notes have no severity), leaving
        // only the IOI.
        let filters = SearchFilters {
            severity: Some("critical"),
            ..Default::default()
        };
        let results = db.search("buffer", None, &filters).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.entity_type == "item_of_interest"));

        // The IOI is "critical", so a "low" filter drops it too → nothing left.
        let none = SearchFilters {
            severity: Some("low"),
            ..Default::default()
        };
        assert!(db.search("buffer", None, &none).unwrap().is_empty());
    }

    #[test]
    fn search_tag_filter() {
        let db = test_db();
        setup_data(&db);
        // The note and the IOI both match "buffer", but only the IOI carries the
        // memory-corruption tag.
        let filters = SearchFilters {
            tags: Some(&["memory-corruption".to_string()]),
            ..Default::default()
        };
        let results = db.search("buffer", None, &filters).unwrap();
        assert!(results.iter().all(|r| r.entity_type == "item_of_interest"));
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_author_type_filter() {
        let db = test_db();
        setup_data(&db);
        // The only "links"-matching entity is the connection, authored by a human.
        // Filtering by the matching author_type returns it...
        let human = SearchFilters {
            author_type: Some("human"),
            ..Default::default()
        };
        let results = db.search("links", None, &human).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.entity_type == "connection"));

        // ...and filtering by the other author_type drops it. This proves the
        // filter is actually consulted (without it, "agent" would also match).
        let agent = SearchFilters {
            author_type: Some("agent"),
            ..Default::default()
        };
        assert!(db.search("links", None, &agent).unwrap().is_empty());
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
