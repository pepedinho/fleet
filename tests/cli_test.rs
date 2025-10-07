use std::path::Path;

use core_lib::cli::builders::build_watch_request;
use core_lib::cli::{self, Cli};
use core_lib::config::parser::load_config;
use core_lib::daemon::server::DaemonRequest;
use core_lib::git::remote::branch_wildcard;
use core_lib::git::repo::Repo;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn test_build_watch_request_branch_some() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli {
        command: cli::Commands::Watch,
    };

    let config = load_config(Path::new("./fleet.yml"))?;
    let (branches, b_name) = if config.branches[0] == "*" {
        (branch_wildcard()?, "*".to_string())
    } else {
        (config.branches.clone(), config.branches[0].clone())
    };
    let mut repo = Repo::build(branches)?;
    repo.branches.name = b_name;
    let watch_req = build_watch_request(&cli).await?;
    assert_eq!(
        watch_req,
        DaemonRequest::AddWatch {
            project_dir: std::env::current_dir()?.to_string_lossy().into_owned(),
            repo: Box::new(repo),
            config: Box::new(config),
        }
    );

    Ok(())
}
