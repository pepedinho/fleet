# `WatchContext`

## Description
The `WatchContext` structure represents the execution context for monitoring a Git repository branch.  
It acts as the central object passed around the orchestrator, containing all the necessary metadata, configuration, and logging utilities required to monitor and process changes.

Each context is associated with:
- A specific Git branch.
- A repository definition (`Repo`).
- Project configuration (`ProjectConfig`).
- A unique identifier (`id`) and project directory path.
- A logger for runtime logging.

The `paused` flag indicates whether monitoring is currently active or suspended.

---

## Fields
- `branch: String` – The name of the branch being monitored.  
- `repo: Repo` – Repository information, including remote and last commit hash.  
- `config: ProjectConfig` – Project-specific configuration.  
- `project_dir: String` – Filesystem path to the project root directory.  
- `id: String` – Unique identifier for this context (used in log file naming).  
- `paused: bool` – Whether the watcher is currently paused.  
- `logger: Logger` – The logger used for output and persistent logging.

---

## Associated Methods

### `stop(&mut self)`
Pauses monitoring by setting the `paused` flag to `true`.  
This does not destroy the context but signals that no new operations should be triggered until resumed.

**Example:**
```rust
ctx.stop();
assert!(ctx.paused);
```

---

### `run(&mut self)`
Resumes monitoring by setting the `paused` flag back to `false`.  

**Example:**
```rust
ctx.run();
assert!(!ctx.paused);
```

---

### `log_path(&self) -> PathBuf`
Returns the full path to the log file associated with this context.  
The path is resolved under the user’s home directory:

```
~/.fleet/logs/<id>.log
```

**Example:**
```rust
let log_file = ctx.log_path();
println!("Log file: {}", log_file.display());
```

---

### `log_path_by_id(id: &str) -> PathBuf`
Static method returning the log file path for a given context `id`, without requiring an instance of `WatchContext`.

**Example:**
```rust
let log_file = WatchContext::log_path_by_id("project-123");
println!("Log file: {}", log_file.display());
```

---

### `init_logs() -> Result<()>`
Asynchronously initializes the logs directory under:

```
~/.fleet/logs/
```

If the directory does not exist, it is created.  
If it already exists, nothing happens.

**Example:**
```rust
WatchContext::init_logs().await?;
```

---

## Notes
- `WatchContext` is cloneable (`Clone`), making it safe to pass across async tasks if needed.  
- The log file is tied to the context `id`, ensuring each project has its own persistent log.  
- Pausing/resuming does not affect the internal state of the repository, only whether actions are triggered.  
