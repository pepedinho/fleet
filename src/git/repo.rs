use git2::{Error, Repository};
use serde::{Deserialize, Serialize};

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
}
