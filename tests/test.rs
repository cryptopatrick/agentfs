//! Integration tests for AgentFS
//!
//! These tests verify that our implementation adheres to the Agent Filesystem
//! Specification (SPEC.md) and matches the behavior of the original agentfs-main.

use agentfs::{AgentFS, FileSystem, KvStore, ToolRecorder};
use agentsql::SqlBackend;

/// Helper to create an in-memory SQLite AgentFS instance for testing
async fn create_test_agentfs() -> AgentFS {
    let backend = SqlBackend::sqlite(":memory:")
        .await
        .expect("Failed to create SQLite backend");

    AgentFS::new(Box::new(backend), "test-agent", "/agent")
        .await
        .expect("Failed to create AgentFS")
}

#[tokio::test]
async fn test_agentfs_creation() {
    let agentfs = create_test_agentfs().await;
    assert_eq!(agentfs.agent_id(), "test-agent");
}

#[tokio::test]
async fn test_kv_operations() {
    let agentfs = create_test_agentfs().await;

    // Set a value
    agentfs.kv.set("test_key", b"test_value").await.unwrap();

    // Get the value
    let value = agentfs.kv.get("test_key").await.unwrap();
    assert_eq!(value, Some(b"test_value".to_vec()));

    // Check existence
    assert!(agentfs.kv.exists("test_key").await.unwrap());
    assert!(!agentfs.kv.exists("nonexistent").await.unwrap());

    // Delete the value
    agentfs.kv.delete("test_key").await.unwrap();

    // Verify deletion
    let value = agentfs.kv.get("test_key").await.unwrap();
    assert_eq!(value, None);
}

#[tokio::test]
async fn test_kv_scan() {
    let agentfs = create_test_agentfs().await;

    // Set multiple values with prefixes
    agentfs.kv.set("prefix_1", b"v1").await.unwrap();
    agentfs.kv.set("prefix_2", b"v2").await.unwrap();
    agentfs.kv.set("other_key", b"v3").await.unwrap();

    // Scan for prefix
    let keys = agentfs.kv.scan("prefix").await.unwrap();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"prefix_1".to_string()));
    assert!(keys.contains(&"prefix_2".to_string()));
}

#[tokio::test]
async fn test_filesystem_mkdir() {
    let agentfs = create_test_agentfs().await;

    // Create a directory
    agentfs.fs.mkdir("/test_dir").await.unwrap();

    // Check directory exists
    assert!(agentfs.fs.exists("/test_dir").await.unwrap());

    // Get stats
    let stats = agentfs.fs.stat("/test_dir").await.unwrap();
    assert!(stats.is_some());
    let stats = stats.unwrap();
    assert!(stats.is_directory());
    assert_eq!(stats.ino, 2); // Root is 1, first created dir is 2
}

#[tokio::test]
async fn test_filesystem_write_and_read() {
    let agentfs = create_test_agentfs().await;

    // Create parent directory first
    agentfs.fs.mkdir("/test_dir").await.unwrap();

    // Write a file
    let data = b"Hello, AgentFS!";
    agentfs
        .fs
        .write_file("/test_dir/test.txt", data)
        .await
        .unwrap();

    // Read the file
    let read_data = agentfs
        .fs
        .read_file("/test_dir/test.txt")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(read_data, data);

    // Check file exists
    assert!(agentfs.fs.exists("/test_dir/test.txt").await.unwrap());
}

#[tokio::test]
async fn test_filesystem_readdir() {
    let agentfs = create_test_agentfs().await;

    // Create a directory
    agentfs.fs.mkdir("/test_dir").await.unwrap();

    // Write multiple files
    agentfs
        .fs
        .write_file("/test_dir/file1.txt", b"content1")
        .await
        .unwrap();
    agentfs
        .fs
        .write_file("/test_dir/file2.txt", b"content2")
        .await
        .unwrap();

    // Create a subdirectory
    agentfs.fs.mkdir("/test_dir/subdir").await.unwrap();

    // List directory
    let entries = agentfs.fs.readdir("/test_dir").await.unwrap().unwrap();
    assert_eq!(entries.len(), 3);
    assert!(entries.contains(&"file1.txt".to_string()));
    assert!(entries.contains(&"file2.txt".to_string()));
    assert!(entries.contains(&"subdir".to_string()));
}

