//! Error types for AgentFS

use thiserror::Error;

/// Result type for AgentFS operations
pub type Result<T> = std::result::Result<T, AgentFsError>;

/// Error types for AgentFS operations
#[derive(Error, Debug)]
pub enum AgentFsError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("Path already exists: {0}")]
    PathExists(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Path traversal attempt: {0}")]
    PathTraversal(String),

    #[error("Database error: {0}")]
    Database(#[from] agentdb::AgentDbError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
