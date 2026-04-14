use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Entity not found: {entity} with id {id}")]
    NotFound { entity: String, id: String },

    #[error("Duplicate name: {entity} '{name}' already exists")]
    DuplicateName { entity: String, name: String },

    #[error("Invalid reference: tag '{0}' is not registered")]
    UnregisteredTag(String),

    #[error("Invalid reference: connection type '{0}' is not registered")]
    UnregisteredConnectionType(String),

    #[error("Invalid reference: {entity} '{id}' does not exist")]
    InvalidReference { entity: String, id: String },

    #[error("Bulk delete requires at least one filter")]
    BulkDeleteNoFilter,
}

pub type Result<T> = std::result::Result<T, DbError>;
