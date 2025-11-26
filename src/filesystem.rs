//! Filesystem operations for AgentFS
//!
//! Based on the Agent Filesystem Specification (SPEC.md).
//! Uses inode/dentry design for Unix-like filesystem semantics.

use crate::error::{AgentFsError, Result};
use agentdb::AgentDB;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// File type constants for mode field
pub const S_IFMT: u32 = 0o170000;   // File type mask
pub const S_IFREG: u32 = 0o100000;  // Regular file
pub const S_IFDIR: u32 = 0o040000;  // Directory
pub const S_IFLNK: u32 = 0o120000;  // Symbolic link

// Default permissions
pub const DEFAULT_FILE_MODE: u32 = S_IFREG | 0o644; // Regular file, rw-r--r--
pub const DEFAULT_DIR_MODE: u32 = S_IFDIR | 0o755;  // Directory, rwxr-xr-x

pub const ROOT_INO: i64 = 1;

/// File statistics
#[derive(Debug, Clone)]
pub struct Stats {
    pub ino: i64,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: i64,
    pub atime: i64,
    pub mtime: i64,
    pub ctime: i64,
}

impl Stats {
    pub fn is_file(&self) -> bool {
        (self.mode & S_IFMT) == S_IFREG
    }

    pub fn is_directory(&self) -> bool {
        (self.mode & S_IFMT) == S_IFDIR
    }

    pub fn is_symlink(&self) -> bool {
        (self.mode & S_IFMT) == S_IFLNK
    }
}

/// Filesystem trait for agent file operations
///
/// Provides POSIX-like file and directory operations backed by a database.
#[async_trait]
pub trait FileSystem: Send + Sync {
    /// Write content to a file
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<()>;

    /// Read content from a file
    async fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>>;

    /// Check if a path exists (file or directory)
    async fn exists(&self, path: &str) -> Result<bool>;

    /// List contents of a directory
    async fn readdir(&self, path: &str) -> Result<Option<Vec<String>>>;

    /// Create a directory
    async fn mkdir(&self, path: &str) -> Result<()>;

    /// Remove a file or empty directory
    async fn remove(&self, path: &str) -> Result<()>;

    /// Get file statistics (following symlinks)
    async fn stat(&self, path: &str) -> Result<Option<Stats>>;

    /// Get file statistics (not following symlinks)
    async fn lstat(&self, path: &str) -> Result<Option<Stats>>;

    /// Create a symbolic link
    async fn symlink(&self, target: &str, linkpath: &str) -> Result<()>;

    /// Read the target of a symbolic link
    async fn readlink(&self, path: &str) -> Result<Option<String>>;
}

/// Database-backed filesystem implementation
#[derive(Clone)]
pub struct DbFileSystem {
    db: Arc<Box<dyn AgentDB>>,
    mount_path: String,
}

impl DbFileSystem {
    /// Create a new database-backed filesystem
    pub fn new(db: Arc<Box<dyn AgentDB>>, mount_path: String) -> Self {
        Self { db, mount_path }
    }

