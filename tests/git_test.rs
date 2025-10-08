use std::fs::{self};

use anyhow::Result;
use core_lib::git::remote::branch_wildcard_from_repo;
use git2::Repository;
use tempfile::tempdir;

#[test]
fn test_branch_wildcard_with_remote_branches() -> Result<()> {
    let dir = tempdir()?;
    let repo = Repository::init(dir.path())?;

    let remotes_dir = dir.path().join(".git/refs/remotes/origin");
    fs::create_dir_all(&remotes_dir)?;

    fs::write(
        remotes_dir.join("main"),
        "0123456789abcdef0123456789abcdef01234567",
    )?;
    fs::write(
        remotes_dir.join("dev"),
        "89abcdef0123456789abcdef0123456789abcdef",
    )?;

    let result = branch_wildcard_from_repo(&repo)?;

    assert_eq!(
        result,
        vec!["origin/dev".to_string(), "origin/main".to_string()]
    );

    Ok(())
}

#[test]
fn test_no_remote_branches_returns_empty_vec() -> Result<()> {
    let dir = tempdir()?;
    let repo = Repository::init(dir.path())?;

    let result = branch_wildcard_from_repo(&repo)?;
    assert!(result.is_empty());
    Ok(())
}

#[test]
fn test_multiple_remotes() -> Result<()> {
    let dir = tempdir()?;
    let repo = Repository::init(dir.path())?;

    let origin_dir = dir.path().join(".git/refs/remotes/origin");
    let upstream_dir = dir.path().join(".git/refs/remotes/upstream");
    fs::create_dir_all(&origin_dir)?;
    fs::create_dir_all(&upstream_dir)?;

    fs::write(
        origin_dir.join("main"),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )?;
    fs::write(
        upstream_dir.join("feature"),
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    )?;

    let mut branches = branch_wildcard_from_repo(&repo)?;
    branches.sort();

    assert_eq!(
        branches,
        vec!["origin/main".to_string(), "upstream/feature".to_string()]
    );

    Ok(())
}
