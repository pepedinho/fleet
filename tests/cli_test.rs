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

    let watch_req = build_watch_request(&cli, repo.clone())?;
    assert_eq!(
        watch_req,
        DaemonRequest::AddWatch {
            project_dir: std::env::current_dir()?.to_string_lossy().into_owned(),
            branch: repo.branch.clone(),
            repo,
            update_cmds: config.update
        }
    );

    Ok(())
}

#[test]
fn test_build_watch_request_branch_some() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli {
        command: cli::Commands::Watch {
            branch: Some(String::from("main")),
        },
    };

    let repo = Repo::build(Some("main".to_string()))?;
    let config = load_config(Path::new("./fleet.yml"))?;

    let watch_req = build_watch_request(&cli, repo.clone())?;
    assert_eq!(
        watch_req,
        DaemonRequest::AddWatch {
            project_dir: std::env::current_dir()?.to_string_lossy().into_owned(),
            branch: String::from("main"),
            repo,
            update_cmds: config.update
        }
    );

    Ok(())
}
