<h1 align="center">
  <br>
  AGENTFS
  <br>
</h1>

<h4 align="center">
  Filesystem Abstraction for AI Agents
</h4>

<p align="center">
  <a href="https://crates.io/crates/agentfs" target="_blank">
    <img src="https://img.shields.io/crates/v/agentfs" alt="Crates.io"/>
  </a>
  <a href="https://crates.io/crates/agentfs" target="_blank">
    <img src="https://img.shields.io/crates/d/agentfs" alt="Downloads"/>
  </a>
  <a href="https://docs.rs/agentfs" target="_blank">
    <img src="https://docs.rs/agentfs/badge.svg" alt="Documentation"/>
  </a>
  <a href="LICENSE" target="_blank">
    <img src="https://img.shields.io/github/license/cryptopatrick/agentfs.svg" alt="License"/>
  </a>
</p>

<b>Author's bio:</b> 👋😀 Hi, I'm CryptoPatrick! I'm currently enrolled as an
Undergraduate student in Mathematics, at Chalmers & the University of Gothenburg, Sweden. <br>
If you have any questions or need more info, then please <a href="https://discord.gg/T8EWmJZpCB">join my Discord Channel: AiMath</a>

---

<p align="center">
  <a href="#-what-is-agentfs">What is AgentFS</a> •
  <a href="#-features">Features</a> •
  <a href="#-architecture">Architecture</a> •
  <a href="#-how-to-use">How To Use</a> •
  <a href="#-documentation">Documentation</a> •
  <a href="#-license">License</a>
</p>

## 🛎 Important Notices
* **POSIX-like** filesystem operations for AI agents
* **Multi-backend** support: SQLite, PostgreSQL, MySQL
* **Tool call auditing** built-in
* **Zero vendor lock-in** - fully open source

<!-- TABLE OF CONTENTS -->
<h2 id="table-of-contents"> :pushpin: Table of Contents</h2>

<details open="open">
  <summary>Table of Contents</summary>
  <ol>
    <li><a href="#-what-is-agentfs">What is AgentFS</a></li>
    <li><a href="#-features">Features</a></li>
      <ul>
        <li><a href="#-filesystem-operations">Filesystem Operations</a></li>
        <li><a href="#-key-value-store">Key-Value Store</a></li>
        <li><a href="#-tool-call-auditing">Tool Call Auditing</a></li>
      </ul>
    <li><a href="#-architecture">Architecture</a></li>
    <li><a href="#-how-to-use">How to Use</a></li>
    <li><a href="#-examples">Examples</a></li>
    <li><a href="#-testing">Testing</a></li>
    <li><a href="#-documentation">Documentation</a></li>
    <li><a href="#-author">Author</a></li>
    <li><a href="#-support">Support</a></li>
    <li><a href="#-license">License</a>
  </ol>
</details>

## 🤔 What is AgentFS

`agentfs` provides a high-level filesystem abstraction for AI agents, offering POSIX-like file operations, key-value storage, and tool call auditing. It enables agents to persist state, store generated files, and maintain audit trails across sessions.

Built on top of the [agentdb](../agentdb) abstraction layer and [agentsql](../agentsql) SQL backends, AgentFS provides a complete storage solution for AI agents with zero vendor lock-in.

### Use Cases

- **Agent Workspaces**: Provide agents with isolated filesystem workspaces for storing outputs
- **Multi-Agent Systems**: Share data between agents through a common filesystem
- **Tool Call Auditing**: Track all agent actions with built-in audit logging
- **State Management**: Store agent configuration and session state in KV store
- **Output Storage**: Persist agent-generated documents, reports, and artifacts
- **Cloud Deployment**: Deploy on managed databases (AWS RDS, Google Cloud SQL, Azure)

## 📷 Features

`agentfs` provides three high-level APIs for AI agent storage with production-grade features:

### 📁 Filesystem Operations

**POSIX-like Interface**:
- **write_file(path, data)**: Create or overwrite files with automatic parent directory creation
- **read_file(path)**: Read complete file contents into memory
- **mkdir(path)**: Create directories recursively
- **readdir(path)**: List directory contents with metadata
- **remove(path)**: Delete files and directories recursively
- **exists(path)**: Check if path exists
- **stat(path)**: Get file metadata (size, timestamps, permissions, type)

**Advanced Features**:
- **Symbolic Links**: Create and follow symlinks transparently
- **Path Normalization**: Automatic path cleaning and validation
- **Inode/Dentry Design**: Unix-like filesystem structure for reliability
- **Concurrent Access**: Safe multi-agent filesystem sharing with locking
- **Mount Point Isolation**: Sandboxed /agent root for security

