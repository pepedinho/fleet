use git2::Remote;

pub fn get_remote_branch_hash(url: &str, branch: &str) -> Result<String, git2::Error> {
    // On crée un repo temporaire en mémoire
    let mut remote = Remote::create_detached(url)?; 

    // On connecte pour lire les refs
    remote.connect(git2::Direction::Fetch)?;

    // On récupère toutes les références du serveur distant
    let refs = remote.list()?;

    // On cherche la branche désirée
    let ref_to_find = format!("refs/heads/{}", branch);
    for r in refs {
        if r.name() == ref_to_find {
            return Ok(r.oid().to_string());
        }
    }
    Err(git2::Error::from_str(format!("Branch {} does not exist", branch).as_str()))
}