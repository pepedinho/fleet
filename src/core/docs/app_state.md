# `AppState`

## Description
The `AppState` structure represents the **in-memory runtime state** of the orchestrator.  
It manages all active `WatchContext` instances in a concurrent-safe way using an `RwLock<HashMap<String, WatchContext>>`.  

This state is central to the daemon’s operation: it allows concurrent tasks (e.g., socket requests, watchers) to safely read and modify the currently tracked projects.

---

## Fields
- `watches: RwLock<HashMap<String, WatchContext>>`  
  A thread-safe map of project identifiers (`id`) to their associated `WatchContext`.  

---

## Associated Methods

### `load_from_disk() -> Result<Self>`
Loads the state from the persisted **watches.json** file.  
- Reconstructs all `WatchContext` objects from disk.  
- Re-initializes their loggers.  
- Returns a fully built `AppState`.  

**Example:**
```rust
let state = AppState::load_from_disk().await?;
```

---

### `save_to_disk(&self) -> Result<()>`
Serializes the current state into the **watches.json** file.  
Ensures that all current contexts (`WatchContext`) are persisted.  

**Example:**
```rust
state.save_to_disk().await?;
```

---

### `init_watch_file() -> Result<()>`
Initializes the **watches.json** file if it does not exist.  
- Ensures the parent directory exists.  
- Creates an empty `WatchRegistry` if no registry file is found.  

**Example:**
```rust
AppState::init_watch_file().await?;
```

---

### `load_watches() -> Result<WatchRegistry>`
Loads the contents of **watches.json** and deserializes it into a `WatchRegistry`.  
This is a **low-level helper** used internally by other methods.  

**Example:**
```rust
let registry = AppState::load_watches().await?;
```

---

### `add_watch(ctx: &WatchContext) -> Result<()>`
Adds or updates a `WatchContext` inside the **watches.json** file.  
- If a project with the same `project_dir` already exists, it is updated.  
- Otherwise, the new watch is appended.  

**Example:**
```rust
let ctx = WatchContextBuilder::new(...).build().await?;
AppState::add_watch(&ctx).await?;
```

---

### `remove_watch_by_id(id: &str) -> Result<()>`
Removes a `WatchContext` from **watches.json** by its identifier.  

**Example:**
```rust
AppState::remove_watch_by_id("project-123").await?;
```

---

## Usage Example

`AppState` is typically wrapped in an `Arc` and shared across async tasks.  
For instance, in a socket listener:

```rust
pub async fn start_socket_listener(state: Arc<AppState>) -> anyhow::Result<()> {
    // state is cloned into each request handler
    let state = Arc::clone(&state);
    // requests are processed concurrently
}
```

This makes `AppState` the **central shared runtime state** for the orchestrator daemon.

---

## Notes
- `AppState` bridges **persistent storage** (`watches.json`) with **in-memory runtime state**.  
- It guarantees safe concurrent access to watches via `RwLock`.  
- It is designed to be long-lived and shared across tasks via `Arc<AppState>`.  
