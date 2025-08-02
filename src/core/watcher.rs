use anyhow::Ok;

use crate::{
    config::parser::ProjectConfig,
    exec::runner::run_update,
    git::{remote::get_remote_branch_hash, repo::Repo},
};

#[derive(Debug, Clone)]
pub struct WatchContext {
    pub branch: String,
    pub repo: Repo,
    pub config: ProjectConfig,
    pub project_dir: String,
}

pub fn watch_once(ctx: &WatchContext) -> Result<(), anyhow::Error> {
    let remote_hash = get_remote_branch_hash(&ctx.repo.remote, &ctx.branch)?;

    if remote_hash != ctx.repo.last_commit {
        println!(
            "new commit detected: {} -> {}",
            ctx.repo.last_commit, remote_hash
        );
        run_update(&ctx.config)?;
    }
    Ok(())
}
