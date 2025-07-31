use std::path::Path;

use crate::{config::{self, parser::load_config}, git::repo::Repo};


pub fn handle_watch(branch_cli: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = Path::new("./fleet.yml");
    if !config_path.exists() {
        return Err("File `fleet.yml` missing from current directory.".into());
    }

    let config = load_config(&config_path)?;

    let repo = Repo::build()?;

    let branch = match branch_cli {
        Some(b) => b,
        _ => {
            if let Some(br) = config.branch {
                 br
            } else {
                repo.branch
            }
        }
    };

    println!("Branche sélectionnée : {}", branch);
    println!("Remote : {}", repo.remote);
    println!("Dernier commit local : {}", repo.last_commit);

    todo!()
}