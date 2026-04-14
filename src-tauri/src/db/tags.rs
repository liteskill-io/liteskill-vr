use rusqlite::params;

use super::error::{DbError, Result};
use super::models::Tag;
use super::{new_id, now, Database};

impl Database {
    pub fn tag_list(&self) -> Result<Vec<Tag>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, description, color, created_at FROM tags ORDER BY name")?;
        let tags = stmt
            .query_map([], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    color: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(tags)
    }

    pub fn tag_create(&self, name: &str, description: &str, color: Option<&str>) -> Result<Tag> {
        let existing: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)",
            params![name],
            |row| row.get(0),
        )?;
        if existing {
            return Err(DbError::DuplicateName {
                entity: "tag".to_string(),
                name: name.to_string(),
            });
        }

        let id = new_id();
        let ts = now();
        self.conn.execute(
            "INSERT INTO tags (id, name, description, color, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, description, color, ts],
        )?;
        Ok(Tag {
            id,
            name: name.to_string(),
            description: description.to_string(),
            color: color.map(String::from),
            created_at: ts,
        })
    }

    pub fn tag_delete(&self, id: &str) -> Result<()> {
        let changes = self
            .conn
            .execute("DELETE FROM tags WHERE id = ?1", params![id])?;
        if changes == 0 {
            return Err(DbError::NotFound {
                entity: "tag".to_string(),
                id: id.to_string(),
            });
        }
        Ok(())
    }

    pub fn validate_tags(&self, tags: &[String]) -> Result<()> {
        for tag in tags {
            let exists: bool = self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)",
                params![tag],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(DbError::UnregisteredTag(tag.clone()));
            }
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

    #[test]
    fn list_returns_default_tags() {
        let db = test_db();
        let tags = db.tag_list().unwrap();
        assert!(tags.len() >= 13);
        let names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"memory-corruption"));
        assert!(names.contains(&"interesting"));
    }

    #[test]
    fn create_and_delete() {
        let db = test_db();
        let tag = db
            .tag_create("custom-tag", "A custom tag", Some("#ff0000"))
            .unwrap();
        assert_eq!(tag.name, "custom-tag");
        assert_eq!(tag.color.as_deref(), Some("#ff0000"));

        let tags = db.tag_list().unwrap();
        assert!(tags.iter().any(|t| t.name == "custom-tag"));

        db.tag_delete(&tag.id).unwrap();
        let tags = db.tag_list().unwrap();
        assert!(!tags.iter().any(|t| t.name == "custom-tag"));
    }

    #[test]
    fn duplicate_name_fails() {
        let db = test_db();
        let result = db.tag_create("memory-corruption", "duplicate", None);
        assert!(matches!(result, Err(DbError::DuplicateName { .. })));
    }

    #[test]
    fn delete_nonexistent_fails() {
        let db = test_db();
        let result = db.tag_delete("nonexistent-id");
        assert!(matches!(result, Err(DbError::NotFound { .. })));
    }

    #[test]
    fn validate_tags_passes_for_registered() {
        let db = test_db();
        let result =
            db.validate_tags(&["memory-corruption".to_string(), "interesting".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_tags_fails_for_unregistered() {
        let db = test_db();
        let result = db.validate_tags(&["nonexistent-tag".to_string()]);
        assert!(matches!(result, Err(DbError::UnregisteredTag(_))));
    }
}
