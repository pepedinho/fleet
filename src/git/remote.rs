#![allow(dead_code)]
use std::path::PathBuf;

use dirs::home_dir;
use git2::{BranchType, Cred, Error, Remote, RemoteCallbacks, Repository};

fn find_ssh_key() -> Result<PathBuf, Error> {
    let keys_name = vec![String::from("id_ed25519"), String::from("id_rsa")];
    for k in keys_name {
        let ssh_key_path = home_dir()
            .map(|h| h.join(".ssh/").join(k))
            .expect("Failed to find HOME directory");
        if ssh_key_path.exists() {
            return Ok(ssh_key_path);
        }
    }
    Err(git2::Error::from_str(
        "Failed to find ssh_key on your machine :/",
    ))
}

pub fn get_remote_branch_hash(url: &str, branch: &str) -> Result<String, Error> {
    let mut callbacks = RemoteCallbacks::new();

    callbacks.credentials(|_url, username_from_url, allowed_types| {
        let username = username_from_url.unwrap_or("git");
        let ssh_key_path = find_ssh_key()?;

        if allowed_types.contains(git2::CredentialType::SSH_KEY)
            && let Ok(cred) = Cred::ssh_key(username, None, &ssh_key_path, None)
        {
            return Ok(cred);
        }

        // Try default credentials
        if allowed_types.contains(git2::CredentialType::DEFAULT)
            && let Ok(cred) = Cred::default()
        {
            return Ok(cred);
        }

        // Try ssh-agent
        if allowed_types.contains(git2::CredentialType::SSH_KEY)
            && let Ok(cred) = Cred::ssh_key_from_agent(username)
        {
            return Ok(cred);
        }

        Err(git2::Error::from_str("No authentication methods available"))
    });

    let mut remote = Remote::create_detached(url)?;
    remote.connect_auth(git2::Direction::Fetch, Some(callbacks), None)?;

    let refs = remote.list()?;
    let ref_to_find = format!("refs/heads/{branch}");

    for r in refs {
        if r.name() == ref_to_find {
            return Ok(r.oid().to_string());
        }
    }

    Err(git2::Error::from_str(&format!("Branch {branch} not found")))
}

/// search in current repository all remotes branches and store it in Vec<String>
pub fn branch_wildcard() -> anyhow::Result<Vec<String>> {
    let repo = Repository::open(".")?;
    branch_wildcard_from_repo(&repo)
}

// search in the repo provided in argument all remotes branches and store it in Vec<String>
pub fn branch_wildcard_from_repo(repo: &Repository) -> anyhow::Result<Vec<String>> {
    let branches = repo.branches(Some(BranchType::Remote))?;

    let names: Vec<_> = branches
        .into_iter()
        .map(|b| {
            let (branche, _) = b?;
            let name = branche.name()?;
            Ok(name.unwrap_or("").to_string())
        })
        .collect::<Result<Vec<_>, git2::Error>>()?;

    Ok(names)
}
