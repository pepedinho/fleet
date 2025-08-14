use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

use crate::ipc::server::{DaemonRequest, DaemonResponse, WatchInfo};

pub async fn send_watch_request(req: DaemonRequest) -> Result<(), anyhow::Error> {
    let mut stream = UnixStream::connect("/tmp/fleetd.sock")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect with daemon => {}", e))?;

    let json = serde_json::to_string(&req)? + "\n";
    stream.write_all(json.as_bytes()).await?;
    stream.flush().await?;

    let response = read_daemon_response(stream).await?;
    handle_daemon_response(response);

    Ok(())
}

/// Reads a single line response from the daemon and deserializes it into a [`DaemonResponse`].
async fn read_daemon_response(stream: UnixStream) -> Result<DaemonResponse, anyhow::Error> {
    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;
    let response = serde_json::from_str(response_line.trim())?;
    Ok(response)
}

/// Processes the [`DaemonResponse`] by printing success, error,
/// or listing watches in a formatted table.
fn handle_daemon_response(response: DaemonResponse) {
    match response {
        DaemonResponse::Success(msg) => {
            println!("✅ {msg}");
        }
        DaemonResponse::Error(e) => {
            eprintln!("❌ Error: {e}");
        }
        DaemonResponse::ListWatches(watches) => {
            print_watches_table(&watches);
        }
    }
}

/// Prints a formatted table of active watches.
fn print_watches_table(watches: &[WatchInfo]) {
    println!(
        "{:<13} {:<10} {:<13} {:<12} {:<20} {:<30}",
        "PROJECT ID", "NAME", "BRANCH", "COMMIT", "REMOTE URL", "DIR"
    );
    for w in watches {
        println!(
            "{:<13} {:<10} {:<13} {:<12} {:<20} {:<30}",
            w.id.to_string(),
            w.repo_name,
            w.branch,
            w.short_commit,
            w.short_url,
            w.project_dir
        );
    }
}
