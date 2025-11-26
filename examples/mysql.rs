//! MySQL Example
//!
//! This example demonstrates:
//! - MySQL backend connection
//! - Cloud deployment patterns
//! - Production configuration
//!
//! Prerequisites:
//! - MySQL server running
//! - Database created: CREATE DATABASE agentfs_demo;
//!
//! Run with:
//! ```bash
//! export DATABASE_URL="mysql://user:password@localhost/agentfs_demo"
//! cargo run --example mysql --features mysql
//! ```

use agentfs::{AgentFS, FileSystem, KvStore, ToolRecorder};
use agentsql::SqlBackend;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== AgentFS MySQL Example ===\n");

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:password@localhost/agentfs_demo".to_string());

    println!("1. Connecting to MySQL...");
    println!("   URL: {}", mask_password(&database_url));

    let backend = SqlBackend::mysql(database_url).await?;
    let agent_fs = AgentFS::new(Box::new(backend), "mysql-agent", "/agent").await?;
    println!("   ✓ Connected successfully\n");

    // Demonstrate typical cloud agent workflow
    println!("2. Cloud Agent Workflow:");

    // Create workspace structure
    agent_fs.fs.mkdir("/workspace").await?;
    agent_fs.fs.mkdir("/workspace/data").await?;
    agent_fs.fs.mkdir("/workspace/results").await?;
    println!("   ✓ Created workspace structure");

    // Write input data
    let input_data = serde_json::json!({
        "task": "process_documents",
        "documents": [
            {"id": 1, "name": "doc1.pdf"},
            {"id": 2, "name": "doc2.pdf"}
        ]
    });
    agent_fs.fs.write_file(
        "/workspace/data/input.json",
        serde_json::to_string_pretty(&input_data)?.as_bytes()
    ).await?;
    println!("   ✓ Wrote input data");

    // Store processing state in KV
    agent_fs.kv.set("job:status", b"processing").await?;
    agent_fs.kv.set("job:progress", b"0").await?;
    println!("   ✓ Initialized job state\n");

    // Simulate document processing with tool calls
    println!("3. Processing Documents:");

    for i in 1..=2 {
        // Start processing
        let call_id = agent_fs.tools.start(
            "process_document",
            Some(serde_json::json!({"doc_id": i}))
        ).await?;
        println!("   ✓ Started processing document {}", i);

        // Simulate work
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Update progress
        agent_fs.kv.set("job:progress", format!("{}", (i * 50)).as_bytes()).await?;

        // Complete processing
        agent_fs.tools.success(
            call_id,
            Some(serde_json::json!({"pages": 10, "words": 5000}))
        ).await?;
        println!("   ✓ Completed document {} processing", i);
    }
    println!();

    // Write results
    println!("4. Generating Results:");

    let results = serde_json::json!({
        "status": "completed",
        "documents_processed": 2,
        "total_pages": 20,
        "total_words": 10000
    });

    agent_fs.fs.write_file(
        "/workspace/results/summary.json",
        serde_json::to_string_pretty(&results)?.as_bytes()
    ).await?;
    println!("   ✓ Wrote results summary");

    // Update job status
    agent_fs.kv.set("job:status", b"completed").await?;
    agent_fs.kv.set("job:progress", b"100").await?;
    println!("   ✓ Updated job status\n");

    // Show final state
    println!("5. Final State:");

    let status = agent_fs.kv.get("job:status").await?.unwrap();
    let progress = agent_fs.kv.get("job:progress").await?.unwrap();
    println!("   Status: {}", String::from_utf8_lossy(&status));
    println!("   Progress: {}%", String::from_utf8_lossy(&progress));

    // Tool call statistics
    let stats = agent_fs.tools.stats_for("process_document").await?.unwrap();
    println!("\n6. Processing Statistics:");
    println!("   Documents processed: {}", stats.total_calls);
    println!("   Success rate: 100%");
    println!("   Avg processing time: {:.2} ms", stats.avg_duration_ms);

    // List workspace contents
    println!("\n7. Workspace Contents:");
    for dir in &["/workspace/data", "/workspace/results"] {
        let entries = agent_fs.fs.readdir(dir).await?.unwrap();
        println!("   {}:", dir);
        for entry in entries {
            println!("     - {}", entry);
        }
    }

    println!("\n=== Example Complete ===");
    println!("\nMySQL backend is ideal for:");
    println!("- Cloud deployments (AWS RDS, Google Cloud SQL, Azure Database)");
    println!("- High availability with replication");
    println!("- Managed database services");
    println!("- Enterprise environments");

    Ok(())
}

/// Mask password in database URL for display
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let mut masked = url.to_string();
            let start = colon_pos + 1;
            let end = at_pos;
            if start < end {
                masked.replace_range(start..end, "****");
            }
            return masked;
        }
    }
    url.to_string()
}
