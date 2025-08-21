use std::path::Path;

use core_lib::app::build_watch_request;
use core_lib::cli::{self, Cli};
use core_lib::config::parser::load_config;
use core_lib::git::repo::Repo;
use core_lib::ipc::server::DaemonRequest;
use pretty_assertions::assert_eq;

#[test]
fn test_build_watch_request_branch_none() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli {
        command: cli::Commands::Watch { branch: None },
    };

    let repo = Repo::build(None)?;
    let config = load_config(Path::new("./fleet.yml"))?;

    let watch_req = build_watch_request(&cli)?;
    assert_eq!(
        watch_req,
        DaemonRequest::AddWatch {
            project_dir: std::env::current_dir()?.to_string_lossy().into_owned(),
            branch: repo.branch.clone(),
            repo: Box::new(repo),
            config: Box::new(config),
        }
    );

    Ok(())
}

#[test]
fn test_build_watch_request_branch_some() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repo::build(None)?;
    let cli = Cli {
        command: cli::Commands::Watch {
            branch: Some(repo.branch.clone()),
        },
    };

    let config = load_config(Path::new("./fleet.yml"))?;

    let watch_req = build_watch_request(&cli)?;
    assert_eq!(
        watch_req,
        DaemonRequest::AddWatch {
            project_dir: std::env::current_dir()?.to_string_lossy().into_owned(),
            branch: repo.branch.clone(),
            repo: Box::new(repo),
            config: Box::new(config),
        }
    );

    Ok(())
}
