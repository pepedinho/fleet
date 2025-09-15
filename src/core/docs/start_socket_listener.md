# `start_socket_listener`

## Description
The `start_socket_listener` function starts a **Unix domain socket server** that listens for incoming client connections.  
It acts as the main communication entry point for the `fleetd` daemon, enabling external processes to interact with the orchestrator through JSON-based requests.

This function runs **indefinitely** in a loop, spawning a new asynchronous task for each client connection.

---

## Arguments
- `state: Arc<AppState>` – A thread-safe shared reference to the application state (`AppState`).  
  Each client request handler receives a clone of this state, allowing concurrent access to watches.

---

## Behavior

1. **Socket Initialization**  
   - Creates a Unix domain socket at `/tmp/fleetd.sock`.  
   - Removes any stale socket file if it already exists.  
   - Binds a new listener to the path and sets permissions (`0o666`) so other processes can connect.  

2. **Listening Loop**  
   - Prints a startup message (`fleetd is listening`).  
   - Waits for incoming client connections in an infinite loop.  

3. **Client Handling**  
   For each accepted connection:  
   - The socket is split into a **read half** and **write half**.  
   - Reads a single line from the client into a buffer.  
   - Attempts to parse the buffer as a `DaemonRequest` (JSON).  
   - If parsing succeeds, forwards the request to `handle_request`, passing the shared state and the writable stream.  
   - If parsing fails, logs an error.  

4. **Concurrency**  
   Each client connection is processed in a dedicated Tokio task (`tokio::spawn`).  
   This ensures multiple clients can interact with the daemon concurrently.  

---

## Example Usage

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = Arc::new(AppState::load_from_disk().await?);
    start_socket_listener(state).await
}
```

Once running, other processes can connect to `/tmp/fleetd.sock` and send JSON-formatted requests.

---

## Notes
- The socket path `/tmp/fleetd.sock` is Linux/Unix specific and may need adaptation on other platforms.  
- Only **one instance** of `fleetd` can listen on the socket at a time.  
- Proper error handling ensures the daemon does not crash on malformed requests.  
- Each client connection is independent, preventing one failing client from blocking others.  
