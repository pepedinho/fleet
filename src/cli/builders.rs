use std::path::Path;

use anyhow::{Ok, Result};

use crate::{
    cli::{Cli, Commands, client::send_watch_request, stats::interface::display_stats_interface},
    config::parser::load_config,
    daemon::server::DaemonRequest,
    git::{remote::branch_wildcard, repo::Repo},
};

/// Handles watch-related CLI commands by delegating to subfunctions
/// that build the appropriate [`DaemonRequest`].
pub async fn handle_watch(cli: &Cli) -> Result<()> {
    let watch_req = build_watch_request(cli).await?;
    send_watch_request(watch_req).await?;
    Ok(())
}

/// Builds the appropriate [`DaemonRequest`] for the given CLI command.
pub async fn build_watch_request(cli: &Cli) -> Result<DaemonRequest> {
    match &cli.command {
        Commands::Watch => build_add_watch_request(),
        Commands::Ps { all } => Ok(DaemonRequest::ListWatches { all: *all }),
        Commands::Logs { id_or_name, follow } => build_logs_request(id_or_name, *follow),
        Commands::Stop { id } => Ok(DaemonRequest::StopWatch { id: id.to_string() }),
        Commands::Up { id } => Ok(DaemonRequest::UpWatch { id: id.to_string() }),
        Commands::Rm { id } => Ok(DaemonRequest::RmWatch { id: id.to_string() }),
        Commands::Stats => {
            display_stats_interface().await?;
            Ok(DaemonRequest::None)
        }
        Commands::Run { id } => Ok(DaemonRequest::RunPipeline { id: id.clone() }),
    }
}

/// Builds an [`AddWatch`] request after validating configuration.
fn build_add_watch_request() -> Result<DaemonRequest> {
    let config_path = Path::new("./fleet.yml");
    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "File `fleet.yml` missing from current directory."
        ));
    }

    let config = load_config(config_path)?;

    let (branches, b_name) = if config.branches[0] == "*" {
        (branch_wildcard()?, "*".to_string())
    } else {
        (config.branches.clone(), config.branches[0].clone())
    };

    println!("debug: branches: {:#?}", branches);

    let mut repo = Repo::build(branches.clone())?;

    repo.branches.name = b_name;
    println!("debug: repo branches: {:#?}", repo.branches);

    Ok(DaemonRequest::AddWatch {
        project_dir: std::env::current_dir()?.to_string_lossy().into_owned(),
        repo: Box::new(repo),
        config: Box::new(config),
    })
}

/// Builds a [`LogsWatches`] request from CLI or repository defaults.
fn build_logs_request(id_or_name: &Option<String>, follow: bool) -> Result<DaemonRequest> {
    match id_or_name {
        Some(s) => Ok(DaemonRequest::LogsWatches {
            id: s.to_string(),
            f: follow,
        }),
        None => {
            let repo = Repo::default_build()?;
            Ok(DaemonRequest::LogsWatches {
                id: repo.name,
                f: follow,
            })
        }
    }
}
