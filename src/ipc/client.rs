use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

use crate::ipc::server::{DaemonRequest, DaemonResponse};

pub async fn send_watch_request(req: DaemonRequest) -> Result<(), anyhow::Error> {
    let mut stream = UnixStream::connect("/tmp/fleetd.sock")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect with daemon => {}", e))?;

    let json = serde_json::to_string(&req)? + "\n";
    stream.write_all(json.as_bytes()).await?;
    stream.flush().await?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();

    reader.read_line(&mut response_line).await?;

    let response: DaemonResponse = serde_json::from_str(response_line.trim())?;

    match response {
        DaemonResponse::Success(msg) => {
            println!("✅ {}", msg);
        }
        DaemonResponse::Error(e) => {
            eprintln!("❌ Error: {}", e);
        }
        DaemonResponse::ListWatches(r) => {
            println!("📋 Currently watching {} project(s):\n", r.len());

            // En-tête du tableau avec alignement par colonnes
            println!(
                "{:<15} {:<20} {:<12} {:<12} {:<40} {:<30}",
                "Project ID", "Name", "Branch", "Commit", "Remote URL", "Project Dir"
            );
            println!("{}", "-".repeat(130));
            for e in r {
                println!(
                    "{:<15} {:<20} {:<12} {:<12} {:<40} {:<30}",
                    e.id.to_string(),
                    e.repo_name,
                    e.branch,
                    e.short_commit,
                    e.short_url,
                    e.project_dir
                );
            }
        }
    }

    Ok(())
}
