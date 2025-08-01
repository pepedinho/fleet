use git2::{Cred, FetchOptions, Remote, RemoteCallbacks};

pub fn get_remote_branch_hash(url: &str, branch: &str) -> Result<String, git2::Error> {
    // On crée un repo temporaire en mémoire
    // let mut remote = Remote::create_detached(url)?; 

    // // On connecte pour lire les refs
    // remote.connect(git2::Direction::Fetch)?;

    // // On récupère toutes les références du serveur distant
    // let refs = remote.list()?;

    // // On cherche la branche désirée
    // let ref_to_find = format!("refs/heads/{}", branch);
    // for r in refs {
    //     if r.name() == ref_to_find {
    //         return Ok(r.oid().to_string());
    //     }
    // }

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_type| {
        Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
    });

    // let mut fetch_options = FetchOptions::new();
    // fetch_options.remote_callbacks(callbacks);

    let mut remote = Remote::create_detached(url)?;
    remote.connect_auth(git2::Direction::Fetch, Some(callbacks), None)?;

    let refs = remote.list()?;

    let ref_to_find = format!("refs/heads/{}", branch);
    for r in refs {
        if r.name() == ref_to_find {
            return Ok(r.oid().to_string());
        }
    }


    Err(git2::Error::from_str(format!("Branch {} not found", branch).as_str()))
}