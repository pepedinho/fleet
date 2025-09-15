# `supervisor_loop`

## Description
The `supervisor_loop` function is the **main orchestration loop** of the daemon.  
It periodically checks for updates in all watched repositories and triggers pipelines when new commits are detected.

This function runs indefinitely, performing the following steps at each interval:
1. Collects updates from all active watches.  
2. Applies commit updates to the in-memory state.  
3. Executes the processing pipeline for updated projects.  
4. Persists the state to disk if changes occurred.  

---

## Arguments
- `state: Arc<AppState>` – Shared application state holding all active `WatchContext` instances.  
- `interval_secs: u64` – The interval in seconds between supervisor checks.  

---

## Behavior

1. **Ticker Initialization**  
   - Uses a Tokio interval (`tokio::time::interval`) to create a periodic timer.  
   - Waits `interval_secs` seconds between each supervisor cycle.  

2. **Update Collection**  
   - Calls `collect_updates(&state)` to determine which projects have new commits available.  
   - Returns a list of `(id, new_commit)` pairs to be updated.  

3. **Commit Updates & Pipeline Execution**  
   For each updated project:  
   - Calls `update_commit` to update the in-memory commit reference.  
   - Retrieves the `WatchContext` via `get_watch_ctx`.  
   - Runs the associated pipeline with `run_pipeline`.  
     - On success, logs a ✅ success message and marks the state as dirty.  
     - On failure, logs a ❌ error with details.  

4. **State Persistence**  
   - If at least one pipeline succeeded (`dirty == true`), the updated state is saved to disk using `state.save_to_disk()`.  
   - Errors during persistence are logged.  

5. **Loop Continuation**  
   - The loop repeats indefinitely, making `supervisor_loop` the central control flow of the orchestrator.  

---

## Example Usage

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = Arc::new(AppState::load_from_disk().await?);
    // Run supervisor every 30 seconds
    supervisor_loop(state, 30).await;
    Ok(())
}
```

---

## Notes
- The supervisor is **always running**, ensuring projects are kept in sync with their remote repositories.  
- Pipeline execution is performed sequentially within each cycle, but each cycle runs concurrently with other daemon tasks (e.g., socket listener).  
- Proper error handling ensures that a failing pipeline does not stop the supervisor loop.  
- The persistence step guarantees that the orchestrator can recover its state after a restart.  
