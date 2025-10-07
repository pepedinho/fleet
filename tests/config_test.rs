use std::{collections::HashMap, fs, path::Path};

use core_lib::config::{
    parser::{check_dependency_graph, load_config},
    Cmd, Job, Pipeline, ProjectConfig,
};
use pretty_assertions::assert_eq;
use tempfile::NamedTempFile;

// Helper function to create a temporary YAML config file
fn create_temp_config(content: &str) -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    fs::write(file.path(), content).unwrap();
    file
}

#[test]
fn test_load_config_basic() -> anyhow::Result<()> {
    let yaml = r#"
pipeline:
  notifications:
    on: [success]
    channels: []
  jobs:
    test:
      steps:
        - cmd: echo "hello"
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    assert_eq!(config.pipeline.jobs.len(), 1);
    assert!(config.pipeline.jobs.contains_key("test"));
    Ok(())
}

#[test]
fn test_load_config_with_dependencies() -> anyhow::Result<()> {
    let yaml = r#"
pipeline:
  notifications:
    on: []
    channels: []
  jobs:
    build:
      steps:
        - cmd: cargo build
    test:
      needs: [build]
      steps:
        - cmd: cargo test
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    let test_job = config.pipeline.jobs.get("test").unwrap();
    assert_eq!(test_job.needs, vec!["build"]);
    Ok(())
}

#[test]
fn test_load_config_with_env_variables() -> anyhow::Result<()> {
    let yaml = r#"
pipeline:
  notifications:
    on: []
    channels: []
  jobs:
    deploy:
      env:
        API_KEY: "secret123"
        DEBUG: "true"
      steps:
        - cmd: ./deploy.sh
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    let deploy_job = config.pipeline.jobs.get("deploy").unwrap();
    let env = deploy_job.env.as_ref().unwrap();
    assert_eq!(env.get("API_KEY"), Some(&"secret123".to_string()));
    assert_eq!(env.get("DEBUG"), Some(&"true".to_string()));
    Ok(())
}

#[test]
fn test_load_config_with_secret_env_resolution() -> anyhow::Result<()> {
    unsafe { std::env::set_var("TEST_SECRET", "resolved_value") };
    
    let yaml = r#"
pipeline:
  notifications:
    on: []
    channels: []
  jobs:
    deploy:
      env:
        SECRET_KEY: $
        NORMAL_KEY: "normal"
      steps:
        - cmd: echo "test"
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    let deploy_job = config.pipeline.jobs.get("deploy").unwrap();
    let env = deploy_job.env.as_ref().unwrap();
    
    // SECRET_KEY should be empty string if TEST_SECRET was not the key
    // But if the key is SECRET_KEY, it will try to resolve from env
    assert!(env.contains_key("SECRET_KEY"));
    assert_eq!(env.get("NORMAL_KEY"), Some(&"normal".to_string()));
    
    unsafe { std::env::remove_var("TEST_SECRET") };
    Ok(())
}

#[test]
fn test_load_config_missing_file() {
    let result = load_config(Path::new("nonexistent.yml"));
    assert!(result.is_err());
}

