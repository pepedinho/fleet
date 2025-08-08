
use std::process::Stdio;

use anyhow::Result;
use tokio::{fs::OpenOptions, io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, process::Command, task::JoinHandle};

use crate::{core::watcher::WatchContext, ipc::server::get_log_file};

pub async fn run_update(ctx: &WatchContext) -> Result<(), anyhow::Error> {
    let mut log_file = get_log_file(&ctx).await?;

    let now = chrono::Local::now();
    log_file
        .write_all(format!("\n--- [{}] Update started ---\n", now).as_bytes())
        .await?;
    log_file.write_all(format!("▶ Update project...\n").as_bytes()).await?;

    for (i, command_line) in ctx.config.update.iter().enumerate() {
        log_file
            .write_all(format!("➡️  [Cmd {}] {}\n", i + 1, command_line).as_bytes())
            .await?;

        let parts: Vec<String> = shell_words::split(command_line).map_err(|e| {
            anyhow::anyhow!("Erreur lors du parsing de '{}': {}", command_line, e)
        })?;

        if parts.is_empty() {
            log_file.write_all(format!("⚠️  Ligne de commande vide, ignorée.\n").as_bytes()).await?;
            continue;
        }

        let program = &parts[0];
        let args = &parts[1..];

        let mut child = Command::new(program)
            .args(args)
            .current_dir(&ctx.project_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Error running command: '{}': {}", command_line, e))?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut log_out = log_file.try_clone().await?;
        let mut log_err = log_file.try_clone().await?;

        let handle_stdout: JoinHandle<Result<()>> = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await? {
                log_out
                    .write_all(format!("🔧 {}\n", line).as_bytes())
                    .await?;
            }
            Ok(())
        });

        let handle_stderr: JoinHandle<Result<()>> = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await? {
                log_err
                    .write_all(format!("🧨 {}\n", line).as_bytes())
                    .await?;
            }
            Ok(())
        });

        let status = child.wait().await?;
        handle_stdout.await??;
        handle_stderr.await??;

        if !status.success() {
            log_file
                .write_all(
                    format!(
                        "❌ Command failed with exit code : {:?}\n",
                        status.code()
                    )
                    .as_bytes(),
                )
                .await?;
            return Err(anyhow::anyhow!(
                "Command '{}' failed with the exit code : {:?}",
                command_line,
                status.code()
            ));
        }

        log_file
            .write_all(format!("✅ Command succeeded.\n").as_bytes())
            .await?;
    }

    log_file.write_all(format!("✅ Update done !\n").as_bytes()).await?;

    let now = chrono::Local::now();
    log_file
        .write_all(format!("--- [{}] Update finished ---\n\n", now).as_bytes())
        .await?;

    Ok(())
}
