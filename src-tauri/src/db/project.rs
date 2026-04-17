use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::error::{DbError, Result};
use super::models::Project;
use super::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedItem {
    pub id: String,
    pub name: String,
    pub item_type: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedNote {
    pub id: String,
    pub item_id: Option<String>,
    pub title: String,
    pub author: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedIoi {
    pub id: String,
    pub item_id: String,
    pub title: String,
    pub severity: Option<String>,
    pub author: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangesSince {
    pub items: Vec<ChangedItem>,
    pub notes: Vec<ChangedNote>,
    pub items_of_interest: Vec<ChangedIoi>,
}

impl Database {
    pub fn project_get(&self) -> Result<Project> {
        self.conn
            .query_row(
                "SELECT id, name, description, created_at, updated_at FROM project LIMIT 1",
                [],
                |row| {
                    Ok(Project {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                    entity: "project".to_string(),
                    id: String::new(),
                },
                other => DbError::Sqlite(other),
            })
    }

    pub fn changes_since(&self, since: &str) -> Result<ChangesSince> {
        let mut items_stmt = self.conn.prepare(
            "SELECT id, name, item_type, created_at, updated_at FROM items
             WHERE created_at >= ?1 OR updated_at >= ?1 ORDER BY updated_at",
        )?;
        let items = items_stmt
            .query_map(params![since], |row| {
                Ok(ChangedItem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    item_type: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut notes_stmt = self.conn.prepare(
            "SELECT id, item_id, title, author, created_at FROM notes
             WHERE created_at >= ?1 OR updated_at >= ?1 ORDER BY updated_at",
        )?;
        let notes = notes_stmt
            .query_map(params![since], |row| {
                Ok(ChangedNote {
                    id: row.get(0)?,
                    item_id: row.get(1)?,
                    title: row.get(2)?,
                    author: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut iois_stmt = self.conn.prepare(
            "SELECT id, item_id, title, severity, author, created_at FROM items_of_interest
             WHERE created_at >= ?1 OR updated_at >= ?1 ORDER BY updated_at",
        )?;
        let iois = iois_stmt
            .query_map(params![since], |row| {
                Ok(ChangedIoi {
                    id: row.get(0)?,
                    item_id: row.get(1)?,
                    title: row.get(2)?,
                    severity: row.get(3)?,
                    author: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(ChangesSince {
            items,
            notes,
            items_of_interest: iois,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::in_memory("test-project").unwrap()
    }

    #[test]
    fn project_get_returns_seeded_project() {
        let db = test_db();
        let p = db.project_get().unwrap();
        assert_eq!(p.name, "test-project");
    }

    #[test]
    fn changes_since_returns_recent_items() {
        let db = test_db();
        let before = chrono::Utc::now().to_rfc3339();
        db.item_create("httpd", "elf", None, None, "", &[]).unwrap();
        let changes = db.changes_since(&before).unwrap();
        assert_eq!(changes.items.len(), 1);
        assert_eq!(changes.items[0].name, "httpd");
    }
}
