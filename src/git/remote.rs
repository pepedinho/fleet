#![allow(dead_code)]
use dirs::home_dir;
use git2::{Cred, Error, Remote, RemoteCallbacks};

pub fn get_remote_branch_hash(url: &str, branch: &str) -> Result<String, Error> {
    let mut callbacks = RemoteCallbacks::new();

    callbacks.credentials(|_url, username_from_url, allowed_types| {
        let username = username_from_url.unwrap_or("git");
        let ssh_key_path = home_dir()
            .map(|h| h.join(".ssh/id_rsa"))
            .expect("Failed to find HOME directory");
        println!("find ssh in key => {}", ssh_key_path.display());

        // Try default key locations (~/.ssh/id_rsa)
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            if let Ok(cred) = Cred::ssh_key(username, None, &ssh_key_path, None) {
                return Ok(cred);
            }
        }
        // Try default credentials
        if allowed_types.contains(git2::CredentialType::DEFAULT) {
            if let Ok(cred) = Cred::default() {
                return Ok(cred);
            }
        }

        // Try ssh-agent
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            if let Ok(cred) = Cred::ssh_key_from_agent(username) {
                return Ok(cred);
            }
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

    Err(git2::Error::from_str(&format!(
        "Branch {branch} not found"
    )))
}
