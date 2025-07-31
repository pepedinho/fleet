use std::{io::{Read, Write}, os::{linux::raw::stat}, sync::Arc};

use crate::{config::parser::ProjectConfig, core::{state::{AppState, SharedState}, watcher::WatchContext}, git::repo::Repo};
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncWriteExt, net::UnixStream};
use uuid::Uuid;


#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "action")]
pub enum DaemonRequest {
    #[serde(rename ="add_watch")]
    AddWatch { 
        project_dir: String,
        branch: String,
        repo: Repo,
        update_cmds: Vec<String>,
    },

    #[serde(rename ="stop_watch")]
    StopWatch { id: Uuid},

    #[serde(rename ="list_watch")]
    ListWatches,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WatchInfo {
    pub branch: String,
    pub project_dir: String,
    pub short_commit: String,
    pub short_url: String,
    pub repo_name: String,
    pub id: Uuid,
}


#[derive(Serialize, Deserialize, Debug)]
pub enum DaemonResponse {
    Success(String),
    Error(String),
    ListWatches(Vec<WatchInfo>)
}

pub async fn handle_request(req: DaemonRequest, state: Arc<AppState>,stream: &mut UnixStream) -> Result<(), anyhow::Error> {
    let response = match req {
        DaemonRequest::AddWatch { project_dir, branch, repo, update_cmds } => {
            let ctx = WatchContext {
                branch,
                repo,
                config: ProjectConfig {
                    update: update_cmds,
                    ..Default::default()
                },
                project_dir,
            };

            let mut guard = state.watches.write().await;
            let id = Uuid::new_v4();
            guard.insert(id, ctx);
            println!("📌 Project registered with ID: {}", id);
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
            let mut r : Vec<WatchInfo> = Vec::new();

            for (id, ctx) in guard.iter() {
                // Extraction des infos depuis ctx.repo
                let repo_name = &ctx.repo.name; // nom du repo
                let branch = &ctx.repo.branch;
                let commit_hash = &ctx.repo.last_commit;
                let remote_url = &ctx.repo.remote;
                let project_dir = &ctx.project_dir;

                // Tronquer le commit hash à 8 caractères (comme git)
                let short_commit = if commit_hash.len() > 8 {
                    &commit_hash[..8]
                } else {
                    &commit_hash
                };

                // Tronquer l'URL si trop longue (40 chars max)
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
                    id: *id
                });
            }

            DaemonResponse::ListWatches(r)
        }
    };

    let response_str = serde_json::to_string(&response)? + "\n";
    stream.write_all(response_str.as_bytes()).await?;
    stream.flush().await?;  
    Ok(())
}