//! # AgentFS - Filesystem Abstraction for AI Agents
//!
//! AgentFS provides a high-level filesystem abstraction for AI agents,
//! offering POSIX-like file operations, key-value storage, and tool call auditing.
//!
//! ## Features
//!
//! - **Filesystem**: POSIX-like file and directory operations
//! - **KV Store**: Key-value storage for agent state
//! - **Tool Recording**: Audit trail for agent tool calls
//! - **Backend Agnostic**: Works with any AgentDB backend (SQL, KV, Graph)
//!
//! ## Example
//!
//! ```rust,ignore
//! use agentfs::AgentFS;
//! use agentsql::{SqlBackend, SqlBackendConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a SQLite-backed agent filesystem
//!     let backend = SqlBackend::sqlite("agent.db").await?;
//!     let agent_fs = AgentFS::new(Box::new(backend), "my-agent", "/agent").await?;
//!
//!     // Write a file
//!     agent_fs.fs.write_file("/output/report.txt", b"Hello World").await?;
//!
//!     // Store state
//!     agent_fs.kv.set("session_id", b"12345").await?;
//!
//!     // Record a tool call
//!     agent_fs.tools.record(
//!         "web_search",
//!         serde_json::json!({"query": "Rust async"}),
//!         Some(serde_json::json!({"results": []})),
//!         chrono::Utc::now(),
//!         Some(chrono::Utc::now()),
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod filesystem;
pub mod kvstore;
pub mod tools;

pub use error::{AgentFsError, Result};
pub use filesystem::{DbFileSystem, FileSystem, Stats};
pub use kvstore::{DbKvStore, KvStore};
pub use tools::{DbToolRecorder, ToolCall, ToolRecorder};

use agentdb::AgentDB;
use std::path::PathBuf;
use std::sync::Arc;

/// Main AgentFS struct providing filesystem, KV store, and tool recording
pub struct AgentFS {
    /// Filesystem operations
    pub fs: DbFileSystem,

    /// Key-value store
    pub kv: DbKvStore,

    /// Tool call recorder
    pub tools: DbToolRecorder,

    /// Agent identifier
    pub agent_id: String,

    /// Mount path for the filesystem
    pub mount_path: PathBuf,
}

impl AgentFS {
    /// Create a new AgentFS instance
    ///
    /// # Arguments
    ///
    /// * `db` - Database backend implementing AgentDB trait
    /// * `agent_id` - Unique identifier for this agent
    /// * `mount_path` - Virtual mount path for the filesystem (default: "/agent")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let backend = SqlBackend::sqlite("agent.db").await?;
    /// let agent_fs = AgentFS::new(Box::new(backend), "my-agent", "/agent").await?;
    /// ```
    pub async fn new(
        db: Box<dyn AgentDB>,
        agent_id: impl Into<String>,
        mount_path: impl Into<PathBuf>,
    ) -> Result<Self> {
        let agent_id = agent_id.into();
        let mount_path = mount_path.into();

        // Wrap database in Arc for shared ownership
        let db_arc = Arc::new(db);

        Ok(Self {
            fs: DbFileSystem::new(db_arc.clone()),
            kv: DbKvStore::new(db_arc.clone(), agent_id.clone()),
            tools: DbToolRecorder::new(db_arc, agent_id.clone()),
            agent_id,
            mount_path,
        })
    }

    /// Convenience constructor for SQLite backend
    #[cfg(feature = "sqlite")]
    pub async fn sqlite(
        path: impl AsRef<std::path::Path>,
        agent_id: impl Into<String>,
    ) -> Result<Self> {
        use agentsql::SqlBackend;

        let backend = SqlBackend::sqlite(path.as_ref().to_string_lossy().to_string())
            .await
            .map_err(|e| AgentFsError::Database(agentdb::AgentDbError::Backend(e.to_string())))?;

        Self::new(Box::new(backend), agent_id, "/agent").await
    }

    /// Convenience constructor for PostgreSQL backend
    #[cfg(feature = "postgres")]
    pub async fn postgres(url: impl Into<String>, agent_id: impl Into<String>) -> Result<Self> {
        use agentsql::SqlBackend;

        let backend = SqlBackend::postgres(url.into())
            .await
            .map_err(|e| AgentFsError::Database(agentdb::AgentDbError::Backend(e.to_string())))?;

        Self::new(Box::new(backend), agent_id, "/agent").await
    }

    /// Convenience constructor for MySQL backend
    #[cfg(feature = "mysql")]
    pub async fn mysql(url: impl Into<String>, agent_id: impl Into<String>) -> Result<Self> {
        use agentsql::SqlBackend;

        let backend = SqlBackend::mysql(url.into())
            .await
            .map_err(|e| AgentFsError::Database(agentdb::AgentDbError::Backend(e.to_string())))?;

        Self::new(Box::new(backend), agent_id, "/agent").await
    }

    /// Get the agent ID
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Get the mount path
    pub fn mount_path(&self) -> &PathBuf {
        &self.mount_path
    }
}
