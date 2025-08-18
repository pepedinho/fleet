#![allow(dead_code)]
use std::collections::HashMap;

use anyhow::Result;

use crate::{
    core::watcher::WatchContext,
    exec::{
        command::{exec_background, exec_timeout},
        container::contain_cmd,
    },
    logging::Logger,
};

const DEFAULT_TIMEOUT: u64 = 300;

pub async fn run_update(ctx: &WatchContext) -> Result<(), anyhow::Error> {
    let logger = Logger::new(&ctx.log_path()).await?;

    logger.info("Update started").await?;
    let update_commands = &ctx.config.update.steps;

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

        let env = ctx.config.update.env.clone();
        let log_path = logger.get_path()?;

        if cmd_line.container.is_some() {
            let image = cmd_line.container.clone().unwrap();
            if cmd_line.blocking {
                //blocking command => run in background and forget
                // background_process(ctx, parts, &logger, env).await?;
                contain_cmd(
                    &image,
                    parts,
                    env,
                    &ctx.project_dir,
                    &log_path,
                    &logger,
                    None,
                )
                .await?;
            } else {
                //classic command w timeout
                contain_cmd(
                    &image,
                    parts,
                    env,
                    &ctx.project_dir,
                    &log_path,
                    &logger,
                    Some(default_timeout),
                )
                .await?;
                // timeout_process(ctx, parts, &logger, env, default_timeout).await?;
            }
        } else if cmd_line.blocking {
            background_process(ctx, parts, &logger, env).await?;
        } else {
            timeout_process(ctx, parts, &logger, env, default_timeout).await?;
        }
    }

    logger.info("=== Update finished successfully ===").await?;

    Ok(())
}

async fn background_process(
    ctx: &WatchContext,
    parts: Vec<String>,
    logger: &Logger,
    env: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    match exec_background(parts.clone(), ctx, logger, env).await {
        Ok(_) => {}
        Err(_e) if parts[0] == "git" && parts[1] == "pull" => {
            run_conflict_process(ctx).await?;
        }
        Err(e) => {
            logger.error(&format!("Failed: {e}")).await?;
        }
    };
    Ok(())
}

async fn timeout_process(
    ctx: &WatchContext,
    parts: Vec<String>,
    logger: &Logger,
    env: Option<HashMap<String, String>>,
    default_timeout: u64,
) -> Result<(), anyhow::Error> {
    match exec_timeout(parts.clone(), ctx, logger, default_timeout, env).await {
        Ok(_) => {}
        Err(_e) if parts[0] == "git" && parts[1] == "pull" => {
            run_conflict_process(ctx).await?;
        }
        Err(e) => {
            logger.error(&format!("Failed: {e}")).await?;
        }
    };
    Ok(())
}

pub async fn run_conflict_process(ctx: &WatchContext) -> Result<(), anyhow::Error> {
    let logger = Logger::new(&ctx.log_path()).await?;

    logger.info("Conflict process started").await?;
    let conflict_commands = &ctx.config.on_conflict.steps;

    if conflict_commands.is_empty() {
        logger
            .warning("No command to execute (check your fleet.yml file)")
            .await?;
        return Ok(());
    }

    let default_timeout = ctx.config.timeout.unwrap_or(DEFAULT_TIMEOUT);

    for (i, cmd_line) in conflict_commands.iter().enumerate() {
        logger
            .info(&format!("Executing command {} : {}", i + 1, cmd_line.cmd))
            .await?;
        let parts = shell_words::split(&cmd_line.cmd)?;
        if parts.is_empty() {
            logger.info("Empty command, ignore ...").await?;
            continue;
        }

        let env = ctx.config.update.env.clone();

        if cmd_line.blocking {
            //blocking command => run in background and forget
            exec_background(parts, ctx, &logger, env).await?;
        } else {
            //classic command w timeout
            exec_timeout(parts, ctx, &logger, default_timeout, env).await?;
        }
    }

    logger
        .info("=== Conflit process finished successfully ===")
        .await?;

    Ok(())
}
