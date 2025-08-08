
use std::{fs::{File, OpenOptions}, os::fd::AsFd, process::Stdio};

use anyhow::Result;
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, process::Command, task::JoinHandle};

use crate::{core::watcher::WatchContext, ipc::server::get_log_file};

pub async fn run_update(ctx: &WatchContext) -> Result<(), anyhow::Error> {
    let mut log_file = get_log_file(&ctx).await?;
    let now = chrono::Local::now();

    log_file
        .write_all(format!("\n--- [{}] Update started ---\n", now).as_bytes())
        .await?;
    log_file
        .write_all(format!("▶ Update project...\n").as_bytes())
        .await?;

    let update_commands = &ctx.config.update;
    if update_commands.is_empty() {
        log_file
            .write_all(format!("⚠️  Aucune commande à exécuter.\n").as_bytes())
            .await?;
        return Ok(());
    }

    // Exécuter toutes les commandes sauf la dernière de façon bloquante
    for (i, command_line) in update_commands.iter().take(update_commands.len() - 1).enumerate() {
        log_file
            .write_all(format!("➡️  [Cmd {}] {}\n", i + 1, command_line).as_bytes())
            .await?;

        // Parse la commande
        let parts = shell_words::split(command_line).map_err(|e| {
            anyhow::anyhow!("Erreur lors du parsing de '{}': {}", command_line, e)
        })?;

        if parts.is_empty() {
            log_file
                .write_all(format!("⚠️  Ligne de commande vide, ignorée.\n").as_bytes())
                .await?;
            continue;
        }

        let program = &parts[0];
        let args = &parts[1..];

        // Lance la commande et attends la fin
        let status = Command::new(program)
            .args(args)
            .current_dir(&ctx.project_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| anyhow::anyhow!("Erreur exécution '{}': {}", command_line, e))?;

        if !status.success() {
            log_file
                .write_all(
                    format!("❌ Command failed with exit code: {:?}\n", status.code()).as_bytes(),
                )
                .await?;
            return Err(anyhow::anyhow!(
                "Commande '{}' échouée avec code : {:?}",
                command_line,
                status.code()
            ));
        }

        log_file
            .write_all(format!("✅ Command succeeded.\n").as_bytes())
            .await?;
    }

    // Lancer la dernière commande en mode non-bloquant (background)
    let last_cmd_line = update_commands.last().unwrap();
    log_file
        .write_all(format!("➡️  [Cmd final (non-bloquant)] {}\n", last_cmd_line).as_bytes())
        .await?;

    let parts = shell_words::split(last_cmd_line).map_err(|e| {
        anyhow::anyhow!("Erreur lors du parsing de '{}': {}", last_cmd_line, e)
    })?;

    if parts.is_empty() {
        log_file
            .write_all(format!("⚠️  Ligne de commande finale vide, ignorée.\n").as_bytes())
            .await?;
    } else {
        let program = &parts[0];
        let args = &parts[1..];

        let std_log_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(ctx.log_path())
            .map_err(|e| anyhow::anyhow!("Impossible d'ouvrir le fichier de log : {}", e))?;

        let stdio = Stdio::from(std_log_file);

        // Lance la commande sans attendre sa fin
        let _child = Command::new(program)
            .args(args)
            .current_dir(&ctx.project_dir)
            .stdout(stdio) // Ou rediriger vers fichier/log selon besoin
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Erreur lancement commande finale '{}': {}", last_cmd_line, e))?;

        log_file
            .write_all(format!("▶ Commande finale lancée en background.\n").as_bytes())
            .await?;
    }

    log_file
        .write_all(format!("✅ Update done !\n").as_bytes())
        .await?;

    let now = chrono::Local::now();
    log_file
        .write_all(format!("--- [{}] Update finished ---\n\n", now).as_bytes())
        .await?;

    Ok(())
}
