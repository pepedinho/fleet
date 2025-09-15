# `WatchContextBuilder::build`

## Description
The `build` method finalizes the construction of a [`WatchContext`] by consuming a 
[`WatchContextBuilder`]. It ensures that all necessary components, such as the logger, 
are initialized before returning the fully constructed context.

This method is **asynchronous** because the logger initialization (`Logger::new`) 
may require I/O operations (e.g., creating or opening log files).

Once called, the builder is consumed and cannot be reused.

---

## Arguments
This method does not take any additional arguments.  
It consumes the builder (`self`) that already contains:

- `branch`: Name of the branch to watch.
- `repo`: The Git repository metadata, including remote and last commit hash.
- `config`: Project configuration associated with this context.
- `project_dir`: Path to the project directory.
- `id`: A unique identifier for the context.
- `paused`: Initial paused state (default: `false`).

---

## Returns
- `Ok(WatchContext)` – A fully initialized watch context with a ready-to-use logger.
- `Err(anyhow::Error)` – An error occurred during logger creation or path resolution.

---

## Example

```rust
use crate::core::WatchContextBuilder;

let builder = WatchContextBuilder::new(
    "main".to_string(),
    repo,
    config,
    "/path/to/project".to_string(),
    "project-123".to_string(),
);

let context = builder.build().await?;
println!("WatchContext initialized for branch: {}", context.branch);
```

## Notes

- The log file is created in the user’s home directory, under ~/.fleet/logs/<id>.log.

- If the home directory cannot be resolved or the logger initialization fails,
the method returns an error.

- This method consumes the builder to ensure immutability of the constructed
[WatchContext].