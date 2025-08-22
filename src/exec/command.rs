#![allow(dead_code)]
use std::{collections::HashMap, fs::OpenOptions, time::Duration};

use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, BufReader},
    process::{Child, Command},
    time::timeout,
};

use crate::{core::watcher::WatchContext, exec::metrics::monitor_process, logging::Logger};

pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status_code: Option<i32>,
    pub cpu_usage: f32,
    pub mem_usage_kb: u64,
}

pub async fn run_command_with_timeout(
    program: &str,
    args: &[String],
    current_dir: &str,
    timeout_secs: u64,
    env: Option<HashMap<String, String>>,
) -> Result<CommandOutput> {
    // Lance le process avec pipes pour stdout et stderr
    let mut cmd = Command::new(program);

    cmd.args(args)
        .current_dir(current_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

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
        println!("METRICS FOUND => {:#?}", metrics);
        let _ = tx.send(metrics).await;
    });

    let stdout = child.stdout.take().expect("stdout should be capture");
    let stderr = child.stderr.take().expect("stderr should be capture");

    let mut stdout_reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);

    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();

    let duration = Duration::from_secs(timeout_secs);

    let run_future = async {
        let stdout_fut = async {
            stdout_reader.read_to_end(&mut stdout_buf).await?;
            Result::<(), std::io::Error>::Ok(())
        };
        let stderr_fut = async {
            stderr_reader.read_to_end(&mut stderr_buf).await?;
            Result::<(), std::io::Error>::Ok(())
        };

        tokio::try_join!(stdout_fut, stderr_fut)?;

        let status = child.wait().await?;

        let (cpu_usage, mem_usage_kb) = rx.recv().await.unwrap_or((0.0, 0));
        println!("METRICS EXTRACTED => {} | {}", cpu_usage, mem_usage_kb);
        anyhow::Ok((status, stdout_buf, stderr_buf, cpu_usage, mem_usage_kb))
    };

    match timeout(duration, run_future).await {
        Ok(Ok((status, stdout_data, stderr_data, cpu_usage, mem_usage_kb))) => Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&stdout_data).to_string(),
            stderr: String::from_utf8_lossy(&stderr_data).to_string(),
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
    match run_command_with_timeout(program, args, &ctx.project_dir, timeout, env).await {
        Ok(output) => {
            if output.status_code != Some(0) {
                logger
                    .error(&format!(
                        "Command failed with exit code {:?}\nstdout:\n{}\nstderr:\n{}",
                        output.status_code, output.stdout, output.stderr
                    ))
                    .await?;
                return Err(anyhow::anyhow!("Failed command: {:?}", parts));
            }
            logger.info(&format!("stdout:\n{}", output.stdout)).await?;
            logger.info(&format!("stderr:\n{}", output.stderr)).await?;
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
