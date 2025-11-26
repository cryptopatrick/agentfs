//! PostgreSQL Example
//!
//! This example demonstrates:
//! - PostgreSQL backend connection
//! - Multi-agent concurrent access
//! - Async operations at scale
//!
//! Prerequisites:
//! - PostgreSQL server running
//! - Database created: CREATE DATABASE agentfs_demo;
//!
//! Run with:
//! ```bash
//! export DATABASE_URL="postgres://user:password@localhost/agentfs_demo"
//! cargo run --example postgres --features postgres
//! ```

use agentfs::{AgentFS, FileSystem, KvStore, ToolRecorder};
use agentsql::SqlBackend;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== AgentFS PostgreSQL Example ===\n");

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/agentfs_demo".to_string());

    println!("1. Connecting to PostgreSQL...");
    println!("   URL: {}", database_url.replace(|c: char| c == ':' && c.is_ascii_digit(), ":****"));

    let backend = SqlBackend::postgres(database_url).await?;
    let agent_fs = AgentFS::new(Box::new(backend), "postgres-agent", "/agent").await?;
    println!("   ✓ Connected successfully\n");

    // Demonstrate concurrent operations
    println!("2. Concurrent File Operations:");

    // Spawn multiple tasks writing files concurrently
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let fs = agent_fs.fs.clone();
            tokio::spawn(async move {
                let dir = format!("/concurrent/agent-{}", i);
                fs.mkdir(&dir).await?;
                let content = format!("Output from concurrent agent {}", i).into_bytes();
                fs.write_file(&format!("{}/output.txt", dir), &content).await?;
                Ok::<_, agentfs::AgentFsError>(())
            })
        })
        .collect();

    // Wait for all writes to complete
    for (i, handle) in handles.into_iter().enumerate() {
        handle.await??;
        println!("   ✓ Agent {} completed write", i);
    }
    println!();

    // Read back the files
    println!("3. Reading Concurrent Writes:");
    for i in 0..5 {
        let path = format!("/concurrent/agent-{}/output.txt", i);
        let content = agent_fs.fs.read_file(&path).await?.unwrap();
        println!("   ✓ {}: {}", path, String::from_utf8_lossy(&content));
    }
    println!();

    // Demonstrate shared KV store across "agents"
    println!("4. Shared Key-Value Store:");

    agent_fs.kv.set("global:counter", b"0").await?;
    println!("   ✓ Initialized global counter");

    // Simulate multiple agents incrementing a counter
    for i in 1..=5 {
        let current = agent_fs.kv.get("global:counter").await?.unwrap();
        let count: i32 = String::from_utf8_lossy(&current).parse().unwrap_or(0);
        let new_count = count + 1;
        agent_fs.kv.set("global:counter", new_count.to_string().as_bytes()).await?;
        println!("   ✓ Agent {} incremented counter to {}", i, new_count);
    }

    let final_count = agent_fs.kv.get("global:counter").await?.unwrap();
    println!("   ✓ Final counter value: {}\n", String::from_utf8_lossy(&final_count));

    // Tool call tracking across sessions
    println!("5. Tool Call Auditing:");

    // Simulate multiple API calls
    for i in 0..3 {
        let id = agent_fs.tools.start(
            "external_api",
            Some(serde_json::json!({"request_id": i}))
        ).await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        if i % 3 != 2 {
            agent_fs.tools.success(id, Some(serde_json::json!({"status": "ok"}))).await?;
        } else {
            agent_fs.tools.error(id, "Rate limit exceeded").await?;
        }
    }

    let stats = agent_fs.tools.stats_for("external_api").await?.unwrap();
    println!("   API Statistics:");
    println!("   - Total calls: {}", stats.total_calls);
    println!("   - Success rate: {:.1}%", (stats.successful as f64 / stats.total_calls as f64) * 100.0);
    println!("   - Avg latency: {:.2} ms\n", stats.avg_duration_ms);

    // List all recent tool calls
    println!("6. Recent Tool Calls:");
    let recent = agent_fs.tools.list(Some(10)).await?;
    for call in recent.iter().take(5) {
        println!("   - {} ({:?}) - {} ms",
            call.name,
            call.status,
            call.duration_ms.unwrap_or(0)
        );
    }

    println!("\n=== Example Complete ===");
    println!("\nPostgreSQL backend demonstrates:");
    println!("- Shared state across multiple agents");
    println!("- Concurrent file system operations");
    println!("- Centralized tool call auditing");
    println!("- Production-ready persistence");

    Ok(())
}