#[tokio::test]
async fn test_filesystem_stat() {
    let agentfs = create_test_agentfs().await;

    // Create directory
    agentfs.fs.mkdir("/test_dir").await.unwrap();

    // Write a file
    let data = b"Test content";
    agentfs
        .fs
        .write_file("/test_dir/test.txt", data)
        .await
        .unwrap();

    // Get file stats
    let stats = agentfs.fs.stat("/test_dir/test.txt").await.unwrap();
    assert!(stats.is_some());
    let stats = stats.unwrap();
    assert!(stats.is_file());
    assert_eq!(stats.size, data.len() as i64);
    assert_eq!(stats.nlink, 1); // Single link

    // Get directory stats
    let dir_stats = agentfs.fs.stat("/test_dir").await.unwrap().unwrap();
    assert!(dir_stats.is_directory());
}

#[tokio::test]
async fn test_filesystem_remove() {
    let agentfs = create_test_agentfs().await;

    // Create and remove a file
    agentfs.fs.mkdir("/test_dir").await.unwrap();
    agentfs
        .fs
        .write_file("/test_dir/test.txt", b"content")
        .await
        .unwrap();

    assert!(agentfs.fs.exists("/test_dir/test.txt").await.unwrap());

    agentfs.fs.remove("/test_dir/test.txt").await.unwrap();

    assert!(!agentfs.fs.exists("/test_dir/test.txt").await.unwrap());

    // Remove empty directory
    agentfs.fs.remove("/test_dir").await.unwrap();
    assert!(!agentfs.fs.exists("/test_dir").await.unwrap());
}

