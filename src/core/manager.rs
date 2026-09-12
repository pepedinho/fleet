#![allow(dead_code)]
use std::{
    collections::HashSet, os::unix::fs::PermissionsExt, path::Path, sync::Arc, time::Duration,
};

use tokio::{
    io::{AsyncBufReadExt, BufReader, split},
    net::UnixListener,
    sync::{Mutex, Semaphore},
    time::{interval, timeout},
};

use crate::{
    core::{
        id::format_commit,
        state::AppState,
        watcher::{WatchContext, watch_once},
    },
    daemon::server::{DaemonRequest, handle_request},
    exec::pipeline::run_pipeline,
    git::repo::Repo,
};

/// How long a single git poll (`watch_once`) may take before it is abandoned.
/// Tunable via `FLEET_GIT_POLL_TIMEOUT_MS` (default 30s).
fn git_poll_timeout() -> Duration {
    let default_ms = 30_000u64;
    let ms = std::env::var("FLEET_GIT_POLL_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_ms);
    Duration::from_millis(ms)
}

/// Max number of git polls running at once. Tunable via `FLEET_MAX_GIT_POLLS`
/// (default 2). Blocking git2 calls are kept off the async runtime and bounded
/// in flight so that one unreachable remote cannot starve the whole daemon.
fn max_concurrent_git_polls() -> usize {
    std::env::var("FLEET_MAX_GIT_POLLS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2)
}

#[doc = include_str!("docs/supervisor_loop.md")]
pub async fn supervisor_loop(state: Arc<AppState>, interval_secs: u64) {
    let mut ticker = interval(Duration::from_secs(interval_secs));
    let git_semaphore = Arc::new(Semaphore::new(max_concurrent_git_polls()));
    let git_timeout = git_poll_timeout();
    let running = Arc::new(Mutex::new(HashSet::new()));
    let save_lock = Arc::new(Mutex::new(()));

    loop {
        ticker.tick().await;

        for id in collect_updates(&state, &git_semaphore, git_timeout).await {
            if running.lock().await.contains(&id) {
                // A pipeline for this watch is already in flight, skip it.
                continue;
            }
            let Some(ctx) = get_watch_ctx(&state, &id).await else {
                continue;
            };
            running.lock().await.insert(id.clone());

            let running = Arc::clone(&running);
            let save_lock = Arc::clone(&save_lock);
            let state = Arc::clone(&state);
            let branch = ctx.repo.branches.last_name.clone();
            let id2 = id.clone();

            tokio::spawn(async move {
                let ctx = Arc::new(ctx);

                // Switch to the branch to be deployed before running the
                // pipeline. Checkout is blocking git2 I/O, so it runs off the
                // async runtime and is best-effort, matching the previous
                // `Repo::switch_branch(...).ok()` semantics.
                if !branch.is_empty() {
                    let switch_ctx = Arc::clone(&ctx);
                    match timeout(
                        git_timeout,
                        tokio::task::spawn_blocking(move || {
                            Repo::switch_branch(&switch_ctx, &branch)
                        }),
                    )
                    .await
                    {
                        Ok(Ok(Ok(()))) => {}
                        Ok(Ok(Err(e))) => {
                            eprintln!("[{id2}] ⚠ branch switch failed (ignored): {e}")
                        }
                        Ok(Err(e)) => {
                            eprintln!("[{id2}] ⚠ branch switch task died (ignored): {e}")
                        }
                        Err(_) => eprintln!("[{id2}] ⚠ branch switch timed out (ignored)"),
                    }
                }

                match run_pipeline(ctx).await {
                    Ok(_) => {
                        println!("[{id2}] ✅ Update succeeded");
                        let _save = save_lock.lock().await;
                        if let Err(e) = state.save_to_disk().await {
                            eprintln!("❌ Failed to save state: {e}");
                        }
                    }
                    Err(e) => {
                        eprintln!("[{id2}] ❌ Update failed => {e}");
                    }
                }
                running.lock().await.remove(&id2);
            });
        }
    }
}

/// Poll every active watch for new commits.
///
/// The state lock is only ever held for cheap in-memory reads/writes: the
/// slow git network calls happen in `spawn_blocking` after a snapshot has been
/// taken and are bounded by `git_timeout`, so a stalled remote no longer blocks
/// the socket listener, the `run` command, or any other watch.
async fn collect_updates(
    state: &Arc<AppState>,
    git_semaphore: &Arc<Semaphore>,
    git_timeout: Duration,
) -> Vec<String> {
    let snapshot: Vec<(String, bool, crate::git::repo::Repo)> = {
        let guard = state.watches.read().await;
        guard
            .iter()
            .map(|(id, ctx)| (id.clone(), ctx.paused, ctx.repo.clone()))
            .collect()
    };

    let mut updated = Vec::new();

    for (id, paused, mut repo) in snapshot {
        if paused {
            continue;
        }

        let _permit = match git_semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => break,
        };

        let poll_result = timeout(
            git_timeout,
            tokio::task::spawn_blocking(move || {
                let result = watch_once(&mut repo);
                result.map(|detected| {
                    detected.map(|commit| (commit, repo.branches.last_name.clone()))
                })
            }),
        )
        .await;

        let (new_commit, branch) = match poll_result {
            Ok(Ok(Ok(Some(found)))) => found,
            Ok(Ok(Ok(None))) => continue,
            Ok(Ok(Err(e))) => {
                eprintln!("[{id}] ❌ Watch failed: {e}");
                continue;
            }
            Ok(Err(e)) => {
                eprintln!("[{id}] ❌ Watch task panicked: {e}");
                continue;
            }
            Err(_) => {
                eprintln!("[{id}] ⏱ git poll timed out after {git_timeout:?}");
                continue;
            }
        };

        // Apply the new commit under a short lock, pulling the logger out so
        // the log write itself does not happen under the lock.
        let logger = {
            let mut guard = state.watches.write().await;
            match guard.get_mut(&id) {
                Some(ctx) => {
                    ctx.repo.branches.last_commit = new_commit.clone();
                    ctx.repo.branches.last_name = branch.clone();
                    Some(ctx.logger.clone())
                }
                None => None,
            }
        };
        let Some(logger) = logger else {
            continue;
        };

        logger
            .info(&format!(
                "New commit [{}] from branch {}",
                format_commit(&new_commit),
                branch
            ))
            .await
            .ok();

        updated.push(id);
    }

    updated
}

/// Clones the `WatchContext` of a watch, if it still exists.
pub async fn get_watch_ctx(state: &Arc<AppState>, id: &str) -> Option<WatchContext> {
    let watches_read: tokio::sync::RwLockReadGuard<
        '_,
        std::collections::HashMap<String, WatchContext>,
    > = state.watches.read().await;
    watches_read.get(id).cloned()
}

#[doc = include_str!("docs/start_socket_listener.md")]
pub async fn start_socket_listener(state: Arc<AppState>) -> anyhow::Result<()> {
    let sock_path = Path::new("/tmp/fleetd.sock");
    if sock_path.exists() {
        std::fs::remove_file(sock_path)?;
    }

    let listener = UnixListener::bind(sock_path)?;
    std::fs::set_permissions(sock_path, std::fs::Permissions::from_mode(0o666))?;

    println!("🔌 fleetd is listening on {sock_path:?}");

    loop {
        let (stream, _) = listener.accept().await?;

        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let (read_half, mut write_half) = split(stream);
            let mut reader = BufReader::new(read_half);
            let mut buf = String::new();

            // A client that connects but never sends a request must not hold
            // its spawned task (and socket) forever.
            let read = timeout(Duration::from_secs(5), reader.read_line(&mut buf)).await;
            match read {
                Ok(Ok(_n)) => {}
                Ok(Err(e)) => {
                    eprintln!("❌ Failed to read from stream: {e}");
                    return;
                }
                Err(_) => {
                    eprintln!("❌ No request received within 5s, closing connection");
                    return;
                }
            }

            let parsed: Result<DaemonRequest, _> = serde_json::from_str(&buf);
            match parsed {
                Ok(req) => {
                    if let Err(e) = handle_request(req, state, &mut write_half).await {
                        eprintln!("❌ Request handling failed: {e}");
                    }
                }
                Err(e) => eprintln!("❌ JSON parsing error: {e}"),
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use tokio::sync::Semaphore;

    use crate::{
        config::ProjectConfig,
        core::state::AppState,
        core::watcher::WatchContext,
        git::repo::{Branch, Branches, Repo},
        log::logger::Logger,
    };

    use super::collect_updates;

    /// Regression test: a remote that accepts connections but never answers
    /// used to block the daemon for the OS-level connect/read timeout while
    /// holding the state lock. `collect_updates` must now give up within the
    /// configured git poll timeout and keep the rest of the daemon responsive.
    #[tokio::test]
    async fn collect_updates_returns_promptly_even_if_remote_hangs() -> anyhow::Result<()> {
        // A TCP server that accepts connections but never speaks. libgit2's ssh
        // transport blocks forever waiting for the server's handshake banner.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let url = format!("ssh://127.0.0.1:{port}/repo.git");

        let (quit_tx, quit_rx) = tokio::sync::oneshot::channel::<()>();
        let hang_server = tokio::spawn(async move {
            let mut quit_rx = quit_rx;
            let mut held = Vec::new();
            loop {
                tokio::select! {
                    _ = &mut quit_rx => break,
                    res = listener.accept() => match res {
                        // Hold the socket open, never respond.
                        Ok((stream, _)) => held.push(stream),
                        Err(_) => break,
                    },
                }
            }
            drop(held);
        });

        let ctx = WatchContext {
            repo: Repo {
                branches: Branches {
                    branches: vec![Branch {
                        branch: "main".to_string(),
                        last_commit: "old".to_string(),
                        remote: url.clone(),
                        name: "hang-repo".to_string(),
                    }],
                    last_commit: "old".to_string(),
                    last_name: "main".to_string(),
                    name: "main".to_string(),
                },
                name: "hang-repo".to_string(),
                remote: url,
            },
            config: ProjectConfig::default(),
            project_dir: "/nonexistent".to_string(),
            id: "hang-watch".to_string(),
            paused: false,
            logger: Logger::placeholder(),
        };

        let mut watches = HashMap::new();
        watches.insert(ctx.id.clone(), ctx);
        let state = Arc::new(AppState {
            watches: tokio::sync::RwLock::new(watches),
        });
        let sem = Arc::new(Semaphore::new(1));

        let began = Instant::now();
        let updates = tokio::time::timeout(
            Duration::from_secs(5),
            collect_updates(&state, &sem, Duration::from_millis(500)),
        )
        .await
        .map_err(|_| anyhow::anyhow!("collect_updates did not return within 5s"))?;

        assert!(
            updates.is_empty(),
            "a hanging remote must not produce updates"
        );
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "collect_updates must respect the git poll timeout"
        );

        // Release the held sockets so the leaked blocking git2 task unwinds.
        drop(quit_tx);
        let _ = hang_server.await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        Ok(())
    }
}
