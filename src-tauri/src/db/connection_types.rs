use rusqlite::params;

use super::error::{DbError, Result};
use super::models::ConnectionType;
use super::{new_id, now, Database};

impl Database {
    pub fn connection_type_list(&self) -> Result<Vec<ConnectionType>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, created_at FROM connection_types ORDER BY name",
        )?;
        let types = stmt
            .query_map([], |row| {
                Ok(ConnectionType {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(types)
    }

    pub fn connection_type_create(&self, name: &str, description: &str) -> Result<ConnectionType> {
        let existing: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM connection_types WHERE name = ?1)",
            params![name],
            |row| row.get(0),
        )?;
        if existing {
            return Err(DbError::DuplicateName {
                entity: "connection_type".to_string(),
                name: name.to_string(),
            });
        }

        let id = new_id();
        let ts = now();
        self.conn.execute(
            "INSERT INTO connection_types (id, name, description, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, description, ts],
        )?;
        Ok(ConnectionType {
            id,
            name: name.to_string(),
            description: description.to_string(),
            created_at: ts,
        })
    }

    pub fn connection_type_delete(&self, id: &str) -> Result<()> {
        let changes = self
            .conn
            .execute("DELETE FROM connection_types WHERE id = ?1", params![id])?;
        if changes == 0 {
            return Err(DbError::NotFound {
                entity: "connection_type".to_string(),
                id: id.to_string(),
            });
        }
        Ok(())
    }

    pub fn validate_connection_type(&self, name: &str) -> Result<()> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM connection_types WHERE name = ?1)",
            params![name],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(DbError::UnregisteredConnectionType(name.to_string()));
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
    fn list_returns_defaults() {
        let db = test_db();
        let types = db.connection_type_list().unwrap();
        assert!(types.len() >= 7);
        let names: Vec<&str> = types.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"calls"));
        assert!(names.contains(&"related"));
    }

    #[test]
    fn create_and_validate() {
        let db = test_db();
        db.connection_type_create("monitors", "Source monitors target")
            .unwrap();
        assert!(db.validate_connection_type("monitors").is_ok());
    }

    #[test]
    fn duplicate_fails() {
        let db = test_db();
        let result = db.connection_type_create("calls", "duplicate");
        assert!(matches!(result, Err(DbError::DuplicateName { .. })));
    }

    #[test]
    fn validate_unregistered_fails() {
        let db = test_db();
        let result = db.validate_connection_type("nonexistent");
        assert!(matches!(
            result,
            Err(DbError::UnregisteredConnectionType(_))
        ));
    }
}
