#![allow(dead_code)]
use std::{collections::HashMap, fs::OpenOptions, time::Duration};

use anyhow::Result;
use tokio::{
    process::{Child, Command},
    time::timeout,
};

use crate::{core::watcher::WatchContext, exec::metrics::monitor_process, logging::Logger};

pub struct CommandOutput {
    pub status_code: Option<i32>,
    pub cpu_usage: f32,
    pub mem_usage_kb: u64,
}

pub async fn run_command_with_timeout(
    program: &str,
    args: &[String],
    current_dir: &str,
    timeout_secs: u64,
    stdout_file: std::fs::File,
    stderr_file: std::fs::File,
    env: Option<HashMap<String, String>>,
) -> Result<CommandOutput> {
    use std::process::Stdio;

    // Lance le process avec pipes pour stdout et stderr
    let mut cmd = Command::new(program);

    let stdout_stdio = Stdio::from(stdout_file);
    let stderr_stdio = Stdio::from(stderr_file);
    cmd.args(args)
        .current_dir(current_dir)
        .stdout(stdout_stdio)
        .stderr(stderr_stdio);

    if let Some(vars) = env {
        for (k, v) in vars {
            cmd.env(k, v);
        }
    }
    let mut child = cmd.spawn()?;
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let child_pid = child.id();
    tokio::spawn(async move {
        let metrics = monitor_process(child_pid.unwrap_or(1)).await;
        let _ = tx.send(metrics).await;
    });

    let duration = Duration::from_secs(timeout_secs);

    let run_future = async {
        let status = child.wait().await?;

        let (cpu_usage, mem_usage_kb) = rx.recv().await.unwrap_or((0.0, 0));
        println!("METRICS EXTRACTED => {} | {}", cpu_usage, mem_usage_kb);
        anyhow::Ok((status, cpu_usage, mem_usage_kb))
    };

    match timeout(duration, run_future).await {
        Ok(Ok((status, cpu_usage, mem_usage_kb))) => Ok(CommandOutput {
            status_code: status.code(),
            cpu_usage,
            mem_usage_kb,
        }),
        Ok(Err(e)) => Err(anyhow::anyhow!("Error during execution : {}", e)),
        Err(_) => {
            child.kill().await.ok();
            Err(anyhow::anyhow!(
                "Command timeout after {} seconds, process killed",
                timeout_secs
            ))
        }
    }
}

pub async fn run_command_background(
    program: &str,
    args: &[String],
    current_dir: &str,
    stdout_file: std::fs::File,
    stderr_file: std::fs::File,
    env: Option<HashMap<String, String>>,
) -> Result<Child> {
    use std::process::Stdio;
    let stdout_stdio = Stdio::from(stdout_file);
    let stderr_stdio = Stdio::from(stderr_file);

    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(current_dir)
        .stdout(stdout_stdio)
        .stderr(stderr_stdio);

    if let Some(vars) = env {
        for (k, v) in vars {
            cmd.env(k, v);
        }
    }

    let child = cmd.spawn()?;

    Ok(child)
}

pub async fn exec_timeout(
    parts: Vec<String>,
    ctx: &WatchContext,
    logger: &Logger,
    timeout: u64,
    env: Option<HashMap<String, String>>,
) -> Result<CommandOutput, anyhow::Error> {
    let program = &parts[0];
    let args = &parts[1..];

    let log_path = ctx.log_path();

    let stdout_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let stderr_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    match run_command_with_timeout(
        program,
        args,
        &ctx.project_dir,
        timeout,
        stdout_file,
        stderr_file,
        env,
    )
    .await
    {
        Ok(output) => {
            if output.status_code != Some(0) {
                logger
                    .error(&format!(
                        "Command failed with exit code {:?}",
                        output.status_code
                    ))
                    .await?;
                return Err(anyhow::anyhow!("Failed command: {:?}", parts));
            }
            logger.info("Command succeeded").await?;
            Ok(output)
        }
        Err(e) => {
            logger
                .error(&format!("Command error or timeout: {parts:?}"))
                .await?;
            Err(e)
        }
    }
}

pub async fn exec_background(
    parts: Vec<String>,
    ctx: &WatchContext,
    logger: &Logger,
    env: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    let program = &parts[0];
    let args = &parts[1..];
    logger
        .info("Command marked as blocking: running in background without waiting")
        .await?;
    let log_path = ctx.log_path();

    let stdout_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let stderr_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    match run_command_background(
        program,
        args,
        &ctx.project_dir,
        stdout_file,
        stderr_file,
        env,
    )
    .await
    {
        Ok(_child) => {
            logger.info("Background command launched").await?;
        }
        Err(e) => {
            logger
                .error(&format!("Failed to launch background command: {e}"))
                .await?;
            return Err(e);
        }
    }

    logger.info("Background command launched").await?;
    Ok(())
}
