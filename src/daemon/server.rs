#![allow(dead_code)]
use std::sync::Arc;

use crate::{
    config::ProjectConfig,
    core::{
        id::short_id,
        manager::get_watch_ctx,
        state::{AppState, get_id_by_name, get_name_by_id},
        watcher::{WatchContext, WatchContextBuilder},
    },
    daemon::utiles::extract_repo_path,
    exec::metrics::ExecMetrics,
    exec::pipeline::run_pipeline,
    git::repo::Repo,
    log::logger::Logger,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, WriteHalf},
    net::UnixStream,
};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "action")]
pub enum DaemonRequest {
    #[serde(rename = "add_watch")]
    AddWatch {
        project_dir: String,
        branches: Vec<String>,
        // use Box (clippy)
        repo: Box<Repo>,
        config: Box<ProjectConfig>,
    },

    #[serde(rename = "run_pipeline")]
    RunPipeline {
        id: String,
    },

    #[serde(rename = "stop_watch")]
    StopWatch {
        id: String,
    },

    #[serde(rename = "up_watch")]
    UpWatch {
        id: String,
    },

    #[serde(rename = "rm_watch")]
    RmWatch {
        id: String,
    },

    #[serde(rename = "list_watch")]
    ListWatches {
        all: bool,
    },

    #[serde(rename = "logs_watch")]
    LogsWatches {
        id: String,
        f: bool,
    },