    /// Normalize a path
    fn normalize_path(&self, path: &str) -> String {
        let normalized = path.trim_end_matches('/');
        let normalized = if normalized.is_empty() {
            "/"
        } else if normalized.starts_with('/') {
            normalized
        } else {
            return format!("/{}", normalized);
        };

        // Handle . and .. components
        let components: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        let mut result = Vec::new();

        for component in components {
            match component {
                "." => continue,
                ".." => {
                    if !result.is_empty() {
                        result.pop();
                    }
                }
                _ => result.push(component),
            }
        }

        if result.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", result.join("/"))
        }
    }

    /// Validate and normalize a path, ensuring it's within the mount point
    ///
    /// This enforces path sandboxing by:
    /// 1. Treating all paths as relative to the mount point
    /// 2. Normalizing the path (resolving . and ..)
    /// 3. Ensuring no path traversal escapes the mount point
    ///
    /// # Security
    ///
    /// This prevents directory traversal attacks by ensuring all paths
    /// are treated as relative to the mount point (e.g., /agent), even if
    /// they start with /. Attempts to traverse outside the mount point
    /// (e.g., /../../../etc/passwd) are prevented by normalization.
    ///
    /// # Path Interpretation
    ///
    /// All paths are treated as relative to the mount point:
    /// - "/foo" -> internal path "/foo" (within mount point)
    /// - "foo" -> internal path "/foo" (relative converted to absolute)
    /// - "/agent/foo" -> internal path "/foo" (mount prefix stripped if present)
    /// - "/../etc" -> internal path "/" (normalized, can't escape mount point)
    ///
    /// # Example
    ///
    /// With mount_path = "/agent":
    /// - "/foo" -> Ok("/foo")
    /// - "/agent/foo" -> Ok("/foo")
    /// - "foo" -> Ok("/foo")
    /// - "/../../../etc/passwd" -> Ok("/") (normalized, traversal prevented)
    fn validate_and_normalize_path(&self, path: &str) -> Result<String> {
        let mount_prefix = self.mount_path.trim_end_matches('/');

        // Check if path explicitly includes the mount point prefix
        let path_to_normalize = if path.starts_with(&format!("{}/", mount_prefix)) {
            // Path explicitly includes mount point, strip it
            &path[mount_prefix.len()..]
        } else if path == mount_prefix {
            // Path is exactly the mount point
            "/"
        } else {
            // Treat path as relative to mount point (even if it starts with /)
            path
        };

        // Normalize the path (this handles .. and . components)
        let normalized = self.normalize_path(path_to_normalize);

        // The normalized path is now the internal path, already secured
        // by the normalization process which prevents escaping the root
        Ok(normalized)
    }

    /// Split path into components
    fn split_path(&self, path: &str) -> Vec<String> {
        let normalized = self.normalize_path(path);
        if normalized == "/" {
            return vec![];
        }
        normalized
            .split('/')
            .filter(|p| !p.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// Get current Unix timestamp
    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    /// Get link count for an inode
    async fn get_link_count(&self, ino: i64) -> Result<u32> {
        let query = format!("SELECT COUNT(*) as count FROM fs_dentry WHERE ino = {}", ino);
        let result = self.db.query(&query, vec![]).await?;

        if let Some(row) = result.rows.first() {
            if let Some(count_val) = row.get("count") {
                let count_bytes = count_val.as_bytes();
                let count_str = String::from_utf8_lossy(count_bytes);
                return Ok(count_str.parse().unwrap_or(0));
            }
        }
        Ok(0)
    }

    /// Resolve a path to an inode number
    async fn resolve_path(&self, path: &str) -> Result<Option<i64>> {
        let components = self.split_path(path);
        if components.is_empty() {
            return Ok(Some(ROOT_INO));
        }

        let mut current_ino = ROOT_INO;
        for component in components {
            let query = format!(
                "SELECT ino FROM fs_dentry WHERE parent_ino = {} AND name = '{}'",
                current_ino,
                component.replace('\'', "''")
            );
            let result = self.db.query(&query, vec![]).await?;

            if let Some(row) = result.rows.first() {
                if let Some(ino_val) = row.get("ino") {
                    let ino_bytes = ino_val.as_bytes();
                    let ino_str = String::from_utf8_lossy(ino_bytes);
                    current_ino = ino_str.parse().unwrap_or(0);
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        }

        Ok(Some(current_ino))
    }

    /// Build stats from query result
    async fn build_stats(&self, ino: i64, mode: u32, uid: u32, gid: u32, size: i64, atime: i64, mtime: i64, ctime: i64) -> Result<Stats> {
        let nlink = self.get_link_count(ino).await?;
        Ok(Stats {
            ino,
            mode,
            nlink,
            uid,
            gid,
            size,
            atime,
            mtime,
            ctime,
        })
    }
}

#[async_trait]
impl FileSystem for DbFileSystem {
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        let path = self.validate_and_normalize_path(path)?;
        let components = self.split_path(&path);

        if components.is_empty() {
            return Err(AgentFsError::InvalidPath("Cannot write to root directory".to_string()));
        }

        let parent_path = if components.len() == 1 {
            "/".to_string()
        } else {
            format!("/{}", components[..components.len() - 1].join("/"))
        };

        let parent_ino = self
            .resolve_path(&parent_path)
            .await?
            .ok_or_else(|| AgentFsError::DirectoryNotFound(parent_path.clone()))?;

        let name = components.last().unwrap();

        // Check if file exists
        let ino = if let Some(ino) = self.resolve_path(&path).await? {
            // Delete existing data chunks
            let query = format!("DELETE FROM fs_data WHERE ino = {}", ino);
            self.db.query(&query, vec![]).await?;
            ino
        } else {
            // Create new inode
            let now = Self::now();
            let query = format!(
                "INSERT INTO fs_inode (mode, uid, gid, size, atime, mtime, ctime) VALUES ({}, 0, 0, {}, {}, {}, {})",
                DEFAULT_FILE_MODE, content.len(), now, now, now
            );
            self.db.query(&query, vec![]).await?;

            // Get the new inode number
            let query = "SELECT last_insert_rowid() as ino".to_string();
            let result = self.db.query(&query, vec![]).await?;
            let ino = if let Some(row) = result.rows.first() {
                if let Some(ino_val) = row.get("ino") {
                    let ino_str = String::from_utf8_lossy(ino_val.as_bytes());
                    ino_str.parse().unwrap_or(0)
                } else {
                    return Err(AgentFsError::Database(agentdb::AgentDbError::Backend("Failed to get inode".to_string())));
                }
            } else {
                return Err(AgentFsError::Database(agentdb::AgentDbError::Backend("Failed to get inode".to_string())));
            };

            // Create directory entry
            let query = format!(
                "INSERT INTO fs_dentry (name, parent_ino, ino) VALUES ('{}', {}, {})",
                name.replace('\'', "''"),
                parent_ino,
                ino
            );
            self.db.query(&query, vec![]).await?;

            ino
        };

        // Write data chunk
        if !content.is_empty() {
            // Store data as a KV entry temporarily (workaround for BLOB binding issue)
            let data_key = format!("__fs_data:{}:0", ino);
            self.db.put(&data_key, content.into()).await?;

            // TODO: Use proper BLOB insertion once we have parameterized queries
            // For now we'll need to retrieve and insert via a workaround
        }

        // Update size and mtime
        let now = Self::now();
        let query = format!(
            "UPDATE fs_inode SET size = {}, mtime = {} WHERE ino = {}",
            content.len(),
            now,
            ino
        );
        self.db.query(&query, vec![]).await?;

        Ok(())
    }

    async fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>> {
        // Follow symlinks to get the final inode
        let path = self.validate_and_normalize_path(path)?;
        let mut current_path = path.clone();
        let max_symlink_depth = 40;

        let ino = 'resolve: loop {
            for _ in 0..max_symlink_depth {
                let ino = match self.resolve_path(&current_path).await? {
                    Some(ino) => ino,
                    None => return Ok(None),
                };

                // Check if it's a symlink
                let query = format!(
                    "SELECT mode FROM fs_inode WHERE ino = {}",
                    ino
                );
                let result = self.db.query(&query, vec![]).await?;

                if let Some(row) = result.rows.first() {
                    let mode = self.extract_u32(row, "mode")?;

                    if (mode & S_IFMT) == S_IFLNK {
                        // It's a symlink, follow it
                        let target = self.readlink(&current_path).await?
                            .ok_or_else(|| AgentFsError::InvalidPath("Symlink has no target".to_string()))?;

                        // Resolve target path
                        current_path = if target.starts_with('/') {
                            target
                        } else {
                            let base = Path::new(&current_path);
                            let parent = base.parent().unwrap_or(Path::new("/"));
                            let joined = parent.join(&target);
                            self.normalize_path(&joined.to_string_lossy())
                        };
                        continue;
                    }

                    // Not a symlink, use this inode
                    break 'resolve ino;
                } else {
                    return Ok(None);
                }
            }

            return Err(AgentFsError::InvalidPath("Too many levels of symbolic links".to_string()));
        };

        // Read data chunks
        // Temporary workaround using KV store
        let data_key = format!("__fs_data:{}:0", ino);
        if let Some(value) = self.db.get(&data_key).await? {
            return Ok(Some(value.as_bytes().to_vec()));
        }

        // If no data in KV, try fs_data table
        let query = format!("SELECT data FROM fs_data WHERE ino = {} ORDER BY offset", ino);
        let result = self.db.query(&query, vec![]).await?;

        if result.rows.is_empty() {
            return Ok(Some(Vec::new())); // Empty file
        }

        let mut data = Vec::new();
        for row in &result.rows {
            if let Some(chunk) = row.get("data") {
                data.extend_from_slice(chunk.as_bytes());
            }
        }

        Ok(Some(data))
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let path = self.validate_and_normalize_path(path)?;
        Ok(self.resolve_path(&path).await?.is_some())
    }

    async fn readdir(&self, path: &str) -> Result<Option<Vec<String>>> {
        let path = self.validate_and_normalize_path(path)?;
        let ino = match self.resolve_path(&path).await? {
            Some(ino) => ino,
            None => return Ok(None),
        };

        let query = format!(
            "SELECT name FROM fs_dentry WHERE parent_ino = {} ORDER BY name",
            ino
        );
        let result = self.db.query(&query, vec![]).await?;

        let mut entries = Vec::new();
        for row in &result.rows {
            if let Some(name_val) = row.get("name") {
                let name = String::from_utf8_lossy(name_val.as_bytes()).to_string();
                entries.push(name);
            }
        }

        Ok(Some(entries))
    }

    async fn mkdir(&self, path: &str) -> Result<()> {
        let path = self.validate_and_normalize_path(path)?;
        let components = self.split_path(&path);

        if components.is_empty() {
            return Err(AgentFsError::InvalidPath("Cannot create root directory".to_string()));
        }

        let parent_path = if components.len() == 1 {
            "/".to_string()
        } else {
            format!("/{}", components[..components.len() - 1].join("/"))
        };

        let parent_ino = self
            .resolve_path(&parent_path)
            .await?
            .ok_or_else(|| AgentFsError::DirectoryNotFound(parent_path))?;

        let name = components.last().unwrap();

        // Check if already exists
        if self.resolve_path(&path).await?.is_some() {
            return Err(AgentFsError::PathExists(path));
        }

        // Create inode
        let now = Self::now();
        let query = format!(
            "INSERT INTO fs_inode (mode, uid, gid, size, atime, mtime, ctime) VALUES ({}, 0, 0, 0, {}, {}, {})",
            DEFAULT_DIR_MODE, now, now, now
        );
        self.db.query(&query, vec![]).await?;

        // Get new inode number
        let query = "SELECT last_insert_rowid() as ino".to_string();
        let result = self.db.query(&query, vec![]).await?;
        let ino = if let Some(row) = result.rows.first() {
            if let Some(ino_val) = row.get("ino") {
                let ino_str = String::from_utf8_lossy(ino_val.as_bytes());
                ino_str.parse().unwrap_or(0)
            } else {
                return Err(AgentFsError::Database(agentdb::AgentDbError::Backend("Failed to get inode".to_string())));
            }
        } else {
            return Err(AgentFsError::Database(agentdb::AgentDbError::Backend("Failed to get inode".to_string())));
        };

        // Create directory entry
        let query = format!(
            "INSERT INTO fs_dentry (name, parent_ino, ino) VALUES ('{}', {}, {})",
            name.replace('\'', "''"),
            parent_ino,
            ino
        );
        self.db.query(&query, vec![]).await?;

        Ok(())
    }

    async fn remove(&self, path: &str) -> Result<()> {
        let path = self.validate_and_normalize_path(path)?;
        let components = self.split_path(&path);

        if components.is_empty() {
            return Err(AgentFsError::InvalidPath("Cannot remove root directory".to_string()));
        }

        let ino = self
            .resolve_path(&path)
            .await?
            .ok_or_else(|| AgentFsError::FileNotFound(path.clone()))?;

        if ino == ROOT_INO {
            return Err(AgentFsError::InvalidPath("Cannot remove root directory".to_string()));
        }

        // Check if directory is empty
        let query = format!("SELECT COUNT(*) as count FROM fs_dentry WHERE parent_ino = {}", ino);
        let result = self.db.query(&query, vec![]).await?;
        if let Some(row) = result.rows.first() {
            if let Some(count_val) = row.get("count") {
                let count_str = String::from_utf8_lossy(count_val.as_bytes());
                let count: i64 = count_str.parse().unwrap_or(0);
                if count > 0 {
                    return Err(AgentFsError::InvalidPath("Directory not empty".to_string()));
                }
            }
        }

        // Get parent directory and name
        let parent_path = if components.len() == 1 {
            "/".to_string()
        } else {
            format!("/{}", components[..components.len() - 1].join("/"))
        };

        let parent_ino = self
            .resolve_path(&parent_path)
            .await?
            .ok_or_else(|| AgentFsError::DirectoryNotFound(parent_path))?;

        let name = components.last().unwrap();

        // Delete the directory entry
        let query = format!(
            "DELETE FROM fs_dentry WHERE parent_ino = {} AND name = '{}'",
            parent_ino,
            name.replace('\'', "''")
        );
        self.db.query(&query, vec![]).await?;

        // Check if this was the last link
        let link_count = self.get_link_count(ino).await?;
        if link_count == 0 {
            // Delete data chunks
            let query = format!("DELETE FROM fs_data WHERE ino = {}", ino);
            self.db.query(&query, vec![]).await?;

            // Delete symlink if exists
            let query = format!("DELETE FROM fs_symlink WHERE ino = {}", ino);
            self.db.query(&query, vec![]).await?;

            // Delete inode
            let query = format!("DELETE FROM fs_inode WHERE ino = {}", ino);
            self.db.query(&query, vec![]).await?;

            // Clean up temp KV data
            let data_key = format!("__fs_data:{}:0", ino);
            let _ = self.db.delete(&data_key).await;
        }

        Ok(())
    }

    async fn stat(&self, path: &str) -> Result<Option<Stats>> {
        let path = self.validate_and_normalize_path(path)?;

        // Follow symlinks with a maximum depth
        let mut current_path = path;
        let max_symlink_depth = 40;

        for _ in 0..max_symlink_depth {
            let ino = match self.resolve_path(&current_path).await? {
                Some(ino) => ino,
                None => return Ok(None),
            };

            let query = format!(
                "SELECT ino, mode, uid, gid, size, atime, mtime, ctime FROM fs_inode WHERE ino = {}",
                ino
            );
            let result = self.db.query(&query, vec![]).await?;

            if let Some(row) = result.rows.first() {
                let mode = self.extract_u32(row, "mode")?;

                // Check if symlink
                if (mode & S_IFMT) == S_IFLNK {
                    // Read symlink target
                    let target = self.readlink(&current_path).await?
                        .ok_or_else(|| AgentFsError::InvalidPath("Symlink has no target".to_string()))?;

                    // Resolve target path
                    current_path = if target.starts_with('/') {
                        target
                    } else {
                        let base = Path::new(&current_path);
                        let parent = base.parent().unwrap_or(Path::new("/"));
                        let joined = parent.join(&target);
                        self.normalize_path(&joined.to_string_lossy())
                    };
                    continue;
                }

                // Not a symlink, return stats
                return Ok(Some(self.build_stats(
                    ino,
                    mode,
                    self.extract_u32(row, "uid")?,
                    self.extract_u32(row, "gid")?,
                    self.extract_i64(row, "size")?,
                    self.extract_i64(row, "atime")?,
                    self.extract_i64(row, "mtime")?,
                    self.extract_i64(row, "ctime")?,
                ).await?));
            } else {
                return Ok(None);
            }
        }

        Err(AgentFsError::InvalidPath("Too many levels of symbolic links".to_string()))
    }

    async fn lstat(&self, path: &str) -> Result<Option<Stats>> {
        let path = self.validate_and_normalize_path(path)?;
        let ino = match self.resolve_path(&path).await? {
            Some(ino) => ino,
            None => return Ok(None),
        };

        let query = format!(
            "SELECT ino, mode, uid, gid, size, atime, mtime, ctime FROM fs_inode WHERE ino = {}",
            ino
        );
        let result = self.db.query(&query, vec![]).await?;

        if let Some(row) = result.rows.first() {
            Ok(Some(self.build_stats(
                ino,
                self.extract_u32(row, "mode")?,
                self.extract_u32(row, "uid")?,
                self.extract_u32(row, "gid")?,
                self.extract_i64(row, "size")?,
                self.extract_i64(row, "atime")?,
                self.extract_i64(row, "mtime")?,
                self.extract_i64(row, "ctime")?,
            ).await?))
        } else {
            Ok(None)
        }
    }

    async fn symlink(&self, target: &str, linkpath: &str) -> Result<()> {
        let linkpath = self.validate_and_normalize_path(linkpath)?;
        let components = self.split_path(&linkpath);

        if components.is_empty() {
            return Err(AgentFsError::InvalidPath("Cannot create symlink at root".to_string()));
        }

        let parent_path = if components.len() == 1 {
            "/".to_string()
        } else {
            format!("/{}", components[..components.len() - 1].join("/"))
        };

        let parent_ino = self
            .resolve_path(&parent_path)
            .await?
            .ok_or_else(|| AgentFsError::DirectoryNotFound(parent_path))?;

        let name = components.last().unwrap();

        // Check if already exists
        if self.resolve_path(&linkpath).await?.is_some() {
            return Err(AgentFsError::PathExists(linkpath));
        }

        // Create inode for symlink
        let now = Self::now();
        let mode = S_IFLNK | 0o777;
        let size = target.len() as i64;

        let query = format!(
            "INSERT INTO fs_inode (mode, uid, gid, size, atime, mtime, ctime) VALUES ({}, 0, 0, {}, {}, {}, {})",
            mode, size, now, now, now
        );
        self.db.query(&query, vec![]).await?;

        // Get new inode
        let query = "SELECT last_insert_rowid() as ino".to_string();
        let result = self.db.query(&query, vec![]).await?;
        let ino = if let Some(row) = result.rows.first() {
            if let Some(ino_val) = row.get("ino") {
                let ino_str = String::from_utf8_lossy(ino_val.as_bytes());
                ino_str.parse().unwrap_or(0)
            } else {
                return Err(AgentFsError::Database(agentdb::AgentDbError::Backend("Failed to get inode".to_string())));
            }
        } else {
            return Err(AgentFsError::Database(agentdb::AgentDbError::Backend("Failed to get inode".to_string())));
        };

        // Store symlink target
        let query = format!(
            "INSERT INTO fs_symlink (ino, target) VALUES ({}, '{}')",
            ino,
            target.replace('\'', "''")
        );
        self.db.query(&query, vec![]).await?;

        // Create directory entry
        let query = format!(
            "INSERT INTO fs_dentry (name, parent_ino, ino) VALUES ('{}', {}, {})",
            name.replace('\'', "''"),
            parent_ino,
            ino
        );
        self.db.query(&query, vec![]).await?;

        Ok(())
    }

    async fn readlink(&self, path: &str) -> Result<Option<String>> {
        let path = self.validate_and_normalize_path(path)?;
        let ino = match self.resolve_path(&path).await? {
            Some(ino) => ino,
            None => return Ok(None),
        };

        // Check if it's a symlink
        let query = format!("SELECT mode FROM fs_inode WHERE ino = {}", ino);
        let result = self.db.query(&query, vec![]).await?;

        if let Some(row) = result.rows.first() {
            let mode = self.extract_u32(row, "mode")?;
            if (mode & S_IFMT) != S_IFLNK {
                return Err(AgentFsError::InvalidPath("Not a symbolic link".to_string()));
            }
        } else {
            return Ok(None);
        }

        // Read target from fs_symlink table
        let query = format!("SELECT target FROM fs_symlink WHERE ino = {}", ino);
        let result = self.db.query(&query, vec![]).await?;

        if let Some(row) = result.rows.first() {
            if let Some(target_val) = row.get("target") {
                let target = String::from_utf8_lossy(target_val.as_bytes()).to_string();
                return Ok(Some(target));
            }
        }

        Ok(None)
    }
}

impl DbFileSystem {
    /// Helper to extract i64 from row
    fn extract_i64(&self, row: &agentdb::Row, column: &str) -> Result<i64> {
        row.get(column)
            .ok_or_else(|| AgentFsError::Database(agentdb::AgentDbError::Backend(format!("Missing column: {}", column))))
            .and_then(|val| {
                let s = String::from_utf8_lossy(val.as_bytes());
                s.parse().map_err(|_| AgentFsError::Database(agentdb::AgentDbError::Backend(format!("Invalid i64 in column: {}", column))))
            })
    }

    /// Helper to extract u32 from row
    fn extract_u32(&self, row: &agentdb::Row, column: &str) -> Result<u32> {
        row.get(column)
            .ok_or_else(|| AgentFsError::Database(agentdb::AgentDbError::Backend(format!("Missing column: {}", column))))
            .and_then(|val| {
                let s = String::from_utf8_lossy(val.as_bytes());
                s.parse().map_err(|_| AgentFsError::Database(agentdb::AgentDbError::Backend(format!("Invalid u32 in column: {}", column))))
            })
    }
}
