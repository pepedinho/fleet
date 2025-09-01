#![allow(dead_code)]
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use anyhow::Result;
use tokio::sync::Mutex;

use crate::{
    config::{Cmd, Job, ProjectConfig, parser::check_dependency_graph},
    core::watcher::WatchContext,
    exec::{
        command::{CommandOutput, exec_background, exec_timeout},
        container::contain_cmd,
        metrics::ExecMetrics,
    },
    logging::Logger,
    notifications::sender::{discord_send_failure, discord_send_succes},
};

const DEFAULT_TIMEOUT: u64 = 300;
struct JobNode {
    job: Arc<Job>,
    depend_on: Vec<String>, // names of jobs this job depends on
    remaining_dependencies: usize,
    dependents: Vec<String>, // names of jobs that depend on this one
}

fn build_dependency_graph(config: &ProjectConfig) -> Result<HashMap<String, JobNode>> {
    let mut graph = HashMap::new();

    for (name, job) in config.pipeline.jobs.iter() {
        if job.needs.contains(name) {
            return Err(anyhow::anyhow!("Job: {} cannot depend on itself", name));
        }
        let node = JobNode {
            job: Arc::new(job.clone()),
            depend_on: job.needs.clone(),
            remaining_dependencies: job.needs.len(),
            dependents: vec![],
        };
        graph.insert(name.clone(), node);
    }

    for name in graph.keys().cloned().collect::<Vec<_>>() {
        let depend_on = graph[&name].depend_on.clone();
        for dep in depend_on {
            if let Some(dep_node) = graph.get_mut(&dep) {
                dep_node.dependents.push(name.clone());
            } else {
                anyhow::bail!("Job '{}' depends on unknown job '{}'", name, dep);
            }
        }
    }
    Ok(graph)
}

