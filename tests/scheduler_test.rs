use std::{collections::HashMap, fs, sync::Arc};

use core_lib::{
    config::{Cmd, Job, Pipeline, ProjectConfig},
    core::watcher::{WatchContext, WatchContextBuilder},
    exec::pipeline::run_pipeline,
    git::repo::Repo,
};

fn build_repo() -> Repo {
    Repo {
        branch: "main".to_string(),
        last_commit: "abc".to_string(),
        name: "name".to_string(),
        remote: "git://github.com/pepedinho/fleet.git".to_string(),
    }
}

async fn build_test_ctx(id: &str, jobs: HashMap<String, Job>) -> anyhow::Result<Arc<WatchContext>> {
    let config = ProjectConfig {
        pipeline: Pipeline {
            jobs,
            ..Default::default()
        },
        branch: None,
        timeout: None,
    };
    Ok(Arc::new(
        WatchContextBuilder::new(
            "main".to_string(),
            build_repo(),
            config,
            ".".to_string(),
            id.to_string(),
        )
        .build()
        .await?,
    ))
}

fn assert_in_log_order(log: &str, a: &str, b: &str) {
    let idx_a = log
        .find(a)
        .unwrap_or_else(|| panic!("{a} not found in log"));
    let idx_b = log
        .find(b)
        .unwrap_or_else(|| panic!("{b} not found in log"));
    assert!(idx_a < idx_b, "{a} should appear before {b} in logs");
}

fn assert_in_log(log: &str, job: &str) {
    assert!(
        log.contains(job),
        "Expected to find {job} in logs, but it wasn't there"
    );
}

