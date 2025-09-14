#![allow(dead_code)]
use std::path::PathBuf;

use anyhow::{Ok, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[allow(unused_imports)]
use crate::git::{remote::get_remote_branch_hash, repo::Repo};
use crate::{config::ProjectConfig, log::logger::Logger};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchContext {
    pub branch: String,
    pub repo: Repo,
    pub config: ProjectConfig,
    pub project_dir: String,
    pub id: String,
    pub paused: bool,
    #[serde(skip, default = "Logger::placeholder")]
    pub logger: Logger,
}

pub struct WatchContextBuilder {
    branch: String,
    repo: Repo,
    config: ProjectConfig,
    project_dir: String,
    id: String,
    paused: bool,
}

impl WatchContextBuilder {
    pub fn new(
        branch: String,
        repo: Repo,
        config: ProjectConfig,
        project_dir: String,
        id: String,
    ) -> Self {
        Self {
            branch,
            repo,
            config,
            project_dir,
            id,
            paused: false,
        }
    }

    pub async fn build(self) -> Result<WatchContext, anyhow::Error> {
        // Création du logger avec les infos du contexte partiel
        let logger = Logger::new(&self.log_path()).await?;

        // Construction du WatchContext complet
        Ok(WatchContext {
            branch: self.branch,
            repo: self.repo,
            config: self.config,
            project_dir: self.project_dir,
            id: self.id,
            paused: self.paused,
            logger,
        })
    }

    fn log_path(&self) -> PathBuf {
        let home = home_dir().unwrap();

        let log_dir = home.join(".fleet").join("logs");
        log_dir.join(self.id.to_string() + ".log")
    }
}

impl WatchContext {
    pub fn stop(&mut self) {
        self.paused = true;
    }

    pub fn run(&mut self) {
        self.paused = false;
    }

    pub fn log_path(&self) -> PathBuf {
        let home = home_dir().unwrap();

        let log_dir = home.join(".fleet").join("logs");
        log_dir.join(self.id.to_string() + ".log")
    }

    pub fn log_path_by_id(id: &str) -> PathBuf {
        let home = home_dir().unwrap();

        let log_dir = home.join(".fleet").join("logs");
        log_dir.join(id.to_string() + ".log")
    }

    pub async fn init_logs() -> Result<()> {
        let home = home_dir().ok_or_else(|| anyhow::anyhow!("Failed to find HOME directory"))?;

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
    #[cfg(not(feature = "force_commit"))]
    {
        let remote_hash = get_remote_branch_hash(&ctx.repo.remote, &ctx.branch)?;

        if remote_hash != ctx.repo.last_commit {
            println!(
                "new commit detected: {} -> {}",
                ctx.repo.last_commit, remote_hash
            );
            return Ok(Some(remote_hash));
        }
        return Ok(None);
    }
    #[cfg(feature = "force_commit")]
    return Ok(Some(ctx.repo.last_commit.clone()));
}