pub async fn run_pipeline(ctx: Arc<WatchContext>) -> Result<()> {
    let metrics = Arc::new(tokio::sync::Mutex::new(ExecMetrics::new(
        &ctx.id,
        &ctx.repo.name,
        ctx.logger.clone(),
    )));

    check_dependency_graph(&ctx.config)?;
    let graph_map = build_dependency_graph(&ctx.config)?;
    let graph = Arc::new(Mutex::new(graph_map));

    let ready_queue = Arc::new(Mutex::new(VecDeque::new()));

    {
        let graph_guard = graph.lock().await;
        let mut ready_guard = ready_queue.lock().await;
        for (name, node) in graph_guard.iter() {
            if node.remaining_dependencies == 0 {
                ready_guard.push_back(name.clone());
            }
        }
    }

    loop {
        let ready_jobs: Vec<String> = {
            let mut queue = ready_queue.lock().await;
            if queue.is_empty() {
                break;
            }
            queue.drain(..).collect()
        };

        //run all the ready job in paralel
        let mut handles: Vec<tokio::task::JoinHandle<std::result::Result<bool, anyhow::Error>>> =
            vec![];
        for job_name in ready_jobs {
            let graph_clone = Arc::clone(&graph);
            let ready_clone = Arc::clone(&ready_queue);
            let ctx_clone = Arc::clone(&ctx);
            let metrics_clone = Arc::clone(&metrics);

            let (job_arc, dependents) = {
                let g = graph.lock().await;
                let node = g.get(&job_name).unwrap();
                (Arc::clone(&node.job), node.dependents.clone())
            };

            let handle = tokio::spawn(async move {
                {
                    let mut m = metrics_clone.lock().await;
                    m.job_started(&job_name);
                }
                // exec all job step
                println!("logger job start");
                ctx_clone.logger.job_start(&job_name).await?;
                for step in &job_arc.steps {
                    match run_step(&ctx_clone, step, &job_arc.env).await {
                        Ok(Some(output)) => {
                            let mut m = metrics_clone.lock().await;
                            m.sys_push(&job_name, output.cpu_usage, output.mem_usage_kb);
                        }
                        Ok(None) => {}
                        Err(e) => {
                            if ctx_clone
                                .config
                                .pipeline
                                .notifications
                                .on
                                .contains(&"failure".to_string())
                            {
                                let err = e.to_string();
                                let lines: Vec<&str> = err.lines().collect();
                                let first_line = lines.first().unwrap_or(&"");
                                let second_line = lines.get(1).unwrap_or(&"");
                                discord_send_failure(
                                    &ctx_clone,
                                    &format!(
                                        "**Job** `{}` **failed**
                                        {}
                                        **Error:** `{}`",
                                        job_name, first_line, second_line,
                                    ),
                                )
                                .await?;
                            }
                            ctx_clone
                                .logger
                                .error(&format!("Job {} failed", job_name))
                                .await?;
                            {
                                let mut m = metrics_clone.lock().await;
                                m.job_finished(&job_name, false);
                            }
                            return Err(e);
                        }
                    }
                }

                {
                    let mut m = metrics_clone.lock().await;
                    m.job_finished(&job_name, true);
                }

                ctx_clone
                    .logger
                    .info(&format!("Job {} succeeded", job_name))
                    .await?;

                // update dependents
                let mut g = graph_clone.lock().await;
                for dep_name in &dependents {
                    let dep_node = g.get_mut(dep_name).unwrap();
                    if dep_node.remaining_dependencies != usize::MAX {
                        dep_node.remaining_dependencies -= 1;
                        if dep_node.remaining_dependencies == 0 {
                            ready_clone.lock().await.push_back(dep_name.clone());
                        }
                    }
                }

                ctx_clone.logger.job_end(&job_name).await?;
                Ok(true)
            });

            handles.push(handle);
        }

        // wait all runnin jobs
        for h in handles {
            match h.await {
                Ok(inner) => match inner {
                    Ok(_) => {}
                    Err(e) => {
                        let mut m = metrics.lock().await;
                        m.finalize();
                        m.save().await.ok();
                        ctx.logger.error(&format!("pipeline failed: {e}")).await?;
                        return Err(anyhow::anyhow!("pipeline failed: {e}"));
                    }
                },
                Err(e) => {
                    let mut m = metrics.lock().await;
                    m.finalize();
                    m.save().await.ok();
                    ctx.logger.error(&format!("pipeline failed: {e}")).await?;
                    return Err(anyhow::anyhow!("pipeline failed: {e}"));
                }
            }
        }
    }

    {
        let mut m = metrics.lock().await;
        m.finalize();
        m.save().await?;
        discord_send_succes(&ctx, &m).await?;
    }

    Ok(())
}

async fn run_step(
    ctx: &WatchContext,
    step: &Cmd,
    env: &Option<HashMap<String, String>>,
) -> Result<Option<CommandOutput>> {
    let parts = shell_words::split(&step.cmd)?;

    if let Some(container) = &step.container {
        contain_cmd(
            container,
            parts,
            env.clone(),
            &ctx.project_dir,
            &ctx.log_path().display().to_string(),
            &ctx.logger,
            Some(ctx.config.timeout.unwrap_or(DEFAULT_TIMEOUT)),
        )
        .await?;
    } else if step.blocking {
        background_process(ctx, parts, &ctx.logger, env.clone()).await?;
    } else {
        return Ok(Some(
            timeout_process(
                ctx,
                parts,
                &ctx.logger,
                env.clone(),
                ctx.config.timeout.unwrap_or(DEFAULT_TIMEOUT),
            )
            .await?,
        ));
    }
    Ok(None)
}

async fn background_process(
    ctx: &WatchContext,
    parts: Vec<String>,
    logger: &Logger,
    env: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    match exec_background(parts.clone(), ctx, logger, env).await {
        Ok(_) => {}
        Err(e) => {
            return Err(e);
        }
    };
    Ok(())
}

async fn timeout_process(
    ctx: &WatchContext,
    parts: Vec<String>,
    logger: &Logger,
    env: Option<HashMap<String, String>>,
    default_timeout: u64,
) -> Result<CommandOutput, anyhow::Error> {
    match exec_timeout(parts.clone(), ctx, logger, default_timeout, env).await {
        Ok(o) => Ok(o),
        Err(e) => Err(e),
    }
}
