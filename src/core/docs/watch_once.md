# watch_once

Checks if a new commit is present on the tracked remote branch.

## Description
Compares the last known local commit (`ctx.repo.last_commit`) with the current
commit hash of the remote branch. Returns `Some(hash)` if a new commit is detected,
or `None` if unchanged.

- Without `force_commit`:
  - Returns `Some(hash)` if a new commit exists.
  - Returns `None` if no changes.
- With `force_commit`:
  - Always returns `Some(last_commit)` to force processing.

## Arguments
- `ctx`: Context containing repository, branch, and other metadata.

## Returns
- `Ok(Some(String))`: New commit hash (or forced).
- `Ok(None)`: No new commit.
- `Err(anyhow::Error)`: Error during remote hash retrieval.

## Notes
- Force commit mode may return a hash even if no new commit exists.
- Errors can occur if the remote branch does not exist or on network failure.

## Example
```ignore
let result = watch_once(&ctx).await?;
match result {
    Some(hash) => println!("New commit: {}", hash),
    None => println!("No new commit detected."),
}