    None,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct WatchInfo {
    pub branch: String,
    pub project_dir: String,
    pub short_commit: String,
    pub short_url: String,
    pub repo_name: String,
    pub id: String,
    pub paused: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum DaemonResponse {
    Success(String),
    Error(String),
    ListWatches(Vec<WatchInfo>),
    LogWatch(String, bool),
    Ignore,
    None,
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

    // let file = File::open(&log_path).await?;
    // let mut reader = BufReader::new(file);
    // let mut contents = String::new();
    // reader.read_to_string(&mut contents).await?;
    match log_path.to_str() {
        Some(p) => Ok(String::from(p)),
        None => Err(anyhow::anyhow!("Failed to find log path")),
    }
}

/// Handles a daemon request and sends the resulting [`DaemonResponse`] back to the client.
/// All errors inside handlers are mapped to `DaemonResponse::Error`.
pub async fn handle_request(
    req: DaemonRequest,
    state: Arc<AppState>,
    stream: &mut WriteHalf<UnixStream>,
) -> Result<(), anyhow::Error> {
    let response = match req {
        DaemonRequest::AddWatch {
            project_dir,
            branch,
            repo,
            config,
        } => handle_add_watch(state, project_dir, branch, *repo, *config).await?,

        DaemonRequest::StopWatch { id } => handle_stop_watch(state, id).await,

        DaemonRequest::UpWatch { id } => handle_up_watch(state, id).await,

        DaemonRequest::RmWatch { id } => handle_rm_watch(state, id).await,

        DaemonRequest::ListWatches { all } => handle_list_watches(state, all).await,

        DaemonRequest::LogsWatches { id, f } => handle_logs_watches(id, f).await,

        DaemonRequest::RunPipeline { id } => {
            handle_run_pipeline(&id, state, stream).await?;
            DaemonResponse::Ignore
        }
        DaemonRequest::None => DaemonResponse::None,
    };

    if response != DaemonResponse::Ignore {
        send_response(stream, response).await?;
    }
    Ok(())
}

async fn handle_run_pipeline(
    id: &str,
    state: Arc<AppState>,
    stream: &mut WriteHalf<UnixStream>,
) -> anyhow::Result<()> {
    if let Some(ctx) = get_watch_ctx(&state, id).await {
        send_response(
            stream,
            DaemonResponse::Success(format!("Pipeline {id} has been runed")),
        )
        .await?;
        match run_pipeline(Arc::new(ctx)).await {
            Ok(_) => {
                println!("[{id}] ✅ Update succeeded");
            }
            Err(e) => {
                eprintln!("[{id}] ❌ Update failed => {e}");
            }
        }
    }
    Ok(())
}

/// Registers a new watch, updates the application state, and returns a response.
/// Any error will be converted to `DaemonResponse::Error`.
async fn handle_add_watch(
    state: Arc<AppState>,
    project_dir: String,
    branch: String,
    mut repo: Repo,
    config: ProjectConfig,
) -> anyhow::Result<DaemonResponse> {
    let mut guard = state.watches.write().await;
    let existing_id = guard
        .iter()
        .find(|(_, ctx)| ctx.project_dir == project_dir)
        .map(|(id, ctx)| {
            if ctx.repo.branch != branch {
                repo.last_commit = ctx.repo.last_commit.clone();
            }
            id.clone()
        });

    let id = existing_id.unwrap_or_else(short_id);

    let ctx = WatchContextBuilder::new(branch, repo, config, project_dir, id.clone())
        .build()
        .await?;

    let result = async {
        let logger = Logger::new(&ctx.log_path()).await?;
        {
            // delete the projects with the same project_dir, before saving the new one
            guard.retain(|_, existing_ctx| existing_ctx.project_dir != ctx.project_dir);
            AppState::add_watch(&ctx).await?;
            guard.insert(id.clone(), ctx);
        }
        logger
            .info(&format!("Project registered with ID : {}", &id))
            .await?;
        Ok::<_, anyhow::Error>(())
    }
    .await;

    match result {
        Ok(_) => Ok(DaemonResponse::Success(format!(
            "📌 Project registered with ID: {id}"
        ))),
        Err(e) => Ok(DaemonResponse::Error(format!("Failed to add watch: {e}"))),
    }
}

/// Stops a watch by ID if it exists in the application state.
async fn handle_stop_watch(state: Arc<AppState>, id: String) -> DaemonResponse {
    match async {
        let mut guard = state.watches.write().await;
        if let Some(w) = guard.get_mut(&id) {
            w.stop();
            AppState::add_watch(w).await?;
            Ok::<_, anyhow::Error>(format!("🛑 Watch stopped for ID: {id}"))
        } else {
            Err(anyhow::anyhow!("⚠ ID not found: {}", id))
        }
    }
    .await
    {
        Ok(msg) => DaemonResponse::Success(msg),
        Err(e) => DaemonResponse::Error(format!("Failed to stop watch: {e}")),
    }
}

/// Run a watch by ID if it exists in the application state.
pub async fn handle_up_watch(state: Arc<AppState>, id: String) -> DaemonResponse {
    match async {
        let mut guard = state.watches.write().await;
        if let Some(w) = guard.get_mut(&id) {
            w.run();
            AppState::add_watch(w).await?;
            Ok::<_, anyhow::Error>(format!("🟢 Watch up for ID: {id}"))
        } else {
            Err(anyhow::anyhow!("⚠ ID not found: {}", id))
        }
    }
    .await
    {
        Ok(msg) => DaemonResponse::Success(msg),
        Err(e) => DaemonResponse::Error(format!("Failed to stop watch: {e}")),
    }
}

/// Rm a watch by ID if it exists in the application state.
pub async fn handle_rm_watch(state: Arc<AppState>, id: String) -> DaemonResponse {
    match async {
        let mut guard = state.watches.write().await;
        if let Some(w) = guard.remove(&id) {
            ExecMetrics::rm_metrics_by_id(&id)?; // remove metrics file
            Logger::rm_logs_by_id(&id)?; // remove log file 
            AppState::remove_watch_by_id(&id).await?; // remove this watch in watches.json
            Ok::<_, anyhow::Error>(format!("Project: {} was deleted", w.repo.name))
        } else {
            Err(anyhow::anyhow!("⚠ ID not found: {}", id))
        }
    }
    .await
    {
        Ok(msg) => DaemonResponse::Success(msg),
        Err(e) => DaemonResponse::Error(format!("Failed to stop watch: {e}")),
    }
}

/// Returns a list of all current watches as a [`DaemonResponse::ListWatches`].
pub async fn handle_list_watches(state: Arc<AppState>, all: bool) -> DaemonResponse {
    match async {
        let guard = state.watches.read().await;
        let result: Result<Vec<WatchInfo>, anyhow::Error> = guard
            .iter()
            .filter(|(_, ctx)| all || !ctx.paused) // if all = true everything pass, else only if paused is false they can pass
            .map(|(id, ctx)| {
                let short_commit = ctx.repo.last_commit.chars().take(8).collect::<String>();
                let short_url = extract_repo_path(&ctx.repo.remote)?;
                let short_branch = if ctx.branch.len() > 12 {
                    format!("{}...", &ctx.branch[..9])
                } else {
                    ctx.branch.clone()
                };
                Ok(WatchInfo {
                    branch: short_branch,
                    project_dir: ctx.project_dir.clone(),
                    short_commit,
                    short_url,
                    repo_name: ctx.repo.name.clone(),
                    id: id.clone(),
                    paused: ctx.paused,
                })
            })
            .collect();
        Ok::<_, anyhow::Error>(DaemonResponse::ListWatches(result?))
    }
    .await
    {
        Ok(resp) => resp,
        Err(e) => DaemonResponse::Error(format!("Failed to list watches: {e}")),
    }
}

/// Fetches logs for a given watch by ID or name.
/// If the watch is not found, sends an error directly to the client.
/// Returns `None` if an error was already sent to the stream.
async fn handle_logs_watches(id: String, follow: bool) -> DaemonResponse {
    match async {
        let id = match get_name_by_id(&id).await {
            Ok(Some(_)) => id,
            Err(_) | Ok(None) => match get_id_by_name(&id).await? {
                Some(uuid) => uuid,
                None => anyhow::bail!("No repo with this name exists"),
            },
        };
        let logs = get_logs_by_id(&id).await?;
        Ok::<_, anyhow::Error>(DaemonResponse::LogWatch(logs, follow))
    }
    .await
    {
        Ok(resp) => resp,
        Err(e) => DaemonResponse::Error(format!("Failed to fetch logs: {e}")),
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
