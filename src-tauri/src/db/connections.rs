use rusqlite::params;

use super::error::{DbError, Result};
use super::models::Connection;
use super::{new_id, now, Database};

pub struct NewConnection<'a> {
    pub source_id: &'a str,
    pub source_type: &'a str,
    pub target_id: &'a str,
    pub target_type: &'a str,
    pub connection_type: &'a str,
    pub description: &'a str,
    pub author: &'a str,
    pub author_type: &'a str,
}

impl Database {
    pub fn connection_create(&self, params: &NewConnection<'_>) -> Result<Connection> {
        self.validate_connection_type(params.connection_type)?;
        self.validate_entity_ref(params.source_id, params.source_type)?;
        self.validate_entity_ref(params.target_id, params.target_type)?;

        let id = new_id();
        let ts = now();
        self.conn.execute(
            "INSERT INTO connections (id, source_id, source_type, target_id, target_type, connection_type, description, author, author_type, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![id, params.source_id, params.source_type, params.target_id, params.target_type, params.connection_type, params.description, params.author, params.author_type, ts],
        )?;
        Ok(Connection {
            id,
            source_id: params.source_id.to_string(),
            source_type: params.source_type.to_string(),
            target_id: params.target_id.to_string(),
            target_type: params.target_type.to_string(),
            connection_type: params.connection_type.to_string(),
            description: params.description.to_string(),
            author: params.author.to_string(),
            author_type: params.author_type.to_string(),
            created_at: ts,
        })
    }

    pub fn connection_list(
        &self,
        entity_id: &str,
        connection_type: Option<&str>,
    ) -> Result<Vec<Connection>> {
        let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            connection_type.map_or_else(
                || (
                    "SELECT id, source_id, source_type, target_id, target_type, connection_type, description, author, author_type, created_at
                     FROM connections WHERE source_id = ?1 OR target_id = ?1 ORDER BY created_at".to_string(),
                    vec![Box::new(entity_id.to_string()) as Box<dyn rusqlite::types::ToSql>],
                ),
                |ct| (
                    "SELECT id, source_id, source_type, target_id, target_type, connection_type, description, author, author_type, created_at
                     FROM connections WHERE (source_id = ?1 OR target_id = ?1) AND connection_type = ?2 ORDER BY created_at".to_string(),
                    vec![Box::new(entity_id.to_string()) as Box<dyn rusqlite::types::ToSql>, Box::new(ct.to_string())],
                ),
            );

        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let conns = stmt
            .query_map(params_ref.as_slice(), Self::row_to_connection)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(conns)
    }