### 🗄️ Key-Value Store

**Simple API**:
- **set(key, value)**: Store arbitrary key-value pairs
- **get(key)**: Retrieve values by key
- **delete(key)**: Remove keys permanently
- **scan(prefix)**: Find all keys matching a prefix
- **exists(key)**: Check key existence

**Use Cases**:
- Session state management for agent conversations
- Agent configuration and settings storage
- Caching computed results for performance
- Metadata storage for files and operations

### 📊 Tool Call Auditing

**Workflow-Based API**:
- **start(name, params)**: Begin tracking a tool call with parameters
- **success(id, result)**: Mark tool call as successful with optional result
- **error(id, error)**: Mark tool call as failed with error message

**Single-Shot API**:
- **record(name, start, end, params, result, error)**: Record completed tool call in one operation

**Analytics**:
- Get per-tool statistics (total calls, success rate, average duration)
- List recent tool calls with filtering by name, status, time range
- Track tool execution timelines and patterns
- Debug agent behavior with complete audit trail

## 📐 Architecture

1. 🏛 **Overall Architecture**

```diagram
┌─────────────────────────────────────────────────────┐
│              Application Layer                      │
│   • Agent frameworks (Rig, LangChain, custom)      │
│   • Multi-agent systems                            │
│   • CLI tools and services                         │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│                  AgentFS APIs                       │
│  ┌──────────────┬─────────────┬──────────────────┐ │
│  │  FileSystem  │   KvStore   │  ToolRecorder    │ │
│  │              │             │                  │ │
│  │ • mkdir      │ • set       │ • start          │ │
│  │ • write_file │ • get       │ • success        │ │
│  │ • read_file  │ • delete    │ • error          │ │
│  │ • readdir    │ • scan      │ • record         │ │
│  │ • remove     │ • exists    │ • statistics     │ │
│  │ • stat       │             │ • recent         │ │
│  └──────────────┴─────────────┴──────────────────┘ │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│              AgentDB Trait Interface                │
│  • Database-agnostic operations                     │
│  • put, get, delete, scan, query                    │
│  • Transaction support                              │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│               AgentSQL (SQLx)                       │
│  • Connection pooling                               │
│  • Migration system                                 │
│  • Type-safe SQL                                    │
│  • Multi-backend support                            │
└───┬────────────────┬────────────────┬───────────────┘
    │                │                │
┌───▼──────┐  ┌──────▼──────┐  ┌─────▼──────┐
│  SQLite  │  │ PostgreSQL  │  │   MySQL    │
│  Local   │  │  Production │  │   Cloud    │
└──────────┘  └─────────────┘  └────────────┘
```

2. 🗂️ **Filesystem Layer Architecture**

```diagram
┌─────────────────────────────────────────────────────┐
│              FileSystem API Calls                   │
│   mkdir("/docs")  write_file("/docs/a.txt")         │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│              Path Resolution                        │
│  • Normalize paths (remove .., .)                   │
│  • Validate against mount point (/agent)            │
│  • Split into parent + name components              │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│              Inode Operations                       │
│  • Lookup parent directory inode                    │
│  • Create new inode for file/dir                    │
│  • Update metadata (size, mtime, permissions)       │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│              Dentry Operations                      │
│  • Insert (parent_ino, name, ino) entry             │
│  • UNIQUE constraint ensures no duplicates          │
│  • Enables path-to-inode lookup                     │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│              Data Storage (for files)               │
│  • Store file content in fs_data table              │
│  • Chunk large files by offset                      │
│  • Link to inode via foreign key                    │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│              SQLite / PostgreSQL / MySQL            │
│  • fs_inode:  metadata (size, times, mode)          │
│  • fs_dentry: name resolution                       │
│  • fs_data:   file contents                         │
└─────────────────────────────────────────────────────┘
```

3. 🔄 **Tool Call Workflow**

```diagram
┌─────────────────────────────────────────────────────┐
│           Agent Calls Tool (e.g., web_search)       │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│     tools.start("web_search", params)               │
│  • Generate UUID for tool call                      │
│  • Store: name, params, started_at                  │
│  • Status: "pending"                                │
│  • Returns: call_id                                 │
└────────────────────┬────────────────────────────────┘
                     │
       ┌─────────────┴─────────────┐
       │                           │
┌──────▼──────────┐      ┌─────────▼──────────┐
│  Tool Succeeds  │      │   Tool Fails       │
└──────┬──────────┘      └─────────┬──────────┘
       │                           │
┌──────▼──────────────────┐  ┌─────▼────────────────────┐
│ tools.success(id, res)  │  │ tools.error(id, err)     │
│ • Update completed_at   │  │ • Update completed_at    │
│ • Store result JSON     │  │ • Store error message    │
│ • Status: "success"     │  │ • Status: "error"        │
│ • Calculate duration_ms │  │ • Calculate duration_ms  │
└─────────┬───────────────┘  └─────────┬────────────────┘
          │                            │
          └────────────┬───────────────┘
                       │
┌──────────────────────▼────────────────────────────────┐
│              Persisted in tool_calls table            │
│  • Query by name, status, time range                  │
│  • Generate statistics (success rate, avg duration)   │
│  • Audit trail for debugging and compliance           │
└───────────────────────────────────────────────────────┘
```

