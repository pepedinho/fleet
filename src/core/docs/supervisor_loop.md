# `supervisor_loop`

## Description
The `supervisor_loop` function is the **main orchestration loop** of the daemon.  
It periodically checks for updates in all watched repositories and triggers pipelines when new commits are detected.

The loop never holds the global state lock across blocking I/O, so a slow or unreachable remote cannot freeze the daemon.

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
   - Calls `collect_updates(&state, &git_semaphore)`, which returns the ids of watches that received a new commit.  
   - Polling is bounded: git calls run in `spawn_blocking` with a 30s timeout and at most `MAX_CONCURRENT_GIT_POLLS` in flight, so one stalled remote cannot starve the daemon.  

3. **Pipeline Scheduling**  
   For each updated project:  
   - Skips it if a pipeline for that watch is already running.  
   - Clones the `WatchContext`, switches to the branch to be deployed (best-effort, off the runtime), then executes the associated pipeline in a **spawned task** — pipelines are no longer run inline, so a long build does not delay the next supervision cycle.  
     - On success, persists the state to disk (single-writer, guarded) and logs a ✅ message.  
     - On failure, logs a ❌ error with details.  

4. **Loop Continuation**  
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
- The state lock is only ever held for cheap in-memory reads/writes; all blocking git I/O happens outside of it.  
- A failing pipeline does not stop the supervisor loop.  
- Per-watch pipelines can overlap across two different watches, but never for the same watch.  
- The persistence step is serialized so `watches.json` is written by a single writer at a time.