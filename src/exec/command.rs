#![allow(dead_code)]
use std::time::Duration;

use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, BufReader},
    process::{Child, Command},
    time::timeout,
};

pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status_code: Option<i32>,
}

pub async fn run_command_capture_output(
    program: &str,
    args: &[String],
    current_dir: &str,
) -> Result<CommandOutput> {
    let output = Command::new(program)
        .args(args)
        .current_dir(current_dir)
        .output()
        .await?;

    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        status_code: output.status.code(),
    })
}

pub async fn run_command_with_timeout(
    program: &str,
    args: &[String],
    current_dir: &str,
    timeout_secs: u64,
) -> Result<CommandOutput> {
    // Lance le process avec pipes pour stdout et stderr
    let mut child = Command::new(program)
        .args(args)
        .current_dir(current_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

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

        anyhow::Ok((status, stdout_buf, stderr_buf))
    };

    match timeout(duration, run_future).await {
        Ok(Ok((status, stdout_data, stderr_data))) => Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&stdout_data).to_string(),
            stderr: String::from_utf8_lossy(&stderr_data).to_string(),
            status_code: status.code(),
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
) -> Result<Child> {
    use std::process::Stdio;
    let stdout_stdio = Stdio::from(stdout_file);
    let stderr_stdio = Stdio::from(stderr_file);

    let child = Command::new(program)
        .args(args)
        .current_dir(current_dir)
        .stdout(stdout_stdio)
        .stderr(stderr_stdio)
        .spawn()?;

    Ok(child)
}
