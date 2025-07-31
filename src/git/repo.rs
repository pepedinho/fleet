use git2::{Error, Repository};

pub struct Repo {
    pub branch: String,
    pub remote: String
    pub last_commit: String,
}


impl Repo {
    pub fn build() -> Result<Self, Error> {
        let repo = Repository::open(".")?;
        
        let head = repo.head()?;
        let branch = head
            .shorthand()
            .ok_or_else(|| Error::from_str("Failed to read branch name"))?
            .to_string();

        let remote = repo
            .find_remote("origin")?
            .url()
            .ok_or_else(|| Error::from_str("Remote URL 'origin' not found"))?
            .to_string();

        let commit = head.peel_to_commit()?.id().to_string();

        Ok(Self {
            branch,
            remote,
            last_commit: commit,
        })
    }
}