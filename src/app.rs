#![allow(dead_code)]
use std::path::Path;

use anyhow::{Ok, Result};

use crate::{
    cli::{Cli, Commands},
    config::parser::load_config,
    git::repo::Repo,
    ipc::{client::send_watch_request, server::DaemonRequest},
};

/// Handles watch-related CLI commands by delegating to subfunctions
/// that build the appropriate [`DaemonRequest`].
pub async fn handle_watch(cli: &Cli) -> Result<()> {
    let watch_req = build_watch_request(cli)?;
    send_watch_request(watch_req).await?;
    Ok(())
}

/// Builds the appropriate [`DaemonRequest`] for the given CLI command.
pub fn build_watch_request(cli: &Cli) -> Result<DaemonRequest> {
    match &cli.command {
        Commands::Watch { branch } => build_add_watch_request(branch.clone()),
        Commands::Ps { all } => Ok(DaemonRequest::ListWatches { all: *all }),
        Commands::Logs { id_or_name } => build_logs_request(id_or_name),
        Commands::Stop { id } => Ok(DaemonRequest::StopWatch { id: id.to_string() }),
        Commands::Up { id } => Ok(DaemonRequest::UpWatch { id: id.to_string() }),
        Commands::Rm { id } => Ok(DaemonRequest::RmWatch { id: id.to_string() }),
    }
}

/// Builds an [`AddWatch`] request after validating configuration.
fn build_add_watch_request(branch_cli: Option<String>) -> Result<DaemonRequest> {
    let config_path = Path::new("./fleet.yml");
    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "File `fleet.yml` missing from current directory."
        ));
    }

    let config = load_config(config_path)?;
    let branch = branch_cli
        .or(config.branch.clone())
        .unwrap_or(Repo::build(None)?.branch.clone());
    let repo = Repo::build(Some(branch.clone()))?;

    Ok(DaemonRequest::AddWatch {
        project_dir: std::env::current_dir()?.to_string_lossy().into_owned(),
        branch,
        repo,
        update: config.update.clone(),
    })
}

/// Builds a [`LogsWatches`] request from CLI or repository defaults.
fn build_logs_request(id_or_name: &Option<String>) -> Result<DaemonRequest> {
    match id_or_name {
        Some(s) => Ok(DaemonRequest::LogsWatches { id: s.to_string() }),
        None => {
            let repo = Repo::build(None)?;
            Ok(DaemonRequest::LogsWatches { id: repo.name })
        }
    }
}
