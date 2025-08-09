#![allow(dead_code)]
use std::sync::Arc;

use crate::{
    config::parser::{ProjectConfig, UpdateCommand},
    core::{
        id::short_id,
        state::{AppState, add_watch, get_id_by_name, get_name_by_id, save_watches},
        watcher::WatchContext,
    },
    git::repo::Repo,
    logging::Logger,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt, BufReader, WriteHalf},
    net::UnixStream,
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "action")]
pub enum DaemonRequest {
    #[serde(rename = "add_watch")]
    AddWatch {
        project_dir: String,
        branch: String,
        repo: Repo,
        update_cmds: Vec<UpdateCommand>,
    },

    #[serde(rename = "stop_watch")]
    StopWatch { id: String },

    #[serde(rename = "list_watch")]
    ListWatches,

    #[serde(rename = "logs_watch")]
    LogsWatches { id: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WatchInfo {
    pub branch: String,
    pub project_dir: String,
    pub short_commit: String,
    pub short_url: String,
    pub repo_name: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DaemonResponse {
    Success(String),
    Error(String),
    ListWatches(Vec<WatchInfo>),
}

pub async fn get_log_file(ctx: &WatchContext) -> Result<File> {
    let log_path = ctx.log_path();
    println!("log path => {}", log_path.to_str().unwrap());
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .await?;
    Ok(log_file)
}

async fn get_logs_by_id(id: &str) -> Result<String> {
    let log_path = WatchContext::log_path_by_id(id);

    let file = File::open(&log_path).await?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents).await?;
    Ok(contents)
}

/// Handles a daemon request and sends the resulting [`DaemonResponse`] back to the client.
/// All errors inside handlers are mapped to `DaemonResponse::Error`.
pub async fn handle_request(
    req: DaemonRequest,
    state: Arc<AppState>,
    stream: &mut WriteHalf<UnixStream>,
) -> Result<(), anyhow::Error> {
    println!("treat the request");

    let response = match req {
        DaemonRequest::AddWatch {
            project_dir,
            branch,
            repo,
            update_cmds,
        } => handle_add_watch(state, project_dir, branch, repo, update_cmds).await,

        DaemonRequest::StopWatch { id } => handle_stop_watch(state, id).await,

        DaemonRequest::ListWatches => handle_list_watches(state).await,

        DaemonRequest::LogsWatches { id } => handle_logs_watches(id).await,
    };

    send_response(stream, response).await?;
    Ok(())
}

/// Registers a new watch, updates the application state, and returns a response.
/// Any error will be converted to `DaemonResponse::Error`.
async fn handle_add_watch(
    state: Arc<AppState>,
    project_dir: String,
    branch: String,
    repo: Repo,
    update_cmds: Vec<UpdateCommand>,
) -> DaemonResponse {
    let id = short_id();
    let ctx = WatchContext {
        branch,
        repo,
        config: ProjectConfig {
            update: update_cmds,
            ..Default::default()
        },
        project_dir,
        id: id.clone(),
    };

    let result = async {
        let logger = Logger::new(&ctx.log_path()).await?;
        {
            let mut guard = state.watches.write().await;
            add_watch(&ctx).await?;
            guard.insert(id.clone(), ctx);
        }
        logger
            .info(&format!("Project registered with ID : {}", &id))
            .await?;
        Ok::<_, anyhow::Error>(())
    }
    .await;

    match result {
        Ok(_) => DaemonResponse::Success(format!("📌 Project registered with ID: {}", id)),
        Err(e) => DaemonResponse::Error(format!("Failed to add watch: {}", e)),
    }
}

/// Stops a watch by ID if it exists in the application state.
async fn handle_stop_watch(state: Arc<AppState>, id: String) -> DaemonResponse {
    match async {
        let mut guard = state.watches.write().await;
        if guard.remove(&id).is_some() {
            Ok(format!("🛑 Watch stopped for ID: {}", id))
        } else {
            Err(format!("⚠ ID not found: {}", id))
        }
    }
    .await
    {
        Ok(msg) => DaemonResponse::Success(msg),
        Err(e) => DaemonResponse::Error(format!("Failed to stop watch: {}", e)),
    }
}

/// Returns a list of all current watches as a [`DaemonResponse::ListWatches`].
async fn handle_list_watches(state: Arc<AppState>) -> DaemonResponse {
    match async {
        let guard = state.watches.read().await;
        let result = guard
            .iter()
            .map(|(id, ctx)| {
                let short_commit = ctx.repo.last_commit.chars().take(8).collect::<String>();
                let short_url = if ctx.repo.remote.len() > 40 {
                    format!("{}...", &ctx.repo.remote[..37])
                } else {
                    ctx.repo.remote.clone()
                };
                WatchInfo {
                    branch: ctx.repo.branch.clone(),
                    project_dir: ctx.project_dir.clone(),
                    short_commit,
                    short_url,
                    repo_name: ctx.repo.name.clone(),
                    id: id.clone(),
                }
            })
            .collect();
        Ok::<_, anyhow::Error>(DaemonResponse::ListWatches(result))
    }
    .await
    {
        Ok(resp) => resp,
        Err(e) => DaemonResponse::Error(format!("Failed to list watches: {}", e)),
    }
}

/// Fetches logs for a given watch by ID or name.
/// If the watch is not found, sends an error directly to the client.
/// Returns `None` if an error was already sent to the stream.
async fn handle_logs_watches(id: String) -> DaemonResponse {
    match async {
        let id = match get_name_by_id(&id).await {
            Ok(Some(_)) => id,
            Err(_) | Ok(None) => match get_id_by_name(&id).await? {
                Some(uuid) => uuid,
                None => anyhow::bail!("No repo with this name exists"),
            },
        };
        let logs = get_logs_by_id(&id).await?;
        Ok::<_, anyhow::Error>(DaemonResponse::Success(logs))
    }
    .await
    {
        Ok(resp) => resp,
        Err(e) => DaemonResponse::Error(format!("Failed to fetch logs: {}", e)),
    }
}

async fn send_response(
    stream: &mut WriteHalf<UnixStream>,
    response: DaemonResponse,
) -> Result<(), anyhow::Error> {
    let response_str = serde_json::to_string(&response)? + "\n";
    stream.write_all(response_str.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

async fn send_error_response(
    stream: &mut WriteHalf<UnixStream>,
    message: &str,
) -> Result<(), anyhow::Error> {
    let response = DaemonResponse::Error(message.to_string());
    let response_str = serde_json::to_string(&response)? + "\n";
    stream.write_all(response_str.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}
