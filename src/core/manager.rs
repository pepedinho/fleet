use std::{os::unix::fs::PermissionsExt, path::Path, sync::Arc, time::Duration};

use tokio::{
    io::{split, AsyncBufReadExt, BufReader}, net::UnixListener, time::interval
};

use crate::{
    core::{state::AppState, watcher::watch_once},
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
        let mut guard = state.watches.write().await;
        let mut dirty = false;

        for (id, ctx) in guard.iter_mut() {
            match watch_once(ctx).await {
                Ok(true) => {
                    println!("[{}] ✔ OK", id);
                    dirty = true;
                },
                Ok(false) => {
                    println!("[{}] ✔ No change", id);
                }
                Err(e) => eprintln!("[{}] ❌ Watch failed: {}", id, e),
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