4. 💾 **Storage Implementation**

```diagram
┌─────────────────────────────────────────────────────┐
│                   AgentFS Instance                  │
│  • agent_name: "my-agent"                           │
│  • mount_point: "/agent"                            │
│  • db: Box<dyn AgentDB>                             │
│                                                     │
│  ┌──────────────────────────────────────────────┐  │
│  │            FileSystem Component              │  │
│  │  Uses: fs_inode, fs_dentry, fs_data tables  │  │
│  └──────────────────────────────────────────────┘  │
│                                                     │
│  ┌──────────────────────────────────────────────┐  │
│  │             KvStore Component                │  │
│  │  Uses: kv_store table                        │  │
│  │  Prefix: "kv:{agent_name}:"                  │  │
│  └──────────────────────────────────────────────┘  │
│                                                     │
│  ┌──────────────────────────────────────────────┐  │
│  │           ToolRecorder Component             │  │
│  │  Uses: tool_calls table                      │  │
│  │  All calls tagged with agent_name            │  │
│  └──────────────────────────────────────────────┘  │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│               Database Backend                      │
│  ┌──────────────────────────────────────────────┐  │
│  │  fs_inode: ino, mode, size, times            │  │
│  ├──────────────────────────────────────────────┤  │
│  │  fs_dentry: parent_ino, name, ino            │  │
│  ├──────────────────────────────────────────────┤  │
│  │  fs_data: ino, offset, size, data (BLOB)     │  │
│  ├──────────────────────────────────────────────┤  │
│  │  kv_store: key, value, timestamps            │  │
│  ├──────────────────────────────────────────────┤  │
│  │  tool_calls: id, name, params, result, ...   │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## 🚙 How to Use

### Requirements
- Rust 1.70 or higher
- Database backend (SQLite, PostgreSQL, or MySQL)

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
agentfs = "0.1"
agentsql = { version = "0.1", features = ["sqlite"] }

# For PostgreSQL:
# agentsql = { version = "0.1", features = ["postgres"] }

# For MySQL:
# agentsql = { version = "0.1", features = ["mysql"] }
```

Or install with cargo:

```bash
cargo add agentfs
cargo add agentsql --features sqlite
```

### Example: SQLite (Local Development)

```rust
use agentfs::{AgentFS, FileSystem, KvStore, ToolRecorder};
use agentsql::SqlBackend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create AgentFS with SQLite backend
    let backend = SqlBackend::sqlite("agent.db").await?;
    let agent_fs = AgentFS::new(Box::new(backend), "my-agent", "/agent").await?;

    // Filesystem operations
    agent_fs.fs.mkdir("/output").await?;
    agent_fs.fs.write_file("/output/report.txt", b"Hello, World!").await?;

    let content = agent_fs.fs.read_file("/output/report.txt").await?.unwrap();
    println!("File content: {}", String::from_utf8_lossy(&content));

    // List directory
    let entries = agent_fs.fs.readdir("/output").await?;
    for entry in entries {
        println!("{}: {} bytes", entry.name, entry.size);
    }

    // Key-value store
    agent_fs.kv.set("config:theme", b"dark").await?;
    let theme = agent_fs.kv.get("config:theme").await?.unwrap();
    println!("Theme: {}", String::from_utf8_lossy(&theme));

    // Tool call auditing
    let id = agent_fs.tools.start("web_search", Some(serde_json::json!({
        "query": "Rust async programming"
    }))).await?;

    // Simulate search...
    agent_fs.tools.success(id, Some(serde_json::json!({
        "results": 10,
        "duration_ms": 123
    }))).await?;

    // Get statistics
    let stats = agent_fs.tools.statistics("web_search").await?;
    println!("Success rate: {:.1}%", stats.success_rate * 100.0);

    Ok(())
}
```

### Example: PostgreSQL (Production)