#[tokio::test]
async fn test_simple_dependency_order() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![
        (
            "job1".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job1".into(),
                    blocking: false,
                    container: None,
                }],
                needs: vec![],
                pipe: String::new(),
                env: None,
            },
        ),
        (
            "job2".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job2".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job1".into()],
                env: None,
            },
        ),
    ]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_multiple_dependencies", jobs).await?;

    run_pipeline(ctx.clone()).await.unwrap();
    let content = fs::read_to_string(ctx.log_path())?;
    let idx1 = content.find("job1").unwrap();
    let idx2 = content.find("job2").unwrap();
    assert!(idx1 < idx2, "job1 should finish before job2");
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_independent_jobs_run_in_parallel() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![
        (
            "job1".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job1".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec![],
                env: None,
            },
        ),
        (
            "job2".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job2".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec![],
                env: None,
            },
        ),
    ]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_multiple_dependencies", jobs).await?;

    run_pipeline(ctx.clone()).await.unwrap();
    let content = fs::read_to_string(ctx.log_path())?;

    assert!(content.contains("job1"));
    assert!(content.contains("job2"));

    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_long_chain_dependency_order() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![
        (
            "job1".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job1".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec![],
                env: None,
            },
        ),
        (
            "job2".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job2".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job1".into()],
                env: None,
            },
        ),
        (
            "job3".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job3".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job2".into()],
                env: None,
            },
        ),
        (
            "job4".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job4".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job3".into()],
                env: None,
            },
        ),
    ]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_multiple_dependencies", jobs).await?;

    run_pipeline(ctx.clone()).await.unwrap();
    let content = fs::read_to_string(ctx.log_path())?;

    let idx1 = content.find("job1").unwrap();
    let idx2 = content.find("job2").unwrap();
    let idx3 = content.find("job3").unwrap();
    let idx4 = content.find("job4").unwrap();

    assert!(idx1 < idx2 && idx2 < idx3 && idx3 < idx4);
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_single_job_pipeline() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![(
        "job1".into(),
        Job {
            steps: vec![Cmd {
                cmd: "echo single".into(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            needs: vec![],
            env: None,
        },
    )]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_multiple_dependencies", jobs).await?;

    run_pipeline(ctx.clone()).await.unwrap();
    let log = fs::read_to_string(ctx.log_path())?;
    assert_in_log(&log, "single");
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_jobs_converging() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![
        (
            "job1".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job1".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec![],
                env: None,
            },
        ),
        (
            "job2".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job2".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job1".into()],
                env: None,
            },
        ),
        (
            "job3".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job3".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job1".into()],
                env: None,
            },
        ),
        (
            "job4".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job4".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job2".into(), "job3".into()],
                env: None,
            },
        ),
    ]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_multiple_dependencies", jobs).await?;

    run_pipeline(ctx.clone()).await.unwrap();
    let log = fs::read_to_string(ctx.log_path())?;
    assert_in_log_order(&log, "job1", "job2");
    assert_in_log_order(&log, "job1", "job3");
    assert_in_log_order(&log, "job2", "job4");
    assert_in_log_order(&log, "job3", "job4");
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_pipeline_empty() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = HashMap::new();
    let ctx = build_test_ctx("test_multiple_dependencies", jobs).await?;

    run_pipeline(ctx.clone()).await.unwrap();
    let log = fs::read_to_string(ctx.log_path())?;
    assert!(log.is_empty(), "Expected empty log for empty pipeline");
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_missing_dependency() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![(
        "job2".into(),
        Job {
            steps: vec![Cmd {
                cmd: "echo job2".into(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            needs: vec!["ghost".into()],
            env: None,
        },
    )]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_multiple_dependencies", jobs).await?;

    let result = run_pipeline(ctx.clone()).await;
    assert!(
        result.is_err(),
        "Pipeline should fail on missing dependency"
    );
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_job_failure_blocks_dependents() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![
        (
            "job1".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "exit 1".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec![],
                env: None,
            },
        ),
        (
            "job2".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job2".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job1".into()],
                env: None,
            },
        ),
    ]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_multiple_dependencies", jobs).await?;

    let result = run_pipeline(ctx.clone()).await;
    assert!(result.is_err(), "Pipeline should fail because job1 failed");
    let log = fs::read_to_string(ctx.log_path())?;
    assert_in_log(&log, "job1");
    assert!(
        !log.contains("job2"),
        "job2 should not run because job1 failed"
    );
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_cyclic_dependency_detected() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![
        (
            "job1".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job1".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job2".into()],
                env: None,
            },
        ),
        (
            "job2".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job2".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job1".into()],
                env: None,
            },
        ),
    ]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_multiple_dependencies", jobs).await?;

    let result = run_pipeline(ctx.clone()).await;
    assert!(
        result.is_err(),
        "Scheduler should reject cyclic dependencies"
    );
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_multiple_steps_in_one_job() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![(
        "job1".into(),
        Job {
            steps: vec![
                Cmd {
                    cmd: "echo step1".into(),
                    blocking: false,
                    container: None,
                },
                Cmd {
                    cmd: "echo step2".into(),
                    blocking: false,
                    container: None,
                },
            ],
            pipe: String::new(),
            needs: vec![],
            env: None,
        },
    )]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_multiple_dependencies", jobs).await?;

    run_pipeline(ctx.clone()).await.unwrap();
    let log = fs::read_to_string(ctx.log_path())?;
    assert_in_log(&log, "step1");
    assert_in_log(&log, "step2");
    assert_in_log_order(&log, "step1", "step2");
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_timeout_job() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![(
        "job1".into(),
        Job {
            steps: vec![Cmd {
                cmd: "sleep 5".into(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            needs: vec![],
            env: None,
        },
    )]
    .into_iter()
    .collect();

    let config = ProjectConfig {
        pipeline: Pipeline {
            jobs,
            ..Default::default()
        },
        branch: None,
        timeout: Some(2),
    };
    let ctx = Arc::new(
        WatchContextBuilder::new(
            "main".to_string(),
            build_repo(),
            config,
            ".".to_string(),
            "test_multiple_dependencies".to_string(),
        )
        .build()
        .await?,
    );

    let result = run_pipeline(ctx.clone()).await;
    assert!(
        result.is_err(),
        "Pipeline should fail because job exceeded timeout"
    );
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_multiple_dependencies() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![
        (
            "job1".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job1".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec![],
                env: None,
            },
        ),
        (
            "job2".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job2".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec![],
                env: None,
            },
        ),
        (
            "job3".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job3".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job1".into(), "job2".into()],
                env: None,
            },
        ),
    ]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_multiple_dependencies", jobs).await?;
    run_pipeline(ctx.clone()).await.unwrap();
    let log = fs::read_to_string(ctx.log_path())?;
    assert_in_log_order(&log, "job1", "job3");
    assert_in_log_order(&log, "job2", "job3");
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_parallel_then_dependent_job() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![
        (
            "job1".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo A".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec![],
                env: None,
            },
        ),
        (
            "job2".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo B".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec![],
                env: None,
            },
        ),
        (
            "job3".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo C".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job1".into(), "job2".into()],
                env: None,
            },
        ),
    ]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_parallel_then_dependent_job", jobs).await?;
    run_pipeline(ctx.clone()).await.unwrap();
    let log = fs::read_to_string(ctx.log_path())?;
    assert_in_log_order(&log, "A", "C");
    assert_in_log_order(&log, "B", "C");
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_env_variables_in_job() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![(
        "job1".into(),
        Job {
            steps: vec![Cmd {
                cmd: "env".into(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            needs: vec![],
            env: Some(HashMap::from([("CUSTOM_ENV".into(), "VALUE123".into())])),
        },
    )]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_env_variables_in_job", jobs).await?;
    run_pipeline(ctx.clone()).await.unwrap();
    let log = fs::read_to_string(ctx.log_path())?;
    assert_in_log(&log, "VALUE123");
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_failing_job_stops_pipeline() -> anyhow::Result<()> {
    let jobs: HashMap<String, Job> = vec![
        (
            "job1".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "exit 1".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec![],
                env: None,
            },
        ),
        (
            "job2".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo should_not_run".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job1".into()],
                env: None,
            },
        ),
    ]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_failing_job_stops_pipeline", jobs).await?;
    let result = run_pipeline(ctx.clone()).await;
    // dbg!(&result);
    assert!(result.is_err(), "Pipeline should fail");
    let log = fs::read_to_string(ctx.log_path())?;
    assert!(
        !log.contains("should_not_run"),
        "job2 must not execute if job1 fails"
    );
    ctx.logger.clean().await?;
    Ok(())
}

#[tokio::test]
async fn test_mixed_parallel_and_sequential() -> anyhow::Result<()> {
    // job1 -> job2
    // job1 -> job3
    // job2 & job3 -> job4
    let jobs: HashMap<String, Job> = vec![
        (
            "job1".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job1".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec![],
                env: None,
            },
        ),
        (
            "job2".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job2".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job1".into()],
                env: None,
            },
        ),
        (
            "job3".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job3".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job1".into()],
                env: None,
            },
        ),
        (
            "job4".into(),
            Job {
                steps: vec![Cmd {
                    cmd: "echo job4".into(),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                needs: vec!["job2".into(), "job3".into()],
                env: None,
            },
        ),
    ]
    .into_iter()
    .collect();

    let ctx = build_test_ctx("test_mixed_parallel_and_sequential", jobs).await?;
    run_pipeline(ctx.clone()).await.unwrap();
    let log = fs::read_to_string(ctx.log_path())?;
    assert_in_log_order(&log, "job1", "job2");
    assert_in_log_order(&log, "job1", "job3");
    assert_in_log_order(&log, "job2", "job4");
    assert_in_log_order(&log, "job3", "job4");
    ctx.logger.clean().await?;
    Ok(())
}
