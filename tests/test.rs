//! Integration tests for AgentFS
//!
//! These tests verify that our implementation adheres to the Agent Filesystem
//! Specification (SPEC.md) and matches the behavior of the original agentfs-main.

use agentfs::{AgentFS, FileSystem, KvStore};
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
