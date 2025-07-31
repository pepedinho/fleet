use tokio::{io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader}, net::UnixStream};

use crate::ipc::server::{DaemonRequest, DaemonResponse};



pub async fn send_watch_request(req: DaemonRequest) -> Result<(), anyhow::Error> {
    let mut stream = UnixStream::connect("/tmp/fleetd.sock").await?;

    let json= serde_json::to_string(&req)? + "\n";
    println!("start to write");
    stream.write_all(json.as_bytes()).await?;
    stream.flush().await?;

    println!("finish to write !");

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();

    println!("start to read");
    reader.read_line(&mut response_line).await?;
    println!("finish to read !");

    let response: DaemonResponse = serde_json::from_str(&response_line.trim())?;

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
                "{:<20} {:<12} {:<8} {:<40} {:<30}",
                "Project ID", "Branch", "Commit", "Remote URL", "Project Dir"
            );
            println!("{}", "-".repeat(115));
            for e in r {
                println!(
                    "{:<10} {:<20} {:<12} {:<8} {:<40} {:<30}",
                    e.id, e.repo_name, e.branch, e.short_commit, e.short_url, e.project_dir
                );
            }
        }
    }

    Ok(())
}