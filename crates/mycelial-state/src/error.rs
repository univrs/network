//! Error types for the state management layer

use thiserror::Error;

/// Errors that can occur in state operations
#[derive(Error, Debug)]
pub enum StateError {
    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Record not found
    #[error("{entity} not found: {id}")]
    NotFound { entity: String, id: String },

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Invalid data
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Duplicate entry
    #[error("Duplicate {entity}: {id}")]
    Duplicate { entity: String, id: String },

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(String),

    /// Cache error
    #[error("Cache error: {0}")]
    Cache(String),

    /// Sync error
    #[error("Sync error: {0}")]
    Sync(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<sqlx::Error> for StateError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => StateError::NotFound {
                entity: "record".to_string(),
                id: "unknown".to_string(),
            },
            sqlx::Error::Database(db_err) => {
                if db_err.message().contains("UNIQUE constraint") {
                    StateError::Duplicate {
                        entity: "record".to_string(),
                        id: "unknown".to_string(),
                    }
                } else {
                    StateError::Database(db_err.to_string())
                }
            }
            _ => StateError::Database(err.to_string()),
        }
    }
}

impl From<sqlx::migrate::MigrateError> for StateError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        StateError::Migration(err.to_string())
    }
}

impl From<serde_json::Error> for StateError {
    fn from(err: serde_json::Error) -> Self {
        StateError::Serialization(err.to_string())
    }
}

/// Result type for state operations
pub type Result<T> = std::result::Result<T, StateError>;
