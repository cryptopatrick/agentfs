//! Key-value store for agent state

use crate::error::Result;
use agentdb::{AgentDB, Value};
use async_trait::async_trait;
use std::sync::Arc;

/// Key-value store trait
#[async_trait]
pub trait KvStore: Send + Sync {
    /// Set a key-value pair
    async fn set(&self, key: &str, value: &[u8]) -> Result<()>;

    /// Get a value by key
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Delete a key
    async fn delete(&self, key: &str) -> Result<()>;

    /// Check if a key exists
    async fn exists(&self, key: &str) -> Result<bool>;

    /// List all keys with a given prefix
    async fn scan(&self, prefix: &str) -> Result<Vec<String>>;
}

/// Database-backed key-value store
pub struct DbKvStore {
    db: Arc<Box<dyn AgentDB>>,
    namespace: String,
}

impl DbKvStore {
    /// Create a new database-backed KV store
    pub fn new(db: Arc<Box<dyn AgentDB>>, namespace: String) -> Self {
        Self { db, namespace }
    }

    /// Add namespace prefix to key
    fn namespaced_key(&self, key: &str) -> String {
        format!("kv:{}:{}", self.namespace, key)
    }

    /// Remove namespace prefix from key
    fn strip_namespace(&self, key: &str) -> String {
        key.strip_prefix(&format!("kv:{}:", self.namespace))
            .unwrap_or(key)
            .to_string()
    }
}

#[async_trait]
impl KvStore for DbKvStore {
    async fn set(&self, key: &str, value: &[u8]) -> Result<()> {
        let namespaced = self.namespaced_key(key);
        self.db.put(&namespaced, Value::from(value)).await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let namespaced = self.namespaced_key(key);
        match self.db.get(&namespaced).await? {
            Some(value) => Ok(Some(value.as_bytes().to_vec())),
            None => Ok(None),
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let namespaced = self.namespaced_key(key);
        self.db.delete(&namespaced).await?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let namespaced = self.namespaced_key(key);
        Ok(self.db.exists(&namespaced).await?)
    }

    async fn scan(&self, prefix: &str) -> Result<Vec<String>> {
        let namespaced_prefix = self.namespaced_key(prefix);
        let result = self.db.scan(&namespaced_prefix).await?;

        let keys = result
            .keys
            .into_iter()
            .map(|k| self.strip_namespace(&k))
            .collect();

        Ok(keys)
    }
}