#[tokio::test]
async fn test_filesystem_remove_non_empty_directory() {
    let agentfs = create_test_agentfs().await;

    // Create directory with a file
    agentfs.fs.mkdir("/test_dir").await.unwrap();
    agentfs
        .fs
        .write_file("/test_dir/test.txt", b"content")
        .await
        .unwrap();

    // Try to remove non-empty directory (should fail)
    let result = agentfs.fs.remove("/test_dir").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_filesystem_overwrite() {
    let agentfs = create_test_agentfs().await;

    agentfs.fs.mkdir("/test_dir").await.unwrap();

    // Write initial content
    agentfs
        .fs
        .write_file("/test_dir/test.txt", b"original")
        .await
        .unwrap();

    // Overwrite with new content
    agentfs
        .fs
        .write_file("/test_dir/test.txt", b"updated")
        .await
        .unwrap();

    // Read and verify new content
    let data = agentfs
        .fs
        .read_file("/test_dir/test.txt")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(data, b"updated");
}

#[tokio::test]
async fn test_filesystem_symlink() {
    let agentfs = create_test_agentfs().await;

    // Create a file
    agentfs.fs.mkdir("/test_dir").await.unwrap();
    agentfs
        .fs
        .write_file("/test_dir/original.txt", b"content")
        .await
        .unwrap();

    // Create a symlink
    agentfs
        .fs
        .symlink("/test_dir/original.txt", "/test_dir/link.txt")
        .await
        .unwrap();

    // Verify symlink exists
    assert!(agentfs.fs.exists("/test_dir/link.txt").await.unwrap());

    // Check lstat (should show it's a symlink)
    let lstat = agentfs.fs.lstat("/test_dir/link.txt").await.unwrap().unwrap();
    assert!(lstat.is_symlink());

    // Check stat (should follow symlink to file)
    let stat = agentfs.fs.stat("/test_dir/link.txt").await.unwrap().unwrap();
    assert!(stat.is_file());

    // Read symlink target
    let target = agentfs
        .fs
        .readlink("/test_dir/link.txt")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(target, "/test_dir/original.txt");

    // Read through symlink
    let data = agentfs
        .fs
        .read_file("/test_dir/link.txt")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(data, b"content");
}

#[tokio::test]
async fn test_filesystem_path_normalization() {
    let agentfs = create_test_agentfs().await;

    // Create directory with various path formats
    agentfs.fs.mkdir("/test_dir").await.unwrap();
    agentfs
        .fs
        .write_file("/test_dir/file.txt", b"content")
        .await
        .unwrap();

    // All these should resolve to the same file
    assert!(agentfs.fs.exists("/test_dir/file.txt").await.unwrap());
    assert!(agentfs.fs.exists("/test_dir//file.txt").await.unwrap());
    assert!(agentfs.fs.exists("/test_dir/./file.txt").await.unwrap());
}

#[tokio::test]
async fn test_root_directory_operations() {
    let agentfs = create_test_agentfs().await;

    // Root should exist
    assert!(agentfs.fs.exists("/").await.unwrap());

    // Root should be a directory
    let stats = agentfs.fs.stat("/").await.unwrap().unwrap();
    assert!(stats.is_directory());
    assert_eq!(stats.ino, 1); // Root inode is always 1

    // Can't remove root
    let result = agentfs.fs.remove("/").await;
    assert!(result.is_err());

    // Can list root
    let entries = agentfs.fs.readdir("/").await.unwrap().unwrap();
    assert!(entries.is_empty()); // Initially empty
}

#[tokio::test]
async fn test_inode_reuse_after_delete() {
    let agentfs = create_test_agentfs().await;

    // Create and delete a file
    agentfs.fs.mkdir("/test").await.unwrap();
    agentfs
        .fs
        .write_file("/test/file.txt", b"content")
        .await
        .unwrap();

    let original_stats = agentfs.fs.stat("/test/file.txt").await.unwrap().unwrap();
    let original_ino = original_stats.ino;

    agentfs.fs.remove("/test/file.txt").await.unwrap();

    // Create a new file - should get a different inode
    agentfs
        .fs
        .write_file("/test/file2.txt", b"new content")
        .await
        .unwrap();

    let new_stats = agentfs.fs.stat("/test/file2.txt").await.unwrap().unwrap();
    assert_ne!(new_stats.ino, original_ino);
}

#[tokio::test]
async fn test_empty_file() {
    let agentfs = create_test_agentfs().await;

    agentfs.fs.mkdir("/test").await.unwrap();

    // Write empty file
    agentfs
        .fs
        .write_file("/test/empty.txt", b"")
        .await
        .unwrap();

    // Read empty file
    let data = agentfs
        .fs
        .read_file("/test/empty.txt")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(data.len(), 0);

    // Check stats
    let stats = agentfs.fs.stat("/test/empty.txt").await.unwrap().unwrap();
    assert_eq!(stats.size, 0);
}

#[tokio::test]
async fn test_nested_directories() {
    let agentfs = create_test_agentfs().await;

    // Create nested directory structure
    agentfs.fs.mkdir("/level1").await.unwrap();
    agentfs.fs.mkdir("/level1/level2").await.unwrap();
    agentfs.fs.mkdir("/level1/level2/level3").await.unwrap();

    // Write file at deepest level
    agentfs
        .fs
        .write_file("/level1/level2/level3/deep.txt", b"deep content")
        .await
        .unwrap();

    // Verify we can read it
    let data = agentfs
        .fs
        .read_file("/level1/level2/level3/deep.txt")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(data, b"deep content");

    // Verify path resolution works at each level
    assert!(agentfs.fs.exists("/level1").await.unwrap());
    assert!(agentfs.fs.exists("/level1/level2").await.unwrap());
    assert!(agentfs.fs.exists("/level1/level2/level3").await.unwrap());
}

#[tokio::test]
async fn test_file_in_root() {
    let agentfs = create_test_agentfs().await;

    // Write file directly in root
    agentfs
        .fs
        .write_file("/root_file.txt", b"in root")
        .await
        .unwrap();

    // Read it back
    let data = agentfs
        .fs
        .read_file("/root_file.txt")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(data, b"in root");

    // List root to see it
    let entries = agentfs.fs.readdir("/").await.unwrap().unwrap();
    assert!(entries.contains(&"root_file.txt".to_string()));
}

// Tool Calls API Tests

#[tokio::test]
async fn test_tool_calls_workflow() {
    let agentfs = create_test_agentfs().await;

    // Start a tool call
    let params = serde_json::json!({"query": "test query"});
    let id = agentfs.tools.start("web_search", Some(params.clone())).await.unwrap();

    // Verify tool call was created
    let tool_call = agentfs.tools.get(id).await.unwrap().unwrap();
    assert_eq!(tool_call.name, "web_search");
    assert_eq!(tool_call.parameters, Some(params));
    assert_eq!(tool_call.status, agentfs::tools::ToolCallStatus::Pending);
    assert!(tool_call.completed_at.is_none());
    assert!(tool_call.duration_ms.is_none());

    // Mark as successful
    let result = serde_json::json!({"results": ["result1", "result2"]});
    agentfs.tools.success(id, Some(result.clone())).await.unwrap();

    // Verify tool call was updated
    let tool_call = agentfs.tools.get(id).await.unwrap().unwrap();
    assert_eq!(tool_call.status, agentfs::tools::ToolCallStatus::Success);
    assert_eq!(tool_call.result, Some(result));
    assert!(tool_call.completed_at.is_some());
    assert!(tool_call.duration_ms.is_some());
    assert!(tool_call.duration_ms.unwrap() >= 0);
}

#[tokio::test]
async fn test_tool_calls_error() {
    let agentfs = create_test_agentfs().await;

    // Start a tool call
    let id = agentfs.tools.start("failing_tool", None).await.unwrap();

    // Mark as failed
    agentfs.tools.error(id, "Connection timeout").await.unwrap();

    // Verify error was recorded
    let tool_call = agentfs.tools.get(id).await.unwrap().unwrap();
    assert_eq!(tool_call.status, agentfs::tools::ToolCallStatus::Error);
    assert_eq!(tool_call.error, Some("Connection timeout".to_string()));
    assert!(tool_call.result.is_none());
    assert!(tool_call.completed_at.is_some());
    assert!(tool_call.duration_ms.is_some());
}

#[tokio::test]
async fn test_tool_calls_record() {
    let agentfs = create_test_agentfs().await;

    // Use single-shot record API
    let params = serde_json::json!({"url": "https://example.com"});
    let result = serde_json::json!({"status": 200});

    let started_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let completed_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let id = agentfs.tools.record(
        "http_request",
        started_at,
        completed_at,
        Some(params.clone()),
        Some(result.clone()),
        None,
    ).await.unwrap();

    // Verify tool call
    let tool_call = agentfs.tools.get(id).await.unwrap().unwrap();
    assert_eq!(tool_call.name, "http_request");
    assert_eq!(tool_call.parameters, Some(params));
    assert_eq!(tool_call.result, Some(result));
    assert_eq!(tool_call.status, agentfs::tools::ToolCallStatus::Success);
    assert_eq!(tool_call.started_at, started_at);
    assert_eq!(tool_call.completed_at, Some(completed_at));
}

#[tokio::test]
async fn test_tool_calls_stats() {
    let agentfs = create_test_agentfs().await;

    // Create multiple tool calls
    let id1 = agentfs.tools.start("api_call", None).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    agentfs.tools.success(id1, None).await.unwrap();

    let id2 = agentfs.tools.start("api_call", None).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    agentfs.tools.success(id2, None).await.unwrap();

    let id3 = agentfs.tools.start("api_call", None).await.unwrap();
    agentfs.tools.error(id3, "Failed").await.unwrap();

    // Get statistics
    let stats = agentfs.tools.stats_for("api_call").await.unwrap().unwrap();
    assert_eq!(stats.name, "api_call");
    assert_eq!(stats.total_calls, 3);
    assert_eq!(stats.successful, 2);
    assert_eq!(stats.failed, 1);
    assert!(stats.avg_duration_ms >= 0.0);

    // Non-existent tool should return None
    let no_stats = agentfs.tools.stats_for("nonexistent").await.unwrap();
    assert!(no_stats.is_none());
}

#[tokio::test]
async fn test_tool_calls_list() {
    let agentfs = create_test_agentfs().await;

    // Create several tool calls
    agentfs.tools.start("tool1", None).await.unwrap();
    agentfs.tools.start("tool2", None).await.unwrap();
    agentfs.tools.start("tool3", None).await.unwrap();

    // List all
    let all_calls = agentfs.tools.list(None).await.unwrap();
    assert_eq!(all_calls.len(), 3);

    // List with limit
    let limited_calls = agentfs.tools.list(Some(2)).await.unwrap();
    assert_eq!(limited_calls.len(), 2);
}

#[tokio::test]
async fn test_path_sandboxing() {
    let agentfs = create_test_agentfs().await;

    // Test 1: Basic paths are sandboxed within mount point
    agentfs.fs.write_file("/test.txt", b"content").await.unwrap();
    let content = agentfs.fs.read_file("/test.txt").await.unwrap();
    assert_eq!(content, Some(b"content".to_vec()));

    // Test 2: Relative paths work
    agentfs.fs.write_file("relative.txt", b"relative").await.unwrap();
    let content = agentfs.fs.read_file("relative.txt").await.unwrap();
    assert_eq!(content, Some(b"relative".to_vec()));

    // Test 3: Paths with mount point prefix are properly stripped
    agentfs.fs.write_file("/agent/prefixed.txt", b"prefixed").await.unwrap();
    let content = agentfs.fs.read_file("/prefixed.txt").await.unwrap();
    assert_eq!(content, Some(b"prefixed".to_vec()));
    let content2 = agentfs.fs.read_file("/agent/prefixed.txt").await.unwrap();
    assert_eq!(content2, Some(b"prefixed".to_vec()));

    // Test 4: Directory traversal attempts are normalized and can't escape
    agentfs.fs.mkdir("/sandbox").await.unwrap();
    agentfs.fs.write_file("/sandbox/file.txt", b"safe").await.unwrap();

    // Attempting to traverse up with .. just normalizes to root
    agentfs.fs.write_file("/sandbox/../escape_attempt.txt", b"normalized").await.unwrap();
    // This should create the file at /escape_attempt.txt (not outside mount point)
    let content = agentfs.fs.read_file("/escape_attempt.txt").await.unwrap();
    assert_eq!(content, Some(b"normalized".to_vec()));

    // Test 5: Multiple .. components can't escape root
    // First create the directory that will result from normalization
    agentfs.fs.mkdir("/etc").await.unwrap();
    agentfs.fs.write_file("/../../../etc/passwd", b"blocked").await.unwrap();
    // This normalizes to /etc/passwd within the mount point
    let content = agentfs.fs.read_file("/etc/passwd").await.unwrap();
    assert_eq!(content, Some(b"blocked".to_vec()));

    // Test 6: Verify filesystem operations all respect sandboxing
    agentfs.fs.mkdir("/agent/sandboxed").await.unwrap();
    agentfs.fs.write_file("/agent/sandboxed/test.txt", b"data").await.unwrap();
    assert!(agentfs.fs.exists("/sandboxed/test.txt").await.unwrap());
    assert!(agentfs.fs.exists("/agent/sandboxed/test.txt").await.unwrap());

    let entries = agentfs.fs.readdir("/").await.unwrap().unwrap();
    // Should have: test.txt, relative.txt, prefixed.txt, sandbox/, escape_attempt.txt, etc/, sandboxed/
    assert!(entries.len() >= 6);

    // Test 7: Stat operations work with sandboxed paths
    let stats = agentfs.fs.stat("/test.txt").await.unwrap();
    assert!(stats.is_some());
    let stats = agentfs.fs.stat("/agent/test.txt").await.unwrap();
    assert!(stats.is_some());
}