```rust
use agentfs::AgentFS;
use agentsql::SqlBackend;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to PostgreSQL
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://user:pass@localhost/agentfs".to_string());

    let backend = SqlBackend::postgres(database_url).await?;
    let agent_fs = AgentFS::new(Box::new(backend), "prod-agent", "/agent").await?;

    // Same API as SQLite!
    agent_fs.fs.write_file("/logs/app.log", b"System started").await?;

    // Scan KV store
    agent_fs.kv.set("session:user123", b"active").await?;
    let sessions = agent_fs.kv.scan("session:").await?;
    println!("Found {} active sessions", sessions.len());

    Ok(())
}
```

### Example: MySQL (Cloud Deployment)

```rust
use agentfs::AgentFS;
use agentsql::SqlBackend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to MySQL (e.g., AWS Aurora)
    let backend = SqlBackend::mysql(
        "mysql://user:pass@aurora-cluster.region.rds.amazonaws.com/agentfs"
    ).await?;

    let agent_fs = AgentFS::new(Box::new(backend), "cloud-agent", "/agent").await?;

    // Multi-agent coordination
    agent_fs.fs.mkdir("/shared").await?;
    agent_fs.fs.write_file("/shared/status.json", b"{\"status\":\"ready\"}").await?;

    // List recent tool calls across all agents
    let recent = agent_fs.tools.recent(10).await?;
    for call in recent {
        println!("{}: {} ({})", call.name, call.status, call.duration_ms);
    }

    Ok(())
}
```

## 🧪 Examples

The repository includes three comprehensive examples demonstrating different backends and use cases:

### Example 1: Basic Operations (SQLite)

See [`examples/basic.rs`](examples/basic.rs) for a complete example demonstrating:
- File write/read operations
- Directory management
- KV store usage
- Tool call recording with start/success/error workflow
- Statistics gathering

Run with:
```bash
cargo run --example basic
```

### Example 2: PostgreSQL Multi-Agent System

See [`examples/postgres.rs`](examples/postgres.rs) for:
- Concurrent file operations from multiple agents
- Shared state across agents using KV store
- PostgreSQL-specific features and configuration
- Production deployment patterns

Run with:
```bash
export DATABASE_URL="postgres://user:password@localhost/agentfs_demo"
cargo run --example postgres --features postgres
```

### Example 3: MySQL Cloud Deployment

See [`examples/mysql.rs`](examples/mysql.rs) for:
- Cloud deployment patterns (AWS Aurora, Google Cloud SQL)
- Production workflow with tool auditing
- Multi-agent coordination example
- MySQL-specific optimizations

Run with:
```bash
export DATABASE_URL="mysql://user:password@localhost/agentfs_demo"
cargo run --example mysql --features mysql
```

## 🧪 Testing

Run the comprehensive test suite:

```bash
# Run all tests (SQLite)
cargo test

# Run tests with output
cargo test -- --nocapture

# Run with all backends
cargo test --all-features

# Test specific backend
cargo test --features postgres
cargo test --features mysql
```

The test suite includes:
- Filesystem operations (mkdir, write, read, remove, readdir)
- Path normalization and validation
- KV store operations (set, get, delete, scan)
- Tool call workflow (start, success, error, record)
- Statistics and analytics
- Concurrent access patterns
- Error handling

## 📚 Documentation

Comprehensive documentation is available at [docs.rs/agentfs](https://docs.rs/agentfs), including:
- Complete API reference for FileSystem, KvStore, and ToolRecorder
- Architecture overview and design decisions
- Migration guide from other agent filesystems
- Performance optimization tips
- Multi-agent coordination patterns
- Backend selection guide (SQLite vs PostgreSQL vs MySQL)

## 🎯 Comparison

| Feature | AgentFS | Other Solutions |
|---------|---------|-----------------|
| **Backend Choice** | SQLite, PostgreSQL, MySQL | Vendor-specific |
| **Open Source** | ✅ MIT Licensed | ⚠️ Varies |
| **Self-Hosted** | ✅ Yes | ❌ Cloud-only |
| **Tool Auditing** | ✅ Built-in | ❌ Not included |
| **Zero Cost** | ✅ Yes | ❌ Usage-based pricing |
| **Local Development** | ✅ SQLite | ⚠️ Requires cloud account |
| **POSIX-like API** | ✅ Yes | ⚠️ Limited |
| **Multi-Agent** | ✅ Native support | ⚠️ Requires workarounds |

## 🖊 Author

<a href="https://x.com/cryptopatrick">CryptoPatrick</a>

Keybase Verification:
https://keybase.io/cryptopatrick/sigs/8epNh5h2FtIX1UNNmf8YQ-k33M8J-Md4LnAN

## 🐣 Support

Leave a ⭐ if you think this project is cool.

## 🗄 License

This project is licensed under MIT. See [LICENSE](LICENSE) for details.
