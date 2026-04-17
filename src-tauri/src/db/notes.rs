use rusqlite::params;

use super::error::{DbError, Result};
use super::models::{Note, NoteWithTags};
use super::{new_id, now, Database};

impl Database {
    pub fn note_create(
        &self,
        item_id: Option<&str>,
        title: &str,
        content: &str,
        author: &str,
        author_type: &str,
        tags: &[String],
    ) -> Result<NoteWithTags> {
        if let Some(item_id) = item_id {
            self.get_item_by_id(item_id)
                .map_err(|_| DbError::InvalidReference {
                    entity: "item".to_string(),
                    id: item_id.to_string(),
                })?;
        }
        self.validate_tags(tags)?;

        let id = new_id();
        let ts = now();
        self.conn.execute(
            "INSERT INTO notes (id, item_id, title, content, author, author_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![id, item_id, title, content, author, author_type, ts],
        )?;
        self.set_note_tags(&id, tags)?;
        Ok(NoteWithTags {
            note: Note {
                id,
                item_id: item_id.map(String::from),
                title: title.to_string(),
                content: content.to_string(),
                author: author.to_string(),
                author_type: author_type.to_string(),
                created_at: ts.clone(),
                updated_at: ts,
            },
            tags: tags.to_vec(),
        })
    }

    pub fn note_update(
        &self,
        id: &str,
        title: Option<&str>,
        content: Option<&str>,
        tags: Option<&[String]>,
    ) -> Result<NoteWithTags> {
        self.get_note_by_id(id)?;
        if let Some(tags) = tags {
            self.validate_tags(tags)?;
        }

        let ts = now();
        self.conn.execute(
            "UPDATE notes SET
                title = COALESCE(?1, title),
                content = COALESCE(?2, content),
                updated_at = ?3
             WHERE id = ?4",
            params![title, content, ts, id],
        )?;
        if let Some(tags) = tags {
            self.set_note_tags(id, tags)?;
        }

        let note = self.get_note_by_id(id)?;
        let tags = self.get_note_tags(id)?;
        Ok(NoteWithTags { note, tags })
    }

    pub fn note_delete(&self, id: &str) -> Result<()> {
        let changes = self
            .conn
            .execute("DELETE FROM notes WHERE id = ?1", params![id])?;
        if changes == 0 {
            return Err(DbError::NotFound {
                entity: "note".to_string(),
                id: id.to_string(),
            });
        }
        Ok(())
    }

    pub(crate) fn get_note_by_id(&self, id: &str) -> Result<Note> {
        self.conn
            .query_row(
                "SELECT id, item_id, title, content, author, author_type, created_at, updated_at
                 FROM notes WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Note {
                        id: row.get(0)?,
                        item_id: row.get(1)?,
                        title: row.get(2)?,
                        content: row.get(3)?,
                        author: row.get(4)?,
                        author_type: row.get(5)?,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                    entity: "note".to_string(),
                    id: id.to_string(),
                },
                other => DbError::Sqlite(other),
            })
    }

    pub(crate) fn get_note_tags(&self, note_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag_name FROM note_tags WHERE note_id = ?1 ORDER BY tag_name")?;
        let tags = stmt
            .query_map(params![note_id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(tags)
    }

    fn set_note_tags(&self, note_id: &str, tags: &[String]) -> Result<()> {
        self.conn
            .execute("DELETE FROM note_tags WHERE note_id = ?1", params![note_id])?;
        for tag in tags {
            self.conn.execute(
                "INSERT INTO note_tags (note_id, tag_name) VALUES (?1, ?2)",
                params![note_id, tag],
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
    fn create_and_update_note() {
        let db = test_db();
        let item_id = create_test_item(&db);
        let note = db
            .note_create(
                Some(&item_id),
                "Analysis",
                "Found a bug",
                "user",
                "human",
                &["memory-corruption".to_string()],
            )
            .unwrap();
        assert_eq!(note.note.title, "Analysis");
        assert_eq!(note.tags, vec!["memory-corruption"]);

        let updated = db
            .note_update(&note.note.id, Some("Updated title"), None, None)
            .unwrap();
        assert_eq!(updated.note.title, "Updated title");
        assert_eq!(updated.note.content, "Found a bug");
    }

    #[test]
    fn delete_note() {
        let db = test_db();
        let item_id = create_test_item(&db);
        let note = db
            .note_create(Some(&item_id), "test", "content", "user", "human", &[])
            .unwrap();
        db.note_delete(&note.note.id).unwrap();
        let result = db.get_note_by_id(&note.note.id);
        assert!(matches!(result, Err(DbError::NotFound { .. })));
    }

    #[test]
    fn create_note_for_nonexistent_item_fails() {
        let db = test_db();
        let result = db.note_create(Some("nonexistent"), "test", "content", "user", "human", &[]);
        assert!(matches!(result, Err(DbError::InvalidReference { .. })));
    }
}
