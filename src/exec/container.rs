use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use bollard::Docker;
use bollard::models::ContainerCreateBody;
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, CreateImageOptionsBuilder, LogsOptionsBuilder,
    RemoveContainerOptionsBuilder, StartContainerOptions,
};
use futures_util::stream::StreamExt;

use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;

use crate::core::id::short_id;
use crate::logging::Logger;

async fn ensure_image(docker: &Docker, image: &str, logger: &Logger) -> Result<()> {
    let image_options = CreateImageOptionsBuilder::default()
        .from_image(image)
        .build();
    let mut stream = docker.create_image(Some(image_options), None, None);

    let mut last = String::new();
    while let Some(status) = stream.next().await {
        match status {
            Ok(info) => {
                if let Some(s) = info.status {
                    if last != s {
                        logger.info(&format!("Pulling {image}: {s}")).await?;
                    }
                    last = s.clone();
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to pull image {image}: {e}"));
            }
        }
    }
    Ok(())
}

pub async fn contain_cmd(
    image: &str,
    cmd: Vec<String>,
    env: Option<HashMap<String, String>>,
    dir: &str,
    log_path: &str,
    logger: &Logger,
    timeout_secs: Option<u64>,
) -> Result<()> {
    logger.info("Building image").await?;
    let docker = Docker::connect_with_local_defaults()?;
    ensure_image(&docker, image, logger).await?;

    let create_option = CreateContainerOptionsBuilder::default()
        .name(&format!("fleet-job-{}", short_id()))
        .build();

    let container_config = ContainerCreateBody {
        image: Some(image.to_string()),
        cmd: Some(cmd),
        env: env.map(|m| m.iter().map(|(k, v)| format!("{k}={v}")).collect()),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        tty: Some(false),
        host_config: Some(bollard::models::HostConfig {
            binds: Some(vec![format!("{dir}:/app",)]),
            ..Default::default()
        }),
        working_dir: Some("/app".to_string()),
        ..Default::default()
    };

    let container = docker
        .create_container(Some(create_option), container_config)
        .await?;

    docker
        .start_container(&container.id, None::<StartContainerOptions>)
        .await?;

    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .await?;

    let logs_options = LogsOptionsBuilder::default()
        .follow(true)
        .stdout(true)
        .stderr(true)
        .build();

    let mut log_stream = docker.logs(&container.id, Some(logs_options));

    let logs_future = async {
        while let Some(log) = log_stream.next().await {
            match log? {
                bollard::container::LogOutput::StdOut { message }
                | bollard::container::LogOutput::StdErr { message } => {
                    log_file.write_all(&message).await?;
                }
                _ => {}
            }
        }
        Ok::<(), anyhow::Error>(())
    };

    let result = if let Some(secs) = timeout_secs {
        // w Timeout
        match timeout(Duration::from_secs(secs), logs_future).await {
            Ok(inner) => inner,
            Err(_) => {
                logger
                    .error(&format!(
                        "Container execution timed out after {secs} seconds"
                    ))
                    .await?;
                Err(anyhow::anyhow!(
                    "Container execution timed out after {secs} seconds"
                ))
            }
        }
    } else {
        // w no timemout
        logs_future.await
    };

    let remove_options = RemoveContainerOptionsBuilder::default().force(true).build();
    docker
        .remove_container(&container.id, Some(remove_options))
        .await?;
    result
}
