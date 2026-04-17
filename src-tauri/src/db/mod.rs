mod connection_types;
mod connections;
pub mod error;
mod ioi;
mod items;
pub mod migrations;
pub mod models;
mod notes;
mod project;
mod search;
mod tags;

pub use connections::NewConnection;
pub use ioi::NewIoi;

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

    /// Open the db at `path` if it exists, otherwise create and seed it with `name`.
    pub fn open_or_init(path: &Path, name: &str) -> Result<Self> {
        if path.exists() {
            Self::open(path)
        } else {
            Self::open_and_seed(path, name)
        }
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

// Tag names are concatenated with ASCII Unit Separator (\x1f) in list queries,
// which avoids N+1 lookups while remaining safe against any byte sequence a user
// might put in a tag name.
const TAG_SEP: char = '\x1f';

fn parse_tag_list(s: Option<String>) -> Vec<String> {
    s.map(|s| {
        s.split(TAG_SEP)
            .filter(|t| !t.is_empty())
            .map(String::from)
            .collect()
    })
    .unwrap_or_default()
}
