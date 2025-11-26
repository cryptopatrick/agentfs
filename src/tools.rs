//! Tool call recording and auditing
//!
//! This module provides functionality for recording and tracking tool calls made by AI agents.
//! It supports both a workflow-based API (start -> success/error) and a single-shot record API.

use crate::error::Result;
use agentdb::AgentDB;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Status of a tool call
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolCallStatus {
    Pending,
    Success,
    Error,
}

impl ToString for ToolCallStatus {
    fn to_string(&self) -> String {
        match self {
            ToolCallStatus::Pending => "pending".to_string(),
            ToolCallStatus::Success => "success".to_string(),
            ToolCallStatus::Error => "error".to_string(),
        }
    }
}

impl From<&str> for ToolCallStatus {
    fn from(s: &str) -> Self {
        match s {
            "success" => ToolCallStatus::Success,
            "error" => ToolCallStatus::Error,
            _ => ToolCallStatus::Pending,
        }
    }
}

/// Tool call record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub status: ToolCallStatus,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

/// Statistics for a specific tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallStats {
    pub name: String,
    pub total_calls: i64,
    pub successful: i64,
    pub failed: i64,
    pub avg_duration_ms: f64,
}

/// Tool recorder trait for auditing agent tool calls
#[async_trait]
pub trait ToolRecorder: Send + Sync {
    /// Start a new tool call and mark it as pending
    /// Returns the ID of the created tool call record
    async fn start(&self, name: &str, parameters: Option<serde_json::Value>) -> Result<i64>;

    /// Mark a tool call as successful
    async fn success(&self, id: i64, result: Option<serde_json::Value>) -> Result<()>;

    /// Mark a tool call as failed
    async fn error(&self, id: i64, error: &str) -> Result<()>;

    /// Get a specific tool call by ID
    async fn get(&self, id: i64) -> Result<Option<ToolCall>>;

    /// Get statistics for a specific tool
    async fn stats_for(&self, tool_name: &str) -> Result<Option<ToolCallStats>>;

    /// Record a completed tool call (single-shot method)
    /// Either result or error should be provided, not both
    /// Returns the ID of the created tool call record
    async fn record(
        &self,
        name: &str,
        started_at: i64,
        completed_at: i64,
        parameters: Option<serde_json::Value>,
        result: Option<serde_json::Value>,
        error: Option<&str>,
    ) -> Result<i64>;

    /// Get all tool calls (optionally limited)
    async fn list(&self, limit: Option<usize>) -> Result<Vec<ToolCall>>;
}

/// Database-backed tool recorder
pub struct DbToolRecorder {
    db: Arc<Box<dyn AgentDB>>,
}

impl DbToolRecorder {
    /// Create a new database-backed tool recorder
    pub fn new(db: Arc<Box<dyn AgentDB>>) -> Self {
        Self { db }
    }

    /// Get current Unix timestamp in seconds
    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    /// Parse a tool call from a database row
    fn parse_tool_call(&self, row: &agentdb::Row) -> Result<ToolCall> {
        let id = self.extract_i64(row, "id")?;
        let name = self.extract_string(row, "name")?;

        let parameters_str = self.extract_string_opt(row, "parameters")?;
        let parameters = parameters_str
            .filter(|s| !s.is_empty())
            .and_then(|s| serde_json::from_str(&s).ok());

        let result_str = self.extract_string_opt(row, "result")?;
        let result = result_str
            .filter(|s| !s.is_empty())
            .and_then(|s| serde_json::from_str(&s).ok());

        let error = self.extract_string_opt(row, "error")?
            .filter(|s| !s.is_empty());

        let status_str = self.extract_string(row, "status")?;
        let status = ToolCallStatus::from(status_str.as_str());

        let started_at = self.extract_i64(row, "started_at")?;
        let completed_at = self.extract_i64_opt(row, "completed_at")?;
        let duration_ms = self.extract_i64_opt(row, "duration_ms")?;

        Ok(ToolCall {
            id,
            name,
            parameters,
            result,
            error,
            status,
            started_at,
            completed_at,
            duration_ms,
        })
    }

    /// Extract an i64 from a row
    fn extract_i64(&self, row: &agentdb::Row, column: &str) -> Result<i64> {
        row.get(column)
            .ok_or_else(|| crate::error::AgentFsError::Database(
                agentdb::AgentDbError::Backend(format!("Missing column: {}", column))
            ))
            .and_then(|v| {
                let s = String::from_utf8_lossy(v.as_bytes());
                s.parse::<i64>().map_err(|e| {
                    crate::error::AgentFsError::Database(
                        agentdb::AgentDbError::Backend(format!("Invalid i64 for {}: {}", column, e))
                    )
                })
            })
    }

