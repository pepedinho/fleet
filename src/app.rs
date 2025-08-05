use std::path::Path;

use anyhow::Result;

use crate::{
    cli::{Cli, Commands},
    config::parser::load_config,
    git::repo::Repo,
    ipc::{client::send_watch_request, server::DaemonRequest},
};

pub async fn handle_watch(cli: &Cli) -> Result<()> {
    let config_path = Path::new("./fleet.yml");
    if !config_path.exists() {
        return Err(anyhow::anyhow!("File `fleet.yml` missing from current directory."))?;
    }

    let config = load_config(config_path)?;
    let repo = Repo::build()?;

    let watch_req = match &cli.command {
        Commands::Watch { branch: branch_cli } => {
            let branch = branch_cli
                .clone()
                .or(config.branch.clone())
                .unwrap_or(repo.branch.clone());

            println!("Branche sélectionnée : {}", branch);
            println!("Remote : {}", &repo.remote);
            println!("Dernier commit local : {}", &repo.last_commit);

            DaemonRequest::AddWatch {
                project_dir: std::env::current_dir()?.to_string_lossy().into_owned(),
                branch,
                repo,
                update_cmds: config.update.clone(),
            }
        }
        Commands::Ps { all: _ } => DaemonRequest::ListWatches,
        Commands::Logs { id_or_name } => {
            match id_or_name {
                Some(s) => {
                    DaemonRequest::LogsWatches {id: s.to_string() }
                },
                None => {
                    DaemonRequest::LogsWatches { id: repo.name }
                }
            }
        }
        _ => {
            return Err(anyhow::anyhow!("oui"))?;
        }
    };

    send_watch_request(watch_req).await?;
    Ok(())
}
