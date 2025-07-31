use std::{io::Read, os::unix::net::UnixStream};

use crate::git::repo::Repo;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "action")]
pub enum DaemonRequest {
    #[serde(rename ="add_watch")]
    AddWatch { 
        project_dir: String,
        branch: String,
        repo: Repo,
        update_cmds: Vec<String>,
    }
}

pub fn handle_request(mut stream: UnixStream) -> Result<(), anyhow::Error> {
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer)?;

    let req: DaemonRequest = serde_json::from_str(&buffer)?;
    match req {
        DaemonRequest::AddWatch { project_dir, branch, repo, update_cmds } => {
            println!("nouveau projet a surveillé : {}", project_dir);
        }
    }

    Ok(())
}