    /// Extract an optional i64 from a row
    fn extract_i64_opt(&self, row: &agentdb::Row, column: &str) -> Result<Option<i64>> {
        match row.get(column) {
            None => Ok(None),
            Some(v) => {
                // Empty bytes mean NULL
                if v.as_bytes().is_empty() {
                    return Ok(None);
                }
                let s = String::from_utf8_lossy(v.as_bytes());
                if s.is_empty() || s == "NULL" {
                    Ok(None)
                } else {
                    s.parse::<i64>()
                        .map(Some)
                        .map_err(|e| crate::error::AgentFsError::Database(
                            agentdb::AgentDbError::Backend(format!("Invalid i64 for {}: {}", column, e))
                        ))
                }
            }
        }
    }

    /// Extract a String from a row
    fn extract_string(&self, row: &agentdb::Row, column: &str) -> Result<String> {
        row.get(column)
            .ok_or_else(|| crate::error::AgentFsError::Database(
                agentdb::AgentDbError::Backend(format!("Missing column: {}", column))
            ))
            .map(|v| String::from_utf8_lossy(v.as_bytes()).to_string())
    }

    /// Extract an optional String from a row
    fn extract_string_opt(&self, row: &agentdb::Row, column: &str) -> Result<Option<String>> {
        Ok(row.get(column).and_then(|v| {
            // Empty bytes mean NULL
            if v.as_bytes().is_empty() {
                None
            } else {
                Some(String::from_utf8_lossy(v.as_bytes()).to_string())
            }
        }))
    }
}

#[async_trait]
impl ToolRecorder for DbToolRecorder {
    async fn start(&self, name: &str, parameters: Option<serde_json::Value>) -> Result<i64> {
        let serialized_params = parameters
            .map(|p| serde_json::to_string(&p))
            .transpose()?
            .unwrap_or_default();

        let started_at = Self::now();

        let query = format!(
            "INSERT INTO tool_calls (name, parameters, status, started_at) VALUES ('{}', '{}', 'pending', {})",
            name.replace('\'', "''"),
            serialized_params.replace('\'', "''"),
            started_at
        );

        self.db.query(&query, vec![]).await?;

        // Get the ID of the just-inserted row using rowid
        // This works across SQLite, PostgreSQL (with oid), and MySQL
        let result = self.db.query(
            "SELECT id FROM tool_calls WHERE rowid = last_insert_rowid()",
            vec![]
        ).await?;

        if let Some(row) = result.rows.first() {
            self.extract_i64(row, "id")
        } else {
            // Fallback: get MAX(id) which should be the just-inserted row
            let result = self.db.query("SELECT MAX(id) as id FROM tool_calls", vec![]).await?;
            if let Some(row) = result.rows.first() {
                self.extract_i64(row, "id")
            } else {
                Err(crate::error::AgentFsError::Database(
                    agentdb::AgentDbError::Backend("Failed to get tool call ID".to_string())
                ))
            }
        }
    }

    async fn success(&self, id: i64, result: Option<serde_json::Value>) -> Result<()> {
        let serialized_result = result
            .map(|r| serde_json::to_string(&r))
            .transpose()?
            .unwrap_or_default();

        let completed_at = Self::now();

        // Get the started_at time to calculate duration
        let query = format!("SELECT started_at FROM tool_calls WHERE id = {}", id);
        let res = self.db.query(&query, vec![]).await?;

        let started_at = if let Some(row) = res.rows.first() {
            self.extract_i64(row, "started_at")?
        } else {
            return Err(crate::error::AgentFsError::Database(
                agentdb::AgentDbError::Backend("Tool call not found".to_string())
            ));
        };

        let duration_ms = (completed_at - started_at) * 1000;

        let query = format!(
            "UPDATE tool_calls SET result = '{}', status = 'success', completed_at = {}, duration_ms = {} WHERE id = {}",
            serialized_result.replace('\'', "''"),
            completed_at,
            duration_ms,
            id
        );

        self.db.query(&query, vec![]).await?;
        Ok(())
    }

