use git2::{Error, Repository};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Repo {
    pub branch: String,
    pub remote: String,
    pub last_commit: String,
    pub name: String,
}

impl Repo {
    pub fn build(branch_name: Option<String>) -> Result<Self, Error> {
        let repo = Repository::open(".")?;

        let (branch, commit) = if let Some(ref name) = branch_name {
            let branch_ref = repo.find_branch(name, git2::BranchType::Local)?;
            let target = branch_ref.get().peel_to_commit()?;

            let branch_name = branch_ref
                .name()?
                .ok_or_else(|| Error::from_str("Failed to read branch name"))?
                .to_string();

            (branch_name, target.id().to_string())
        } else {
            let head = repo.head()?;
            let branch = head
                .shorthand()
                .ok_or_else(|| Error::from_str("Failed to read branch name"))?
                .to_string();
            let commit_id = head.peel_to_commit()?.id().to_string();
            (branch, commit_id)
        };

        let remote = repo
            .find_remote("origin")?
            .url()
            .ok_or_else(|| Error::from_str("Remote URL 'origin' not found"))?
            .to_string();

        let name = remote
            .rsplit('/')
            .next()
            .and_then(|s| s.strip_suffix(".git").or(Some(s)))
            .ok_or_else(|| Error::from_str("Failed to parse repo name from remote URL"))?
            .to_string();

        Ok(Self {
            branch,
            remote,
            last_commit: commit,
            name,
        })
    }
}
