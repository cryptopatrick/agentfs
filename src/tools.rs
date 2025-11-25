//! Tool call recording and auditing

use crate::error::Result;
use agentdb::AgentDB;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Tool call record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub tool_name: String,
    pub params: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub agent_id: String,
}

/// Tool recorder trait for auditing agent tool calls
#[async_trait]
pub trait ToolRecorder: Send + Sync {
    /// Record a tool call
    async fn record(
        &self,
        tool_name: &str,
        params: serde_json::Value,
        result: Option<serde_json::Value>,
        start_time: DateTime<Utc>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<String>;

    /// Query tool calls
    async fn query(
        &self,
        tool_name: Option<&str>,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ToolCall>>;

    /// Get a specific tool call by ID
    async fn get(&self, id: &str) -> Result<Option<ToolCall>>;

    /// Get all tool calls for this agent
    async fn list(&self, limit: Option<usize>) -> Result<Vec<ToolCall>>;
}

/// Database-backed tool recorder
pub struct DbToolRecorder {
    db: Arc<Box<dyn AgentDB>>,
    agent_id: String,
}

impl DbToolRecorder {
    /// Create a new database-backed tool recorder
    pub fn new(db: Arc<Box<dyn AgentDB>>, agent_id: String) -> Self {
        Self { db, agent_id }
    }
}

#[async_trait]
impl ToolRecorder for DbToolRecorder {
    async fn record(
        &self,
        tool_name: &str,
        params: serde_json::Value,
        result: Option<serde_json::Value>,
        start_time: DateTime<Utc>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();

        let params_str = serde_json::to_string(&params)?;
        let result_str = result
            .as_ref()
            .map(|r| serde_json::to_string(r))
            .transpose()?;

        let query = format!(
            "INSERT INTO tool_calls (id, tool_name, params, result, start_time, end_time, agent_id) VALUES ('{}', '{}', '{}', {}, '{}', {}, '{}')",
            id,
            tool_name,
            params_str.replace('\'', "''"),
            result_str
                .as_ref()
                .map(|s| format!("'{}'", s.replace('\'', "''")))
                .unwrap_or_else(|| "NULL".to_string()),
            start_time.to_rfc3339(),
            end_time
                .map(|t| format!("'{}'", t.to_rfc3339()))
                .unwrap_or_else(|| "NULL".to_string()),
            self.agent_id
        );

        self.db.query(&query, vec![]).await?;
        Ok(id)
    }

    async fn query(
        &self,
        tool_name: Option<&str>,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ToolCall>> {
        let mut conditions = vec![format!("agent_id = '{}'", self.agent_id)];

        if let Some(name) = tool_name {
            conditions.push(format!("tool_name = '{}'", name));
        }

        if let Some(time) = since {
            conditions.push(format!("start_time >= '{}'", time.to_rfc3339()));
        }

        let where_clause = conditions.join(" AND ");
        let query = format!(
            "SELECT id, tool_name, params, result, start_time, end_time, agent_id FROM tool_calls WHERE {} ORDER BY start_time DESC",
            where_clause
        );

        let result = self.db.query(&query, vec![]).await?;

        let mut tool_calls = Vec::new();
        for row in result.rows {
            // Parse row data (simplified)
            let id = String::from_utf8_lossy(
                row.get("id")
                    .ok_or_else(|| crate::error::AgentFsError::Database(agentdb::AgentDbError::Backend("Missing id".to_string())))?
                    .as_bytes(),
            )
            .to_string();

            let tool_name_val = String::from_utf8_lossy(
                row.get("tool_name")
                    .ok_or_else(|| crate::error::AgentFsError::Database(agentdb::AgentDbError::Backend("Missing tool_name".to_string())))?
                    .as_bytes(),
            )
            .to_string();

            // For now, create a simplified ToolCall
            tool_calls.push(ToolCall {
                id,
                tool_name: tool_name_val,
                params: serde_json::Value::Null,
                result: None,
                start_time: Utc::now(),
                end_time: None,
                agent_id: self.agent_id.clone(),
            });
        }

        Ok(tool_calls)
    }

    async fn get(&self, id: &str) -> Result<Option<ToolCall>> {
        let query = format!(
            "SELECT id, tool_name, params, result, start_time, end_time, agent_id FROM tool_calls WHERE id = '{}' AND agent_id = '{}'",
            id, self.agent_id
        );

        let result = self.db.query(&query, vec![]).await?;

        if result.rows.is_empty() {
            return Ok(None);
        }

        // Parse the row (simplified)
        Ok(Some(ToolCall {
            id: id.to_string(),
            tool_name: "unknown".to_string(),
            params: serde_json::Value::Null,
            result: None,
            start_time: Utc::now(),
            end_time: None,
            agent_id: self.agent_id.clone(),
        }))
    }

    async fn list(&self, limit: Option<usize>) -> Result<Vec<ToolCall>> {
        let limit_clause = limit
            .map(|l| format!(" LIMIT {}", l))
            .unwrap_or_default();

        let query = format!(
            "SELECT id, tool_name, params, result, start_time, end_time, agent_id FROM tool_calls WHERE agent_id = '{}' ORDER BY start_time DESC{}",
            self.agent_id, limit_clause
        );

        let result = self.db.query(&query, vec![]).await?;

        // Simplified parsing
        let mut tool_calls = Vec::new();
        for _ in 0..result.rows.len() {
            tool_calls.push(ToolCall {
                id: Uuid::new_v4().to_string(),
                tool_name: "unknown".to_string(),
                params: serde_json::Value::Null,
                result: None,
                start_time: Utc::now(),
                end_time: None,
                agent_id: self.agent_id.clone(),
            });
        }

        Ok(tool_calls)
    }
}
