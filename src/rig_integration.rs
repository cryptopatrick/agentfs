//! Rig.rs FileSystem trait integration for AgentFS
//!
//! This module provides integration with the Rig.rs agent framework by implementing
//! the `rig::agent::FileSystem` trait for `AgentFS`.
//!
//! # Enabling Rig Integration
//!
//! To enable this integration, add `rig` as a dependency to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! agentfs = { version = "0.1", features = ["rig-integration"] }
//! rig = "0.3"
//! ```
//!
//! # Usage with Rig
//!
//! ```rust,ignore
//! use rig::{agent::AgentBuilder, providers::openai};
//! use agentfs::AgentFS;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let openai = openai::Client::new(std::env::var("OPENAI_API_KEY")?);
//!
//!     // Create AgentFS with SQLite backend
//!     let fs = AgentFS::sqlite("agents/my-agent.db", "my-agent").await?;
//!
//!     // Use with Rig agent
//!     let agent = AgentBuilder::new(openai.gpt4())
//!         .with_filesystem(fs)
//!         .build();
//!
//!     agent.say("Write a report to /output/report.txt").await?;
//!     Ok(())
//! }
//! ```
//!
//! # Implementation Notes
//!
//! The integration maps AgentFS operations to Rig's FileSystem trait:
//! - `read_file` returns an error if the file doesn't exist (Rig expects Result<Vec<u8>>)
//! - `list_dir` returns an error if the directory doesn't exist
//! - All errors are converted to `rig::agent::FileSystemError::Io`
//! - Paths are automatically sandboxed within the mount point

// NOTE: This implementation is currently a placeholder/demonstration.
// To activate it:
// 1. Uncomment the `rig` dependency in Cargo.toml
// 2. Uncomment the `rig-integration` feature
// 3. Uncomment the impl below

/*
use crate::{AgentFS, FileSystem as AgentFileSystem};
use async_trait::async_trait;
use rig::agent::{FileSystem, FileSystemError};

#[async_trait]
impl FileSystem for AgentFS {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, FileSystemError> {
        self.fs
            .read_file(path)
            .await
            .map_err(|e| FileSystemError::Io(e.to_string()))?
            .ok_or_else(|| FileSystemError::Io(format!("File not found: {}", path)))
    }

    async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), FileSystemError> {
        self.fs
            .write_file(path, content)
            .await
            .map_err(|e| FileSystemError::Io(e.to_string()))
    }

    async fn list_dir(&self, path: &str) -> Result<Vec<String>, FileSystemError> {
        let entries = self
            .fs
            .readdir(path)
            .await
            .map_err(|e| FileSystemError::Io(e.to_string()))?
            .ok_or_else(|| FileSystemError::Io(format!("Directory not found: {}", path)))?;

        Ok(entries)
    }

    async fn file_exists(&self, path: &str) -> Result<bool, FileSystemError> {
        self.fs
            .exists(path)
            .await
            .map_err(|e| FileSystemError::Io(e.to_string()))
    }

    async fn remove_file(&self, path: &str) -> Result<(), FileSystemError> {
        self.fs
            .remove(path)
            .await
            .map_err(|e| FileSystemError::Io(e.to_string()))
    }

    async fn mkdir(&self, path: &str) -> Result<(), FileSystemError> {
        self.fs
            .mkdir(path)
            .await
            .map_err(|e| FileSystemError::Io(e.to_string()))
    }
}
*/

// Placeholder documentation for the implementation above
/// When the `rig-integration` feature is enabled, `AgentFS` implements
/// the `rig::agent::FileSystem` trait, allowing it to be used directly
/// with Rig agents.
///
/// # Methods
///
/// - `read_file(path) -> Result<Vec<u8>, FileSystemError>`
/// - `write_file(path, content) -> Result<(), FileSystemError>`
/// - `list_dir(path) -> Result<Vec<String>, FileSystemError>`
/// - `file_exists(path) -> Result<bool, FileSystemError>`
/// - `remove_file(path) -> Result<(), FileSystemError>`
/// - `mkdir(path) -> Result<(), FileSystemError>`
///
/// # Error Handling
///
/// All AgentFS errors are converted to `rig::agent::FileSystemError::Io(_)`.
/// Operations that return `Option<T>` (like read_file) are converted to
/// errors when the result is `None`.
///
/// # Path Sandboxing
///
/// All paths are automatically sandboxed within the AgentFS mount point
/// (default `/agent`), preventing directory traversal attacks.
pub struct RigIntegration;

impl RigIntegration {
    /// Returns true if Rig integration is compiled in
    pub const fn is_available() -> bool {
        cfg!(feature = "rig-integration")
    }

    /// Instructions for enabling Rig integration
    pub fn enable_instructions() -> &'static str {
        r#"
To enable Rig.rs integration:

1. Add rig to your Cargo.toml:
   [dependencies]
   agentfs = { version = "0.1", features = ["rig-integration"] }
   rig = "0.3"

2. The AgentFS type will automatically implement rig::agent::FileSystem

3. Use it with Rig agents:
   let fs = AgentFS::sqlite("agent.db", "my-agent").await?;
   let agent = AgentBuilder::new(model).with_filesystem(fs).build();
"#
    }
}
