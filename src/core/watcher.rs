use std::{any::Any, path::PathBuf};

use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use crate::{
    config::parser::ProjectConfig,
    exec::runner::run_update,
    git::{remote::get_remote_branch_hash, repo::Repo},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchContext {
    pub branch: String,
    pub repo: Repo,
    pub config: ProjectConfig,
    pub project_dir: String,
    pub id: Uuid,
}

impl WatchContext {
    pub fn log_path(&self) -> PathBuf {
        PathBuf::from(format!("/var/log/fleet/{}.log", self.id))
    }

    pub fn log_path_by_id(id: Uuid) -> PathBuf {
        PathBuf::from(format!("/var/log/fleet/{}.log", id))
    }

    pub async fn init_logs() -> Result<()> {
        let path = PathBuf::from("/var/log/fleet");
        if !fs::try_exists(&path).await? {
            fs::create_dir(path).await?;
        }
        println!("create log dir");
        Ok(())
    }
}

pub async fn watch_once(ctx: &WatchContext) -> Result<(), anyhow::Error> {
    let remote_hash = get_remote_branch_hash(&ctx.repo.remote, &ctx.branch)?;

    if remote_hash != ctx.repo.last_commit {
        println!(
            "new commit detected: {} -> {}",
            ctx.repo.last_commit, remote_hash
        );
        run_update(&ctx).await?;
    }
    Ok(())
}
