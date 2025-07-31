use anyhow::Ok;
use serde::{Deserialize, Serialize};

use crate::{config::parser::ProjectConfig, exec::runner::run_update, git::{remote::get_remote_branch_hash, repo::Repo}};


#[derive(Serialize, Deserialize, Debug)]
pub struct WatchRequest {
    pub project_dir: String,
    pub branch: String,
    pub repo: Repo,
    pub update_cmds: Vec<String>,
}

pub fn watch_once(ctx: &WatchRequest) -> Result<(), anyhow::Error> {
    let remote_hash = get_remote_branch_hash(&ctx.repo.remote, &ctx.branch)?;

    if remote_hash != ctx.repo.last_commit {
        println!("new commit detected: {} -> {}", ctx.repo.last_commit, remote_hash);
        // run_update(&ctx.config)?;
    }
    Ok(())
}