#![allow(dead_code)]
use std::{os::unix::fs::PermissionsExt, path::Path, sync::Arc, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, BufReader, split},
    net::UnixListener,
    time::interval,
};

use crate::{
    core::{state::AppState, watcher::watch_once},
    exec::runner::run_update,
    ipc::server::{DaemonRequest, handle_request},
};

pub async fn supervisor_loop(state: Arc<AppState>, interval_secs: u64) {
    let mut ticker = interval(Duration::from_secs(interval_secs));

    loop {
        ticker.tick().await;

        // let projects = {
        //     let guard = state.watches.read().await;
        //     guard.clone()
        // };
        let mut dirty = false;
        let mut to_update = Vec::new();
        {
            let guard = state.watches.read().await;

            for (id, ctx) in guard.iter() {
                match watch_once(ctx).await {
                    Ok(Some(new_commit)) => {
                        println!("[{}] ✔ OK", id);
                        to_update.push((id.clone(), new_commit));
                    }
                    Ok(None) => {
                        println!("[{}] ✔ No change", id);
                    }
                    Err(e) => eprintln!("[{}] ❌ Watch failed: {}", id, e),
                }
            }
        } // guard drop here

        for (id, new_commit) in to_update {
            {
                let mut watches_write = state.watches.write().await;
                if let Some(ctx) = watches_write.get_mut(&id) {
                    ctx.repo.last_commit = new_commit.clone();
                }
            } // drop lock here

            if let Some(ctx) = {
                let watches_read = state.watches.read().await;
                watches_read.get(&id).cloned()
            } {
                if let Err(e) = run_update(&ctx).await {
                    eprintln!("[{}] ❌ Update failed: {}", id, e);
                } else {
                    println!("[{}] ✅ Update succeeded", id);
                    dirty = true;
                }
            }
        }
        if dirty {
            if let Err(e) = state.save_to_disk().await {
                eprintln!("❌ Failed to save state: {}", e);
            }
        }
    }
}

pub async fn start_socket_listener(state: Arc<AppState>) -> anyhow::Result<()> {
    let sock_path = Path::new("/tmp/fleetd.sock");
    if sock_path.exists() {
        std::fs::remove_file(sock_path)?;
    }

    let listener = UnixListener::bind(sock_path)?;
    std::fs::set_permissions(sock_path, std::fs::Permissions::from_mode(0o666))?;

    println!("🔌 fleetd is listening on {:?}", sock_path);

    loop {
        let (stream, _) = listener.accept().await?;

        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let (read_half, mut write_half) = split(stream);
            let mut reader = BufReader::new(read_half);
            let mut buf = String::new();
            if let Err(e) = reader.read_line(&mut buf).await {
                eprintln!("❌ Failed to read from stream: {}", e);
                return;
            }

            let parsed: Result<DaemonRequest, _> = serde_json::from_str(&buf);
            match parsed {
                Ok(req) => {
                    if let Err(e) = handle_request(req, state, &mut write_half).await {
                        eprintln!("❌ Request handling failed: {}", e);
                    }
                    println!("oui");
                }
                Err(e) => eprintln!("❌ JSON parsing error: {}", e),
            }
        });
    }
}