    pub fn connection_list_all(&self) -> Result<Vec<Connection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, source_type, target_id, target_type, connection_type, description, author, author_type, created_at
             FROM connections ORDER BY created_at",
        )?;
        let conns = stmt
            .query_map([], Self::row_to_connection)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(conns)
    }

    pub fn connection_delete(&self, id: &str) -> Result<()> {
        let changes = self
            .conn
            .execute("DELETE FROM connections WHERE id = ?1", params![id])?;
        if changes == 0 {
            return Err(DbError::NotFound {
                entity: "connection".to_string(),
                id: id.to_string(),
            });
        }
        Ok(())
    }

    fn validate_entity_ref(&self, id: &str, entity_type: &str) -> Result<()> {
        let table = match entity_type {
            "item" => "items",
            "item_of_interest" => "items_of_interest",
            _ => {
                return Err(DbError::InvalidReference {
                    entity: entity_type.to_string(),
                    id: id.to_string(),
                });
            }
        };
        let exists: bool = self.conn.query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?1)"),
            params![id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(DbError::InvalidReference {
                entity: entity_type.to_string(),
                id: id.to_string(),
            });
        }
        Ok(())
    }

    fn row_to_connection(row: &rusqlite::Row) -> rusqlite::Result<Connection> {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::in_memory("test").unwrap()
    }

    fn create_two_items(db: &Database) -> (String, String) {
        let a = db
            .item_create("httpd", "elf", None, None, "", &[])
            .unwrap()
            .item
            .id;
        let b = db
            .item_create("libfoo.so", "shared_object", None, None, "", &[])
            .unwrap()
            .item
            .id;
        (a, b)
    }

    #[test]
    fn create_connection_between_items() {
        let db = test_db();
        let (a, b) = create_two_items(&db);
        let conn = db
            .connection_create(&NewConnection {
                source_id: &a,
                source_type: "item",
                target_id: &b,
                target_type: "item",
                connection_type: "links",
                description: "httpd links libfoo",
                author: "user",
                author_type: "human",
            })
            .unwrap();
        assert_eq!(conn.connection_type, "links");
    }

    #[test]
    fn list_connections_bidirectional() {
        let db = test_db();
        let (a, b) = create_two_items(&db);
        db.connection_create(&NewConnection {
            source_id: &a,
            source_type: "item",
            target_id: &b,
            target_type: "item",
            connection_type: "links",
            description: "desc",
            author: "user",
            author_type: "human",
        })
        .unwrap();

        let from_a = db.connection_list(&a, None).unwrap();
        assert_eq!(from_a.len(), 1);
        let from_b = db.connection_list(&b, None).unwrap();
        assert_eq!(from_b.len(), 1);
        assert_eq!(from_a[0].id, from_b[0].id);
    }

    #[test]
    fn list_connections_filtered_by_type() {
        let db = test_db();
        let (a, b) = create_two_items(&db);
        let c = db
            .item_create("httpd.conf", "config", None, None, "", &[])
            .unwrap()
            .item
            .id;
        db.connection_create(&NewConnection {
            source_id: &a,
            source_type: "item",
            target_id: &b,
            target_type: "item",
            connection_type: "links",
            description: "",
            author: "user",
            author_type: "human",
        })
        .unwrap();
        db.connection_create(&NewConnection {
            source_id: &a,
            source_type: "item",
            target_id: &c,
            target_type: "item",
            connection_type: "reads_config",
            description: "",
            author: "user",
            author_type: "human",
        })
        .unwrap();

        let links = db.connection_list(&a, Some("links")).unwrap();
        assert_eq!(links.len(), 1);
    }

    #[test]
    fn connection_list_all() {
        let db = test_db();
        let (a, b) = create_two_items(&db);
        db.connection_create(&NewConnection {
            source_id: &a,
            source_type: "item",
            target_id: &b,
            target_type: "item",
            connection_type: "links",
            description: "",
            author: "user",
            author_type: "human",
        })
        .unwrap();
        let all = db.connection_list_all().unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn invalid_connection_type_fails() {
        let db = test_db();
        let (a, b) = create_two_items(&db);
        let result = db.connection_create(&NewConnection {
            source_id: &a,
            source_type: "item",
            target_id: &b,
            target_type: "item",
            connection_type: "nonexistent_type",
            description: "",
            author: "user",
            author_type: "human",
        });
        assert!(matches!(
            result,
            Err(DbError::UnregisteredConnectionType(_))
        ));
    }

    #[test]
    fn invalid_entity_ref_fails() {
        let db = test_db();
        let a = db
            .item_create("httpd", "elf", None, None, "", &[])
            .unwrap()
            .item
            .id;
        let result = db.connection_create(&NewConnection {
            source_id: &a,
            source_type: "item",
            target_id: "nonexistent",
            target_type: "item",
            connection_type: "links",
            description: "",
            author: "user",
            author_type: "human",
        });
        assert!(matches!(result, Err(DbError::InvalidReference { .. })));
    }

    #[test]
    fn delete_connection() {
        let db = test_db();
        let (a, b) = create_two_items(&db);
        let conn = db
            .connection_create(&NewConnection {
                source_id: &a,
                source_type: "item",
                target_id: &b,
                target_type: "item",
                connection_type: "links",
                description: "",
                author: "user",
                author_type: "human",
            })
            .unwrap();
        db.connection_delete(&conn.id).unwrap();
        let all = db.connection_list_all().unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn item_delete_cascades_connections() {
        let db = test_db();
        let (a, b) = create_two_items(&db);
        db.connection_create(&NewConnection {
            source_id: &a,
            source_type: "item",
            target_id: &b,
            target_type: "item",
            connection_type: "links",
            description: "",
            author: "user",
            author_type: "human",
        })
        .unwrap();
        db.item_delete(&a).unwrap();
        let all = db.connection_list_all().unwrap();
        assert!(all.is_empty());
    }
}
