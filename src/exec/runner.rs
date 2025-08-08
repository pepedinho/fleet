#![allow(dead_code)]
use std::fs::OpenOptions;

use anyhow::Result;

use crate::{
    core::watcher::WatchContext,
    exec::command::{run_command_background, run_command_with_timeout},
    logging::Logger,
};

const DEFAULT_TIMEOUT: u64 = 300;

pub async fn run_update(ctx: &WatchContext) -> Result<(), anyhow::Error> {
    let logger = Logger::new(&ctx.log_path()).await?;

    logger.info("Update started").await?;
    let update_commands = &ctx.config.update;

    if update_commands.is_empty() {
        logger
            .warning("No command to execute (check your fleet.yml file)")
            .await?;
        return Ok(());
    }

    let default_timeout = ctx.config.timeout.unwrap_or(DEFAULT_TIMEOUT);

    for (i, cmd_line) in update_commands.iter().enumerate() {
        logger
            .info(&format!("Executing command {} : {}", i + 1, cmd_line.cmd))
            .await?;
        let parts = shell_words::split(&cmd_line.cmd)?;
        if parts.is_empty() {
            logger.info("Empty command, ignore ...").await?;
            continue;
        }

        let program = &parts[0];
        let args = &parts[1..];

        if cmd_line.blocking {
            //blocking command => run in background and forget
            logger
                .info("Command marked as blocking: running in background without waiting")
                .await?;
            //TODO: run in background
            let log_path = ctx.log_path();

            let stdout_file = OpenOptions::new()
                .create(true)
                .append(true)
                .write(true)
                .open(&log_path)?;
            let stderr_file = OpenOptions::new()
                .create(true)
                .append(true)
                .write(true)
                .open(&log_path)?;

            match run_command_background(
                program,
                &args.to_vec(),
                &ctx.project_dir,
                stdout_file,
                stderr_file,
            )
            .await
            {
                Ok(_child) => {
                    logger
                        .info(&format!("Background command launched",))
                        .await?;
                }
                Err(e) => {
                    logger
                        .error(&format!("Failed to launch background command: {}", e))
                        .await?;
                    return Err(e);
                }
            }

            logger.info("Background command launched").await?;
        } else {
            //classic command w timeout
            match run_command_with_timeout(program, args, &ctx.project_dir, default_timeout).await {
                Ok(output) => {
                    if output.status_code != Some(0) {
                        logger
                            .error(&format!(
                                "Command failed with exit code {:?}\nstdout:\n{}\nstderr:\n{}",
                                output.status_code, output.stdout, output.stderr
                            ))
                            .await?;
                        return Err(anyhow::anyhow!("Failed command: {}", cmd_line.cmd));
                    }
                    logger.info(&format!("stdout:\n{}", output.stdout)).await?;
                    logger.info(&format!("stderr:\n{}", output.stderr)).await?;
                    logger.info("Command succeeded").await?;
                }
                Err(e) => {
                    logger
                        .error(&format!("Command error or timeout: {}", e))
                        .await?;
                    return Err(e);
                }
            }
        }
    }

    logger.info("=== Update finished successfully ===").await?;

    Ok(())
}