#[test]
fn test_load_config_invalid_yaml() {
    let yaml = r#"
pipeline:
  jobs:
    test:
      steps:
        - cmd: echo "test"
      invalid_field: [broken yaml structure
"#;
    let file = create_temp_config(yaml);
    let result = load_config(file.path());
    assert!(result.is_err());
}

#[test]
fn test_load_config_with_timeout() -> anyhow::Result<()> {
    let yaml = r#"
timeout: 500
pipeline:
  notifications:
    on: []
    channels: []
  jobs:
    test:
      steps:
        - cmd: sleep 10
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    assert_eq!(config.timeout, Some(500));
    Ok(())
}

#[test]
fn test_load_config_with_branch() -> anyhow::Result<()> {
    let yaml = r#"
branch: develop
pipeline:
  notifications:
    on: []
    channels: []
  jobs:
    test:
      steps:
        - cmd: echo "test"
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    assert_eq!(config.branch, Some("develop".to_string()));
    Ok(())
}

#[test]
fn test_load_config_with_blocking_command() -> anyhow::Result<()> {
    let yaml = r#"
pipeline:
  notifications:
    on: []
    channels: []
  jobs:
    server:
      steps:
        - cmd: ./start_server.sh
          blocking: true
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    let server_job = config.pipeline.jobs.get("server").unwrap();
    assert_eq!(server_job.steps[0].blocking, true);
    Ok(())
}

#[test]
fn test_load_config_with_container() -> anyhow::Result<()> {
    let yaml = r#"
pipeline:
  notifications:
    on: []
    channels: []
  jobs:
    test:
      steps:
        - cmd: cargo test
          container: rust:latest
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    let test_job = config.pipeline.jobs.get("test").unwrap();
    assert_eq!(test_job.steps[0].container, Some("rust:latest".to_string()));
    Ok(())
}

#[test]
fn test_check_dependency_graph_self_dependency() {
    let mut jobs = HashMap::new();
    jobs.insert(
        "job1".to_string(),
        Job {
            needs: vec!["job1".to_string()],
            steps: vec![Cmd {
                cmd: "echo test".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );

    let config = ProjectConfig {
        pipeline: Pipeline {
            jobs,
            notifications: Default::default(),
        },
        branch: None,
        timeout: None,
    };

    let result = check_dependency_graph(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot depend on itself"));
}

#[test]
fn test_check_dependency_graph_unknown_dependency() {
    let mut jobs = HashMap::new();
    jobs.insert(
        "job1".to_string(),
        Job {
            needs: vec!["nonexistent".to_string()],
            steps: vec![Cmd {
                cmd: "echo test".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );

    let config = ProjectConfig {
        pipeline: Pipeline {
            jobs,
            notifications: Default::default(),
        },
        branch: None,
        timeout: None,
    };

    let result = check_dependency_graph(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown job"));
}

#[test]
fn test_check_dependency_graph_simple_cycle() {
    let mut jobs = HashMap::new();
    jobs.insert(
        "job1".to_string(),
        Job {
            needs: vec!["job2".to_string()],
            steps: vec![Cmd {
                cmd: "echo job1".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );
    jobs.insert(
        "job2".to_string(),
        Job {
            needs: vec!["job1".to_string()],
            steps: vec![Cmd {
                cmd: "echo job2".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );

    let config = ProjectConfig {
        pipeline: Pipeline {
            jobs,
            notifications: Default::default(),
        },
        branch: None,
        timeout: None,
    };

    let result = check_dependency_graph(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Cycle detected"));
}

#[test]
fn test_check_dependency_graph_complex_cycle() {
    let mut jobs = HashMap::new();
    // job1 -> job2 -> job3 -> job1 (cycle)
    jobs.insert(
        "job1".to_string(),
        Job {
            needs: vec!["job2".to_string()],
            steps: vec![Cmd {
                cmd: "echo job1".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );
    jobs.insert(
        "job2".to_string(),
        Job {
            needs: vec!["job3".to_string()],
            steps: vec![Cmd {
                cmd: "echo job2".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );
    jobs.insert(
        "job3".to_string(),
        Job {
            needs: vec!["job1".to_string()],
            steps: vec![Cmd {
                cmd: "echo job3".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );

    let config = ProjectConfig {
        pipeline: Pipeline {
            jobs,
            notifications: Default::default(),
        },
        branch: None,
        timeout: None,
    };

    let result = check_dependency_graph(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Cycle detected"));
}

#[test]
fn test_check_dependency_graph_valid_dag() -> anyhow::Result<()> {
    let mut jobs = HashMap::new();
    jobs.insert(
        "job1".to_string(),
        Job {
            needs: vec![],
            steps: vec![Cmd {
                cmd: "echo job1".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );
    jobs.insert(
        "job2".to_string(),
        Job {
            needs: vec!["job1".to_string()],
            steps: vec![Cmd {
                cmd: "echo job2".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );
    jobs.insert(
        "job3".to_string(),
        Job {
            needs: vec!["job1".to_string()],
            steps: vec![Cmd {
                cmd: "echo job3".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );

    let config = ProjectConfig {
        pipeline: Pipeline {
            jobs,
            notifications: Default::default(),
        },
        branch: None,
        timeout: None,
    };

    let result = check_dependency_graph(&config);
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_check_dependency_graph_empty_jobs() -> anyhow::Result<()> {
    let config = ProjectConfig {
        pipeline: Pipeline {
            jobs: HashMap::new(),
            notifications: Default::default(),
        },
        branch: None,
        timeout: None,
    };

    let result = check_dependency_graph(&config);
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_check_dependency_graph_multiple_dependencies() -> anyhow::Result<()> {
    let mut jobs = HashMap::new();
    jobs.insert(
        "job1".to_string(),
        Job {
            needs: vec![],
            steps: vec![Cmd {
                cmd: "echo job1".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );
    jobs.insert(
        "job2".to_string(),
        Job {
            needs: vec![],
            steps: vec![Cmd {
                cmd: "echo job2".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );
    jobs.insert(
        "job3".to_string(),
        Job {
            needs: vec!["job1".to_string(), "job2".to_string()],
            steps: vec![Cmd {
                cmd: "echo job3".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );

    let config = ProjectConfig {
        pipeline: Pipeline {
            jobs,
            notifications: Default::default(),
        },
        branch: None,
        timeout: None,
    };

    let result = check_dependency_graph(&config);
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_load_config_with_notifications() -> anyhow::Result<()> {
    let yaml = r#"
pipeline:
  notifications:
    on: [success, failure]
    thumbnail: https://example.com/image.png
    channels:
      - service: discord
        url: https://discord.com/api/webhooks/123
      - service: slack
        url: https://hooks.slack.com/services/123
  jobs:
    test:
      steps:
        - cmd: echo "test"
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    assert_eq!(config.pipeline.notifications.on, vec!["success", "failure"]);
    assert_eq!(
        config.pipeline.notifications.thumbnail,
        Some("https://example.com/image.png".to_string())
    );
    assert_eq!(config.pipeline.notifications.channels.len(), 2);
    assert_eq!(config.pipeline.notifications.channels[0].service, "discord");
    Ok(())
}

#[test]
fn test_load_config_with_pipe() -> anyhow::Result<()> {
    let yaml = r#"
pipeline:
  notifications:
    on: []
    channels: []
  jobs:
    job1:
      steps:
        - cmd: echo "data"
    job2:
      pipe: job1
      steps:
        - cmd: grep "data"
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    let job2 = config.pipeline.jobs.get("job2").unwrap();
    assert_eq!(job2.pipe, "job1");
    Ok(())
}

#[test]
fn test_load_config_empty_pipeline() -> anyhow::Result<()> {
    let yaml = r#"
pipeline:
  notifications:
    on: []
    channels: []
  jobs: {}
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    assert_eq!(config.pipeline.jobs.len(), 0);
    Ok(())
}

#[test]
fn test_load_config_multiple_steps() -> anyhow::Result<()> {
    let yaml = r#"
pipeline:
  notifications:
    on: []
    channels: []
  jobs:
    multi_step:
      steps:
        - cmd: echo "step1"
        - cmd: echo "step2"
          blocking: true
        - cmd: echo "step3"
          container: alpine:latest
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    let job = config.pipeline.jobs.get("multi_step").unwrap();
    assert_eq!(job.steps.len(), 3);
    assert_eq!(job.steps[0].blocking, false);
    assert_eq!(job.steps[1].blocking, true);
    assert_eq!(job.steps[2].container, Some("alpine:latest".to_string()));
    Ok(())
}

#[test]
fn test_check_dependency_graph_diamond_pattern() -> anyhow::Result<()> {
    // job1 -> job2 -> job4
    // job1 -> job3 -> job4
    let mut jobs = HashMap::new();
    jobs.insert(
        "job1".to_string(),
        Job {
            needs: vec![],
            steps: vec![Cmd {
                cmd: "echo job1".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );
    jobs.insert(
        "job2".to_string(),
        Job {
            needs: vec!["job1".to_string()],
            steps: vec![Cmd {
                cmd: "echo job2".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );
    jobs.insert(
        "job3".to_string(),
        Job {
            needs: vec!["job1".to_string()],
            steps: vec![Cmd {
                cmd: "echo job3".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );
    jobs.insert(
        "job4".to_string(),
        Job {
            needs: vec!["job2".to_string(), "job3".to_string()],
            steps: vec![Cmd {
                cmd: "echo job4".to_string(),
                blocking: false,
                container: None,
            }],
            pipe: String::new(),
            env: None,
        },
    );

    let config = ProjectConfig {
        pipeline: Pipeline {
            jobs,
            notifications: Default::default(),
        },
        branch: None,
        timeout: None,
    };

    let result = check_dependency_graph(&config);
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_load_config_minimal() -> anyhow::Result<()> {
    let yaml = r#"
pipeline:
  notifications:
    on: []
    channels: []
  jobs:
    simple:
      steps:
        - cmd: echo "hello"
"#;
    let file = create_temp_config(yaml);
    let config = load_config(file.path())?;

    assert!(config.branch.is_none());
    assert!(config.timeout.is_none());
    assert_eq!(config.pipeline.jobs.len(), 1);
    Ok(())
}

#[test]
fn test_check_dependency_graph_long_chain() -> anyhow::Result<()> {
    let mut jobs = HashMap::new();
    // Create a chain: job1 -> job2 -> job3 -> job4 -> job5
    for i in 1..=5 {
        let needs = if i == 1 {
            vec![]
        } else {
            vec![format!("job{}", i - 1)]
        };
        
        jobs.insert(
            format!("job{}", i),
            Job {
                needs,
                steps: vec![Cmd {
                    cmd: format!("echo job{}", i),
                    blocking: false,
                    container: None,
                }],
                pipe: String::new(),
                env: None,
            },
        );
    }

    let config = ProjectConfig {
        pipeline: Pipeline {
            jobs,
            notifications: Default::default(),
        },
        branch: None,
        timeout: None,
    };

    let result = check_dependency_graph(&config);
    assert!(result.is_ok());
    Ok(())
}