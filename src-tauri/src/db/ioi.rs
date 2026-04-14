use rusqlite::params;

use super::error::{DbError, Result};
use super::models::{IoiWithTags, ItemOfInterest};
use super::{new_id, now, Database};

#[derive(Debug, Clone)]
pub struct DuplicateWarning {
    pub existing_id: String,
    pub existing_title: String,
}

pub struct NewIoi<'a> {
    pub item_id: &'a str,
    pub title: &'a str,
    pub description: &'a str,
    pub location: Option<&'a str>,
    pub severity: Option<&'a str>,
    pub author: &'a str,
    pub author_type: &'a str,
    pub tags: &'a [String],
}

impl Database {
    pub fn ioi_create(
        &self,
        params: &NewIoi<'_>,
    ) -> Result<(IoiWithTags, Option<DuplicateWarning>)> {
        self.get_item_by_id(params.item_id)
            .map_err(|_| DbError::InvalidReference {
                entity: "item".to_string(),
                id: params.item_id.to_string(),
            })?;
        self.validate_tags(params.tags)?;

        let warning = self.check_ioi_duplicate(params.item_id, params.title, params.location)?;

        let id = new_id();
        let ts = now();
        self.conn.execute(
            "INSERT INTO items_of_interest (id, item_id, title, description, location, severity, author, author_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            rusqlite::params![id, params.item_id, params.title, params.description, params.location, params.severity, params.author, params.author_type, ts],
        )?;
        self.set_ioi_tags(&id, params.tags)?;

        Ok((
            IoiWithTags {
                ioi: ItemOfInterest {
                    id,
                    item_id: params.item_id.to_string(),
                    title: params.title.to_string(),
                    description: params.description.to_string(),
                    location: params.location.map(String::from),
                    severity: params.severity.map(String::from),
                    author: params.author.to_string(),
                    author_type: params.author_type.to_string(),
                    created_at: ts.clone(),
                    updated_at: ts,
                },
                tags: params.tags.to_vec(),
            },
            warning,
        ))
    }

    pub fn ioi_update(
        &self,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        location: Option<Option<&str>>,
        severity: Option<Option<&str>>,
        tags: Option<&[String]>,
    ) -> Result<IoiWithTags> {
        self.get_ioi_by_id(id)?;
        if let Some(tags) = tags {
            self.validate_tags(tags)?;
        }

        let ts = now();
        if let Some(title) = title {
            self.conn.execute(
                "UPDATE items_of_interest SET title = ?1, updated_at = ?2 WHERE id = ?3",
                params![title, ts, id],
            )?;
        }
        if let Some(desc) = description {
            self.conn.execute(
                "UPDATE items_of_interest SET description = ?1, updated_at = ?2 WHERE id = ?3",
                params![desc, ts, id],
            )?;
        }
        if let Some(loc) = location {
            self.conn.execute(
                "UPDATE items_of_interest SET location = ?1, updated_at = ?2 WHERE id = ?3",
                params![loc, ts, id],
            )?;
        }
        if let Some(sev) = severity {
            self.conn.execute(
                "UPDATE items_of_interest SET severity = ?1, updated_at = ?2 WHERE id = ?3",
                params![sev, ts, id],
            )?;
        }
        if let Some(tags) = tags {
            self.set_ioi_tags(id, tags)?;
        }
        self.conn.execute(
            "UPDATE items_of_interest SET updated_at = ?1 WHERE id = ?2",
            params![ts, id],
        )?;

        let ioi = self.get_ioi_by_id(id)?;
        let tags = self.get_ioi_tags(id)?;
        Ok(IoiWithTags { ioi, tags })
    }

    pub fn ioi_delete(&self, id: &str) -> Result<()> {
        let changes = self
            .conn
            .execute("DELETE FROM items_of_interest WHERE id = ?1", params![id])?;
        if changes == 0 {
            return Err(DbError::NotFound {
                entity: "item_of_interest".to_string(),
                id: id.to_string(),
            });
        }
        Ok(())
    }

