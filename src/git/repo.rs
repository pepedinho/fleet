use git2::{Cred, Error, FetchOptions, RemoteCallbacks, Repository};
use serde::{Deserialize, Serialize};

use crate::{core::watcher::WatchContext, git::remote::find_ssh_key};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Repo {
    pub branches: Branches,
    pub name: String,
    pub remote: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Branch {
    pub branch: String,      // the name of this branch
    pub last_commit: String, // last commit of this branch
    pub remote: String,      // remote url of the repo
    pub name: String,        // name of the repo
                             // (do not use if you want to retrieve the branch name, use .branch instead)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Branches {
    pub branches: Vec<Branch>,
    pub last_commit: String, // the last commit who triggered a pipeline (used for log)
    pub last_name: String,   // the last name of the branch who triggered a pipeline
    pub name: String, // the name of the last branch in branches: Vec<Branch> (used for ps command)
}

impl Branches {
    pub fn last_mut(&mut self) -> anyhow::Result<&mut Branch> {
        if let Some(last) = self.branches.last_mut() {
            Ok(last)
        } else {
            anyhow::bail!("failed to recover branch");
        }
    }

    pub fn last(&self) -> anyhow::Result<&Branch> {
        if let Some(last) = self.branches.last() {
            Ok(last)
        } else {
            anyhow::bail!("failed to recover branch");
        }
    }

    pub fn default_last_commit(&self) -> anyhow::Result<String> {
        let last = self.last()?;
        Ok(last.last_commit.clone())
    }

    pub fn try_for_each<F, T>(&mut self, mut f: F) -> anyhow::Result<Vec<T>>
    where
        F: FnMut(&mut Branch) -> anyhow::Result<T>,
    {
        let mut results = Vec::new();

        for branch in &mut self.branches {
            results.push(f(branch)?);
        }

        Ok(results)
    }
}

impl From<Vec<Branch>> for Branches {
    fn from(branches: Vec<Branch>) -> Self {
        Branches {
            branches,
            last_commit: String::default(),
            name: String::default(),
            last_name: String::default(),
        }
    }
}

impl From<Branch> for Branches {
    fn from(branch: Branch) -> Self {
        Branches {
            branches: vec![branch],
            last_commit: String::default(),
            name: String::default(),
            last_name: String::default(),
        }
    }
}

impl Repo {
    pub fn build(branch_names: Vec<String>) -> anyhow::Result<Self> {
        let repo = Repository::open(".")?;
        let mut remote = String::new();
        let mut repo_name = String::new();

        let branches: Vec<Branch> = branch_names
            .iter()
            .map(|name| {
                let (branch, commit) = {
                    let branch_ref = repo.find_branch(name, git2::BranchType::Remote)?;
                    let target = branch_ref.get().peel_to_commit()?;

                    let branch_name = branch_ref
                        .name()?
                        .ok_or_else(|| Error::from_str("Failed to read branch name"))?
                        .to_string();

                    (branch_name, target.id().to_string())
                };
                remote = repo
                    .find_remote("origin")?
                    .url()
                    .ok_or_else(|| Error::from_str("Remote URL 'origin' not found"))?
                    .to_string();
                repo_name = remote
                    .rsplit('/')
                    .next()
                    .and_then(|s| s.strip_suffix(".git").or(Some(s)))
                    .ok_or_else(|| Error::from_str("Failed to parse repo name from remote URL"))?
                    .to_string();
                Ok(Branch {
                    branch,
                    last_commit: commit,
                    remote: remote.clone(),
                    name: repo_name.clone(),
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(Self {
            branches: branches.into(),
            remote,
            name: repo_name,
        })
    }

    pub fn default_build() -> anyhow::Result<Self> {
        let repo = Repository::open(".")?;

        let branch = {
            let head = repo.head()?;
            let branch = head
                .shorthand()
                .ok_or_else(|| Error::from_str("Failed to read branch name"))?
                .to_string();
            let commit_id = head.peel_to_commit()?.id().to_string();

            let remote = repo
                .find_remote("origin")?
                .url()
                .ok_or_else(|| Error::from_str("Remote URL 'origin' not found"))?
                .to_string();

            let repo_name = remote
                .rsplit('/')
                .next()
                .and_then(|s| s.strip_suffix(".git").or(Some(s)))
                .ok_or_else(|| Error::from_str("Failed to parse repo name from remote URL"))?
                .to_string();

            Branch {
                branch,
                last_commit: commit_id,
                remote: remote.clone(),
                name: repo_name.clone(),
            }
        };

        //trash code (only for prototyping)

        let remote = repo
            .find_remote("origin")?
            .url()
            .ok_or_else(|| Error::from_str("Remote URL 'origin' not found"))?
            .to_string();

        let repo_name = remote
            .rsplit('/')
            .next()
            .and_then(|s| s.strip_suffix(".git").or(Some(s)))
            .ok_or_else(|| Error::from_str("Failed to parse repo name from remote URL"))?
            .to_string();

        Ok(Self {
            branches: branch.into(),
            remote,
            name: repo_name,
        })
    }

    pub fn pull(repo: &Repository, branch_name: &str) -> anyhow::Result<()> {
        let ssh_key_path = find_ssh_key()?;

        let mut cb = RemoteCallbacks::new();

        cb.credentials(|_, username_from_url, _| {
            Cred::ssh_key(username_from_url.unwrap(), None, &ssh_key_path, None)
        });

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(cb);

        let mut remote = repo.find_remote("origin")?;
        remote.fetch(&[branch_name], Some(&mut fo), None)?;
        Ok(())
    }

    pub fn switch_branch(ctx: &WatchContext, remote_branch: &str) -> anyhow::Result<()> {
        Repo::switch_branch_inner(ctx, remote_branch, 0)
    }

    /// This function behaves like `git checkout`.
    /// - First, it tries to switch to the branch locally.
    /// - If the branch is not found locally, it looks for the corresponding remote branch (`origin/<branch>`).
    /// - If the remote branch exists, it creates a new local branch tracking it and retries.
    /// - If the remote branch does not exist, it fetches from `origin` and retries once more.
    /// - If the branch still cannot be found after the allowed number of attempts, it returns an error.
    fn switch_branch_inner(
        ctx: &WatchContext,
        remote_branch: &str,
        attempt: u8,
    ) -> anyhow::Result<()> {
        const MAX_ATTEMPTS: u8 = 2;

        if attempt > MAX_ATTEMPTS {
            anyhow::bail!(
                "Unable to find branch `{}` locally or remotely",
                remote_branch
            );
        }

        let repo = Repository::open(&ctx.project_dir)?;

        let branch_name = remote_branch
            .strip_prefix("origin/")
            .unwrap_or(remote_branch);

        if repo.head()?.shorthand().unwrap_or_default() == branch_name {
            // already on the right branch
            eprintln!("already on the good branch");
            return Ok(());
        }

        let branch = repo.find_branch(branch_name, git2::BranchType::Local);
        match branch {
            Ok(b) => {
                let branch_ref = b.get();
                let commit = branch_ref.peel_to_commit()?;
                repo.set_head(branch_ref.name().unwrap())?;
                repo.checkout_tree(commit.as_object(), None)?;
                Ok(())
            }
            Err(_) => {
                match repo.find_branch(remote_branch, git2::BranchType::Remote) {
                    Ok(remote_ref) => {
                        // if remote exists
                        let target_commit = remote_ref.get().peel_to_commit()?;
                        repo.branch(branch_name, &target_commit, false)?;
                        Repo::switch_branch_inner(ctx, remote_branch, attempt + 1)
                    }
                    Err(_) => {
                        // if we dont find remote -> fetch and retry
                        Repo::pull(&repo, branch_name)?;
                        Repo::switch_branch_inner(ctx, remote_branch, attempt + 1)
                    }
                }
            }
        }
    }
}
