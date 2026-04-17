use std::fmt::Write;

use rusqlite::params;

use super::error::{DbError, Result};
use super::models::{
    Connection, IoiWithTags, Item, ItemDetail, ItemSummary, ItemWithTags, NoteWithTags,
};
use super::{new_id, now, parse_tag_list, Database, TAG_SEP};

impl Database {
    pub fn item_list(
        &self,
        item_type: Option<&str>,
        analysis_status: Option<&str>,
        tags: Option<&[String]>,
    ) -> Result<Vec<ItemSummary>> {
        let mut sql = format!(
            "SELECT i.id, i.name, i.item_type, i.path, i.architecture, i.description,
                    i.analysis_status, i.created_at, i.updated_at,
                    (SELECT COUNT(*) FROM notes WHERE item_id = i.id) as note_count,
                    (SELECT COUNT(*) FROM items_of_interest WHERE item_id = i.id) as ioi_count,
                    (SELECT COUNT(*) FROM connections WHERE
                        (source_id = i.id AND source_type = 'item') OR
                        (target_id = i.id AND target_type = 'item')
                    ) as conn_count,
                    (SELECT GROUP_CONCAT(tag_name, char({sep})) FROM
                        (SELECT tag_name FROM item_tags WHERE item_id = i.id ORDER BY tag_name)
                    ) as tag_list
             FROM items i WHERE 1=1",
            sep = TAG_SEP as u32,
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(it) = item_type {
            param_values.push(Box::new(it.to_string()));
            let _ = write!(sql, " AND i.item_type = ?{}", param_values.len());
        }
        if let Some(status) = analysis_status {
            param_values.push(Box::new(status.to_string()));
            let _ = write!(sql, " AND i.analysis_status = ?{}", param_values.len());
        }
        if let Some(tags) = tags {
            for tag in tags {
                param_values.push(Box::new(tag.clone()));
                let _ = write!(
                    sql,
                    " AND EXISTS(SELECT 1 FROM item_tags WHERE item_id = i.id AND tag_name = ?{})",
                    param_values.len()
                );
            }
        }
        sql.push_str(" ORDER BY i.name");

        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let items = stmt
            .query_map(params_ref.as_slice(), |row| {
                Ok(ItemSummary {
                    item: ItemWithTags {
                        item: Item {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            item_type: row.get(2)?,
                            path: row.get(3)?,
                            architecture: row.get(4)?,
                            description: row.get(5)?,
                            analysis_status: row.get(6)?,
                            created_at: row.get(7)?,
                            updated_at: row.get(8)?,
                        },
                        tags: parse_tag_list(row.get(12)?),
                    },
                    note_count: row.get(9)?,
                    ioi_count: row.get(10)?,
                    connection_count: row.get(11)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items)
    }

    pub fn item_get(&self, id: &str) -> Result<ItemDetail> {
        let item = self.get_item_by_id(id)?;
        let tags = self.get_item_tags(id)?;
        let notes = self.get_notes_for_item(id)?;
        let iois = self.get_iois_for_item(id)?;
        let connections = self.get_connections_for_entity(id)?;
        Ok(ItemDetail {
            item: ItemWithTags { item, tags },
            notes,
            items_of_interest: iois,
            connections,
        })
    }

    pub fn item_create(
        &self,
        name: &str,
        item_type: &str,
        path: Option<&str>,
        architecture: Option<&str>,
        description: &str,
        tags: &[String],
    ) -> Result<ItemWithTags> {
        self.validate_tags(tags)?;
        let id = new_id();
        let ts = now();
        self.conn.execute(
            "INSERT INTO items (id, name, item_type, path, architecture, description, analysis_status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'untouched', ?7, ?7)",
            params![id, name, item_type, path, architecture, description, ts],
        )?;
        self.set_item_tags(&id, tags)?;
        Ok(ItemWithTags {
            item: Item {
                id,
                name: name.to_string(),
                item_type: item_type.to_string(),
                path: path.map(String::from),
                architecture: architecture.map(String::from),
                description: description.to_string(),
                analysis_status: "untouched".to_string(),
                created_at: ts.clone(),
                updated_at: ts,
            },
            tags: tags.to_vec(),
        })
    }

    pub fn item_update(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        analysis_status: Option<&str>,
        tags: Option<&[String]>,
    ) -> Result<ItemWithTags> {
        self.get_item_by_id(id)?;
        if let Some(tags) = tags {
            self.validate_tags(tags)?;
        }

        let ts = now();
        self.conn.execute(
            "UPDATE items SET
                name = COALESCE(?1, name),
                description = COALESCE(?2, description),
                analysis_status = COALESCE(?3, analysis_status),
                updated_at = ?4
             WHERE id = ?5",
            params![name, description, analysis_status, ts, id],
        )?;
        if let Some(tags) = tags {
            self.set_item_tags(id, tags)?;
        }

        let item = self.get_item_by_id(id)?;
        let tags = self.get_item_tags(id)?;
        Ok(ItemWithTags { item, tags })
    }

    pub fn item_delete(&self, id: &str) -> Result<()> {
        let changes = self
            .conn
            .execute("DELETE FROM items WHERE id = ?1", params![id])?;
        if changes == 0 {
            return Err(DbError::NotFound {
                entity: "item".to_string(),
                id: id.to_string(),
            });
        }
        Ok(())
    }

    pub(crate) fn get_item_by_id(&self, id: &str) -> Result<Item> {
        self.conn
            .query_row(
                "SELECT id, name, item_type, path, architecture, description, analysis_status, created_at, updated_at
                 FROM items WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Item {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        item_type: row.get(2)?,
                        path: row.get(3)?,
                        architecture: row.get(4)?,
                        description: row.get(5)?,
                        analysis_status: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                    entity: "item".to_string(),
                    id: id.to_string(),
                },
                other => DbError::Sqlite(other),
            })
    }

    fn get_item_tags(&self, item_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag_name FROM item_tags WHERE item_id = ?1 ORDER BY tag_name")?;
        let tags = stmt
            .query_map(params![item_id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(tags)
    }

    fn set_item_tags(&self, item_id: &str, tags: &[String]) -> Result<()> {
        self.conn
            .execute("DELETE FROM item_tags WHERE item_id = ?1", params![item_id])?;
        for tag in tags {
            self.conn.execute(
                "INSERT INTO item_tags (item_id, tag_name) VALUES (?1, ?2)",
                params![item_id, tag],
            )?;
        }
        Ok(())
    }

    fn get_notes_for_item(&self, item_id: &str) -> Result<Vec<NoteWithTags>> {
        let sql = format!(
            "SELECT n.id, n.item_id, n.title, n.content, n.author, n.author_type,
                    n.created_at, n.updated_at,
                    (SELECT GROUP_CONCAT(tag_name, char({sep})) FROM
                        (SELECT tag_name FROM note_tags WHERE note_id = n.id ORDER BY tag_name)
                    ) AS tag_list
             FROM notes n WHERE n.item_id = ?1 ORDER BY n.created_at",
            sep = TAG_SEP as u32,
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let notes = stmt
            .query_map(params![item_id], |row| {
                Ok(NoteWithTags {
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

    fn get_iois_for_item(&self, item_id: &str) -> Result<Vec<IoiWithTags>> {
        let sql = format!(
            "SELECT o.id, o.item_id, o.title, o.description, o.location, o.severity,
                    o.status, o.author, o.author_type, o.created_at, o.updated_at,
                    (SELECT GROUP_CONCAT(tag_name, char({sep})) FROM
                        (SELECT tag_name FROM ioi_tags WHERE ioi_id = o.id ORDER BY tag_name)
                    ) AS tag_list
             FROM items_of_interest o WHERE o.item_id = ?1 ORDER BY o.created_at",
            sep = TAG_SEP as u32,
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let iois = stmt
            .query_map(params![item_id], |row| {
                Ok(IoiWithTags {
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

    fn get_connections_for_entity(&self, entity_id: &str) -> Result<Vec<Connection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, source_type, target_id, target_type, connection_type, description, author, author_type, created_at
             FROM connections WHERE source_id = ?1 OR target_id = ?1 ORDER BY created_at",
        )?;
        let conns = stmt
            .query_map(params![entity_id], |row| {
                Ok(Connection {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::in_memory("test").unwrap()
    }

    #[test]
    fn create_and_get_item() {
        let db = test_db();
        let item = db
            .item_create(
                "httpd",
                "elf",
                Some("/usr/bin/httpd"),
                Some("arm32"),
                "Web server",
                &["interesting".to_string()],
            )
            .unwrap();
        assert_eq!(item.item.name, "httpd");
        assert_eq!(item.tags, vec!["interesting"]);

        let detail = db.item_get(&item.item.id).unwrap();
        assert_eq!(detail.item.item.name, "httpd");
        assert_eq!(detail.item.tags, vec!["interesting"]);
        assert!(detail.notes.is_empty());
        assert!(detail.items_of_interest.is_empty());
        assert!(detail.connections.is_empty());
    }

    #[test]
    fn list_items_with_filters() {
        let db = test_db();
        db.item_create("httpd", "elf", None, None, "", &[]).unwrap();
        db.item_create("libfoo.so", "shared_object", None, None, "", &[])
            .unwrap();
        db.item_create(
            "httpd.conf",
            "config",
            None,
            None,
            "",
            &["insecure-config".to_string()],
        )
        .unwrap();

        let all = db.item_list(None, None, None).unwrap();
        assert_eq!(all.len(), 3);

        let elfs = db.item_list(Some("elf"), None, None).unwrap();
        assert_eq!(elfs.len(), 1);

        let tagged = db
            .item_list(None, None, Some(&["insecure-config".to_string()]))
            .unwrap();
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].item.item.name, "httpd.conf");
    }

    #[test]
    fn update_item() {
        let db = test_db();
        let item = db.item_create("httpd", "elf", None, None, "", &[]).unwrap();
        let updated = db
            .item_update(
                &item.item.id,
                None,
                Some("Updated desc"),
                Some("in_progress"),
                None,
            )
            .unwrap();
        assert_eq!(updated.item.description, "Updated desc");
        assert_eq!(updated.item.analysis_status, "in_progress");
    }

    #[test]
    fn delete_item_cascades() {
        let db = test_db();
        let item = db.item_create("httpd", "elf", None, None, "", &[]).unwrap();
        db.note_create(
            Some(&item.item.id),
            "test note",
            "content",
            "human",
            "human",
            &[],
        )
        .unwrap();
        db.item_delete(&item.item.id).unwrap();
        let result = db.item_get(&item.item.id);
        assert!(matches!(result, Err(DbError::NotFound { .. })));
    }

    #[test]
    fn create_with_invalid_tag_fails() {
        let db = test_db();
        let result = db.item_create("httpd", "elf", None, None, "", &["nonexistent".to_string()]);
        assert!(matches!(result, Err(DbError::UnregisteredTag(_))));
    }
}
