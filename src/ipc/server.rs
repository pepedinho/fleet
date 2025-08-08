use std::{path::PathBuf, str::FromStr, sync::Arc};

use crate::{
    config::parser::{ProjectConfig, UpdateCommand},
    core::{
        id::short_id,
        state::{AppState, add_watch, get_id_by_name, get_name_by_id},
        watcher::WatchContext,
    },
    git::repo::Repo,
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
        } => {
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
            let mut log_file = get_log_file(&ctx).await?;
            let mut guard = state.watches.write().await;
            add_watch(&ctx).await?;
            guard.insert(id.clone(), ctx);
            log_file
                .write_all(format!("📌 Project registered with ID: {}", &id).as_bytes())
                .await?;
            DaemonResponse::Success(format!("📌 Project registered with ID: {}", id))
        }
        DaemonRequest::StopWatch { id } => {
            let mut guard = state.watches.write().await;
            if guard.remove(&id).is_some() {
                DaemonResponse::Success(format!("🛑 Watch stopped for ID: {}", id))
            } else {
                DaemonResponse::Success(format!("⚠ ID not found: {}", id))
            }
        }

        DaemonRequest::ListWatches => {
            let guard = state.watches.read().await;
            let mut r: Vec<WatchInfo> = Vec::new();

            for (id, ctx) in guard.iter() {
                // Extraction des infos depuis ctx.repo
                let repo_name = &ctx.repo.name;
                let branch = &ctx.repo.branch;
                let commit_hash = &ctx.repo.last_commit;
                let remote_url = &ctx.repo.remote;
                let project_dir = &ctx.project_dir;

                let short_commit = if commit_hash.len() > 8 {
                    &commit_hash[..8]
                } else {
                    commit_hash
                };

                let short_url = if remote_url.len() > 40 {
                    format!("{}...", &remote_url[..37])
                } else {
                    remote_url.clone()
                };

                r.push(WatchInfo {
                    branch: branch.to_string(),
                    project_dir: project_dir.to_string(),
                    short_commit: short_commit.to_string(),
                    short_url,
                    repo_name: repo_name.to_string(),
                    id: String::from(id),
                });
            }

            DaemonResponse::ListWatches(r)
        }
        DaemonRequest::LogsWatches { id } => {
            let id = match get_name_by_id(&id).await {
                Ok(_name) => id,
                Err(_) => match get_id_by_name(&id).await? {
                    Some(uuid) => uuid,
                    None => {
                        return send_error_response(stream, "❌ No repo with this name exists")
                            .await;
                    }
                },
            };

            match get_logs_by_id(&id).await {
                Ok(logs) => DaemonResponse::Success(logs),
                Err(e) => {
                    return send_error_response(
                        stream,
                        &format!("❌ Failed to read logs for ID {}: {}", id, e),
                    )
                    .await;
                }
            }
        }
    };

    let response_str = serde_json::to_string(&response)? + "\n";
    stream.write_all(response_str.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}