    fn check_ioi_duplicate(
        &self,
        item_id: &str,
        title: &str,
        location: Option<&str>,
    ) -> Result<Option<DuplicateWarning>> {
        let result: std::result::Result<(String, String), _> = self.conn.query_row(
            "SELECT id, title FROM items_of_interest
             WHERE item_id = ?1 AND (title = ?2 OR (location IS NOT NULL AND location = ?3))",
            params![item_id, title, location],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );
        match result {
            Ok((id, existing_title)) => Ok(Some(DuplicateWarning {
                existing_id: id,
                existing_title,
            })),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::Sqlite(e)),
        }
    }

    pub(crate) fn get_ioi_by_id(&self, id: &str) -> Result<ItemOfInterest> {
        self.conn
            .query_row(
                "SELECT id, item_id, title, description, location, severity, author, author_type, created_at, updated_at
                 FROM items_of_interest WHERE id = ?1",
                params![id],
                |row| {
                    Ok(ItemOfInterest {
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
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                    entity: "item_of_interest".to_string(),
                    id: id.to_string(),
                },
                other => DbError::Sqlite(other),
            })
    }

    pub(crate) fn get_ioi_tags(&self, ioi_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag_name FROM ioi_tags WHERE ioi_id = ?1 ORDER BY tag_name")?;
        let tags = stmt
            .query_map(params![ioi_id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(tags)
    }

    fn set_ioi_tags(&self, ioi_id: &str, tags: &[String]) -> Result<()> {
        self.conn
            .execute("DELETE FROM ioi_tags WHERE ioi_id = ?1", params![ioi_id])?;
        for tag in tags {
            self.conn.execute(
                "INSERT INTO ioi_tags (ioi_id, tag_name) VALUES (?1, ?2)",
                params![ioi_id, tag],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::in_memory("test").unwrap()
    }

    fn create_test_item(db: &Database) -> String {
        db.item_create("httpd", "elf", None, None, "", &[])
            .unwrap()
            .item
            .id
    }

    #[test]
    fn create_ioi() {
        let db = test_db();
        let item_id = create_test_item(&db);
        let (ioi, warning) = db
            .ioi_create(&NewIoi {
                item_id: &item_id,
                title: "parse_header()",
                description: "No bounds check",
                location: Some("0x08041234"),
                severity: Some("critical"),
                author: "claude",
                author_type: "agent",
                tags: &["memory-corruption".to_string()],
            })
            .unwrap();
        assert_eq!(ioi.ioi.title, "parse_header()");
        assert_eq!(ioi.ioi.severity.as_deref(), Some("critical"));
        assert!(warning.is_none());
    }

    #[test]
    fn duplicate_detection_by_title() {
        let db = test_db();
        let item_id = create_test_item(&db);
        db.ioi_create(&NewIoi {
            item_id: &item_id,
            title: "parse_header()",
            description: "first",
            location: None,
            severity: None,
            author: "user",
            author_type: "human",
            tags: &[],
        })
        .unwrap();
        let (_, warning) = db
            .ioi_create(&NewIoi {
                item_id: &item_id,
                title: "parse_header()",
                description: "second",
                location: None,
                severity: None,
                author: "user",
                author_type: "human",
                tags: &[],
            })
            .unwrap();
        assert!(warning.is_some());
        assert_eq!(warning.unwrap().existing_title, "parse_header()");
    }

    #[test]
    fn duplicate_detection_by_location() {
        let db = test_db();
        let item_id = create_test_item(&db);
        db.ioi_create(&NewIoi {
            item_id: &item_id,
            title: "func_a",
            description: "first",
            location: Some("0x1234"),
            severity: None,
            author: "user",
            author_type: "human",
            tags: &[],
        })
        .unwrap();
        let (_, warning) = db
            .ioi_create(&NewIoi {
                item_id: &item_id,
                title: "func_b",
                description: "second",
                location: Some("0x1234"),
                severity: None,
                author: "user",
                author_type: "human",
                tags: &[],
            })
            .unwrap();
        assert!(warning.is_some());
    }

    #[test]
    fn update_ioi() {
        let db = test_db();
        let item_id = create_test_item(&db);
        let (ioi, _) = db
            .ioi_create(&NewIoi {
                item_id: &item_id,
                title: "test",
                description: "desc",
                location: None,
                severity: None,
                author: "user",
                author_type: "human",
                tags: &[],
            })
            .unwrap();
        let updated = db
            .ioi_update(
                &ioi.ioi.id,
                Some("updated title"),
                None,
                None,
                Some(Some("high")),
                None,
            )
            .unwrap();
        assert_eq!(updated.ioi.title, "updated title");
        assert_eq!(updated.ioi.severity.as_deref(), Some("high"));
    }

    #[test]
    fn delete_ioi() {
        let db = test_db();
        let item_id = create_test_item(&db);
        let (ioi, _) = db
            .ioi_create(&NewIoi {
                item_id: &item_id,
                title: "test",
                description: "desc",
                location: None,
                severity: None,
                author: "user",
                author_type: "human",
                tags: &[],
            })
            .unwrap();
        db.ioi_delete(&ioi.ioi.id).unwrap();
        let result = db.ioi_delete(&ioi.ioi.id);
        assert!(matches!(result, Err(DbError::NotFound { .. })));
    }
}
