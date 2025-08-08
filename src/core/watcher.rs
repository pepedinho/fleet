use std::{any::Any, path::PathBuf};

use anyhow::{Ok, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use crate::{
    config::parser::ProjectConfig, core::state::{add_watch, save_watches}, exec::runner::run_update, git::{remote::get_remote_branch_hash, repo::Repo}
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
        let home  = home_dir().unwrap();

        let log_dir = home.join(".fleet").join("logs");
        log_dir.join(self.id.to_string())
    }

    pub fn log_path_by_id(id: Uuid) -> PathBuf {
        let home  = home_dir().unwrap();

        let log_dir = home.join(".fleet").join("logs");
        log_dir.join(id.to_string())
    }

    pub async fn init_logs() -> Result<()> {
        let home  = home_dir().ok_or_else(|| anyhow::anyhow!("Failed to find HOME directory"))?;

        let log_dir = home.join(".fleet").join("logs");

        if !fs::try_exists(&log_dir).await? {
            fs::create_dir_all(&log_dir).await?;
            println!("init logs directory : {}", log_dir.display());
        } else {
            println!("log folder already exist : {}", log_dir.display());
        }
        Ok(())
    }
}

pub async fn watch_once(ctx: &WatchContext) -> Result<Option<String>, anyhow::Error> {
    let remote_hash = get_remote_branch_hash(&ctx.repo.remote, &ctx.branch)?;

    if remote_hash != ctx.repo.last_commit {
        println!(
            "new commit detected: {} -> {}",
            ctx.repo.last_commit, remote_hash
        );
        // run_update(ctx).await?;
        // ctx.repo.last_commit = String::from(remote_hash);
        return Ok(Some(remote_hash));
    }
    Ok(None)
}