    async fn error(&self, id: i64, error: &str) -> Result<()> {
        let completed_at = Self::now();

        // Get the started_at time to calculate duration
        let query = format!("SELECT started_at FROM tool_calls WHERE id = {}", id);
        let res = self.db.query(&query, vec![]).await?;

        let started_at = if let Some(row) = res.rows.first() {
            self.extract_i64(row, "started_at")?
        } else {
            return Err(crate::error::AgentFsError::Database(
                agentdb::AgentDbError::Backend("Tool call not found".to_string())
            ));
        };

        let duration_ms = (completed_at - started_at) * 1000;

        let query = format!(
            "UPDATE tool_calls SET error = '{}', status = 'error', completed_at = {}, duration_ms = {} WHERE id = {}",
            error.replace('\'', "''"),
            completed_at,
            duration_ms,
            id
        );

        self.db.query(&query, vec![]).await?;
        Ok(())
    }

    async fn get(&self, id: i64) -> Result<Option<ToolCall>> {
        let query = format!(
            "SELECT id, name, parameters, result, error, status, started_at, completed_at, duration_ms FROM tool_calls WHERE id = {}",
            id
        );

        let result = self.db.query(&query, vec![]).await?;

        if let Some(row) = result.rows.first() {
            Ok(Some(self.parse_tool_call(row)?))
        } else {
            Ok(None)
        }
    }

    async fn stats_for(&self, tool_name: &str) -> Result<Option<ToolCallStats>> {
        let query = format!(
            "SELECT
                COUNT(*) as total_calls,
                SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) as successful,
                SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as failed,
                AVG(CASE WHEN duration_ms IS NOT NULL THEN duration_ms ELSE 0 END) as avg_duration_ms
            FROM tool_calls
            WHERE name = '{}'",
            tool_name.replace('\'', "''")
        );

        let result = self.db.query(&query, vec![]).await?;

        if let Some(row) = result.rows.first() {
            let total_calls = self.extract_i64(row, "total_calls")?;

            if total_calls == 0 {
                return Ok(None);
            }

            let successful = self.extract_i64(row, "successful")?;
            let failed = self.extract_i64(row, "failed")?;

            let avg_duration_str = self.extract_string(row, "avg_duration_ms")?;
            let avg_duration_ms = avg_duration_str.parse::<f64>().unwrap_or(0.0);

            Ok(Some(ToolCallStats {
                name: tool_name.to_string(),
                total_calls,
                successful,
                failed,
                avg_duration_ms,
            }))
        } else {
            Ok(None)
        }
    }

    async fn record(
        &self,
        name: &str,
        started_at: i64,
        completed_at: i64,
        parameters: Option<serde_json::Value>,
        result: Option<serde_json::Value>,
        error: Option<&str>,
    ) -> Result<i64> {
        let serialized_params = parameters
            .map(|p| serde_json::to_string(&p))
            .transpose()?
            .unwrap_or_default();

        let serialized_result = result
            .map(|r| serde_json::to_string(&r))
            .transpose()?
            .unwrap_or_default();

        let duration_ms = (completed_at - started_at) * 1000;
        let status = if error.is_some() { "error" } else { "success" };

        let query = format!(
            "INSERT INTO tool_calls (name, parameters, result, error, status, started_at, completed_at, duration_ms)
             VALUES ('{}', '{}', '{}', '{}', '{}', {}, {}, {})",
            name.replace('\'', "''"),
            serialized_params.replace('\'', "''"),
            serialized_result.replace('\'', "''"),
            error.unwrap_or("").replace('\'', "''"),
            status,
            started_at,
            completed_at,
            duration_ms
        );

        self.db.query(&query, vec![]).await?;

        // Get the ID of the just-inserted row using rowid
        let result = self.db.query(
            "SELECT id FROM tool_calls WHERE rowid = last_insert_rowid()",
            vec![]
        ).await?;

        if let Some(row) = result.rows.first() {
            self.extract_i64(row, "id")
        } else {
            // Fallback: get MAX(id) which should be the just-inserted row
            let result = self.db.query("SELECT MAX(id) as id FROM tool_calls", vec![]).await?;
            if let Some(row) = result.rows.first() {
                self.extract_i64(row, "id")
            } else {
                Err(crate::error::AgentFsError::Database(
                    agentdb::AgentDbError::Backend("Failed to get tool call ID".to_string())
                ))
            }
        }
    }

    async fn list(&self, limit: Option<usize>) -> Result<Vec<ToolCall>> {
        let limit_clause = limit
            .map(|l| format!(" LIMIT {}", l))
            .unwrap_or_default();

        let query = format!(
            "SELECT id, name, parameters, result, error, status, started_at, completed_at, duration_ms
             FROM tool_calls
             ORDER BY started_at DESC{}",
            limit_clause
        );

        let result = self.db.query(&query, vec![]).await?;

        let mut tool_calls = Vec::new();
        for row in &result.rows {
            tool_calls.push(self.parse_tool_call(row)?);
        }

        Ok(tool_calls)
    }
}
