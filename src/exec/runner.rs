
use tokio::{fs::OpenOptions, io::AsyncWriteExt, process::Command};

use crate::{core::watcher::WatchContext, ipc::server::get_log_file};

pub async fn run_update(ctx: &WatchContext) -> Result<(), anyhow::Error> {
    let mut log_file = get_log_file(&ctx).await?;

    // En-tête de log
    let now = chrono::Local::now();
    log_file
        .write_all(format!("\n--- [{}] Update started ---\n", now).as_bytes())
        .await?;

    // Log du début
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

        let output = Command::new(program)
            .args(args)
            .current_dir(&ctx.project_dir)
            .output()
            .await?;

        log_file
            .write_all(format!("🔧 Command stdout:\n{}\n", String::from_utf8_lossy(&output.stdout)).as_bytes())
            .await?;

        log_file
            .write_all(format!("🧨 Command stderr:\n{}\n", String::from_utf8_lossy(&output.stderr)).as_bytes())
            .await?;

        if !output.status.success() {
            log_file
                .write_all(format!("❌ Command failed with code: {:?}\n", output.status.code()).as_bytes())
                .await?;

            return Err(anyhow::anyhow!(
                "Command '{}' failed with exit code: {:?}",
                command_line,
                output.status.code().unwrap_or(-1)
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
