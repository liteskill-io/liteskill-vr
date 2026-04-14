mod connection_types;
mod connections;
pub mod error;
mod ioi;
mod items;
pub mod migrations;
pub mod models;
mod notes;
mod tags;

pub use connections::NewConnection;
pub use ioi::NewIoi;
mod search;

use rusqlite::Connection;
use std::path::Path;

use error::Result;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        migrations::run_migrations(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_and_seed(path: &Path, project_name: &str) -> Result<Self> {
        let db = Self::open(path)?;
        migrations::seed_defaults(&db.conn, project_name)?;
        Ok(db)
    }

    pub fn in_memory(project_name: &str) -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        migrations::run_migrations(&conn)?;
        migrations::seed_defaults(&conn, project_name)?;
        Ok(Self { conn })
    }

    pub const fn conn(&self) -> &Connection {
        &self.conn
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
