use std::path::Path;

use crate::{config::parser::load_config, core::watcher::WatchRequest, git::repo::Repo, ipc::{client::send_watch_request, server::DaemonRequest}};


pub fn handle_watch(branch_cli: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = Path::new("./fleet.yml");
    if !config_path.exists() {
        return Err("File `fleet.yml` missing from current directory.".into());
    }

    let config = load_config(&config_path)?;
    let repo = Repo::build()?;

    let branch = branch_cli
        .or(config.branch.clone())
        .unwrap_or(repo.branch.clone());

    println!("Branche sélectionnée : {}", branch);
    println!("Remote : {}", &repo.remote);
    println!("Dernier commit local : {}", &repo.last_commit);

    let watch_req = DaemonRequest::AddWatch {
        project_dir: std::env::current_dir()?.to_string_lossy().into_owned(),
        branch,
        repo,
        update_cmds: config.update.clone(),
    };

    send_watch_request(watch_req)?;

    todo!()
}