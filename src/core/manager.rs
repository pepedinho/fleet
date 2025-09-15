#![allow(dead_code)]
use std::{os::unix::fs::PermissionsExt, path::Path, sync::Arc, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, BufReader, split},
    net::UnixListener,
    time::interval,
};

use crate::{
    core::{
        state::AppState,
        watcher::{WatchContext, watch_once},
    },
    daemon::server::{DaemonRequest, handle_request},
    exec::pipeline::run_pipeline,
};

#[doc = include_str!("docs/supervisor_loop.md")]
pub async fn supervisor_loop(state: Arc<AppState>, interval_secs: u64) {
    let mut ticker = interval(Duration::from_secs(interval_secs));

    loop {
        ticker.tick().await;

        let to_update = collect_updates(&state).await;
        let mut dirty = false;

        for (id, new_commit) in to_update {
            update_commit(&state, &id, new_commit.clone()).await;
            if let Some(ctx) = get_watch_ctx(&state, &id).await {
                match run_pipeline(Arc::new(ctx)).await {
                    Ok(_) => {
                        println!("[{id}] ✅ Update succeeded");
                        dirty = true;
                    }
                    Err(e) => {
                        eprintln!("[{id}] ❌ Update failed => {e}");
                    }
                }
            }
        }

        if dirty && let Err(e) = state.save_to_disk().await {
            eprintln!("❌ Failed to save state: {e}");
        }
    }
}

/// Loop through the watches, call `watch_once` on each one,
/// return the (id, new_commit) to update.
async fn collect_updates(state: &Arc<AppState>) -> Vec<(String, String)> {
    let mut to_update = Vec::new();
    let guard = state.watches.read().await;

    for (id, ctx) in guard.iter() {
        if ctx.paused {
            continue;
        }
        match watch_once(ctx).await {
            Ok(Some(new_commit)) => {
                println!("[{id}] ✔ OK");
                to_update.push((id.clone(), new_commit));
            }
            Ok(None) => {}
            Err(e) => eprintln!("[{id}] ❌ Watch failed: {e}"),
        }
    }

    to_update
}

/// Updates the commit stored in the state for a given watch.
async fn update_commit(state: &Arc<AppState>, id: &str, new_commit: String) {
    let mut watches_write = state.watches.write().await;
    if let Some(ctx) = watches_write.get_mut(id) {
        ctx.repo.last_commit = new_commit;
    }
}

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
            if let Err(e) = reader.read_line(&mut buf).await {
                eprintln!("❌ Failed to read from stream: {e}");
                return;
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
