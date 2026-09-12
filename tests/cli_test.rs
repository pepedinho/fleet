use std::env;
use std::path::Path;

use core_lib::cli;
use core_lib::cli::Cli;
use core_lib::cli::builders::build_watch_request;
use core_lib::config::parser::load_config;
use core_lib::daemon::server::DaemonRequest;
use core_lib::git::repo::Repo;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

/// Removes an env var when dropped, even on panic.
struct EnvVarGuard(&'static str);

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        unsafe { env::remove_var(self.0) };
    }
}

fn set_env(key: &'static str, value: &str) -> EnvVarGuard {
    unsafe { env::set_var(key, value) };
    EnvVarGuard(key)
}

/// Restores the process working directory when dropped.
struct CurrentDirGuard(std::path::PathBuf);

impl CurrentDirGuard {
    fn from_dir(dir: &Path) -> Self {
        let old = env::current_dir().expect("failed to read current dir");
        env::set_current_dir(dir).expect("failed to change current dir");
        Self(old)
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        env::set_current_dir(&self.0).expect("failed to restore current dir");
    }
}

// These tests are run from the repository root by CI (and by the repo's own
// `fleet.yml`), which is a git repo with a `fleet.yml` and remote-tracking
// branches. They must be executed serially (`--test-threads=1`).

#[tokio::test]
async fn test_watch_command_builds_add_watch_request() -> anyhow::Result<()> {
    let _env = set_env("SECRET_TOKEN", "test");

    let cli = Cli {
        command: cli::Commands::Watch,
    };

    let watch_req = build_watch_request(&cli).await?;

    let (project_dir, repo, config) = match watch_req {
        DaemonRequest::AddWatch {
            project_dir,
            repo,
            config,
        } => (project_dir, *repo, *config),
        other => panic!("expected AddWatch, got {other:?}"),
    };

    assert_eq!(project_dir, env::current_dir()?.to_string_lossy());
    assert_eq!(config.branches, vec!["*".to_string()]);
    assert_eq!(config.timeout, Some(200));
    let jobs = config.pipeline.jobs;
    for expected in ["test_rust", "sleep_test", "deploy"] {
        assert!(jobs.contains_key(expected), "missing job {expected}");
    }
    assert!(
        !repo.name.is_empty(),
        "repo name must be derived from origin"
    );
    assert!(repo.remote.contains("github.com"));

    Ok(())
}

#[tokio::test]
async fn test_logs_command_defaults_to_current_repo_name() -> anyhow::Result<()> {
    let cli = Cli {
        command: cli::Commands::Logs {
            id_or_name: None,
            follow: false,
        },
    };

    let req = build_watch_request(&cli).await?;

    let expected_name = Repo::default_build()?.name;
    match req {
        DaemonRequest::LogsWatches { id, f } => {
            assert_eq!(id, expected_name);
            assert!(!f);
        }
        other => panic!("expected LogsWatches, got {other:?}"),
    }

    Ok(())
}

#[test]
fn test_load_config_parses_repo_self_config() -> anyhow::Result<()> {
    let _env = set_env("SECRET_TOKEN", "test");

    let config = load_config(Path::new("./fleet.yml"))?;

    assert_eq!(config.branches, vec!["*".to_string()]);
    assert_eq!(config.timeout, Some(200));
    assert!(config.pipeline.jobs.contains_key("deploy"));

    Ok(())
}

#[tokio::test]
async fn test_watch_command_missing_fleet_yml() -> anyhow::Result<()> {
    let empty_dir = TempDir::new()?;
    let _cwd = CurrentDirGuard::from_dir(empty_dir.path());

    let cli = Cli {
        command: cli::Commands::Watch,
    };

    let err = build_watch_request(&cli).await.unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("fleet.yml"),
        "expected a fleet.yml error, got: {msg}"
    );

    Ok(())
}

#[cfg(feature = "no-tty")]
#[tokio::test]
async fn test_load_config_missing_env_fails_headless() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let config_path = dir.path().join("fleet.yml");
    std::fs::write(
        &config_path,
        concat!(
            "branches: ['*']\n",
            "pipeline:\n",
            "  jobs:\n",
            "    build:\n",
            "      env:\n",
            "        FLEET_TEST_MISSING_VAR: '$'\n",
            "      steps:\n",
            "        - cmd: echo hi\n",
        ),
    )?;

    let err =
        load_config(&config_path).expect_err("missing env var must be rejected without a TTY");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("Missing env variable"),
        "expected a missing-env error, got: {msg}"
    );

    Ok(())
}
