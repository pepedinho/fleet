#![allow(dead_code)]
use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use tokio::sync::Mutex;

use crate::{
    config::{Cmd, Job, ProjectConfig},
    core::watcher::WatchContext,
    exec::{
        OutpuStrategy, PipeRegistry,
        command::{CommandOutput, exec_background, exec_timeout},
        container::contain_cmd,
    },
    log::logger::Logger,
};

const DEFAULT_TIMEOUT: u64 = 300;
pub struct JobNode {
    pub job: Arc<Job>,
    pub depend_on: Vec<String>, // names of jobs this job depends on
    pub remaining_dependencies: usize,
    pub dependents: Vec<String>, // names of jobs that depend on this one
}

pub fn build_dependency_graph(config: &ProjectConfig) -> Result<HashMap<String, JobNode>> {
    let mut graph = HashMap::new();

    for (name, job) in config.pipeline.jobs.iter() {
        if job.needs.contains(name) {
            return Err(anyhow::anyhow!("Job: {} cannot depend on itself", name));
        }

        let job_needs = if !job.pipe.is_empty() && !job.needs.contains(&job.pipe) {
            let mut j = job.needs.clone();
            j.push(job.pipe.clone());
            j
        } else {
            job.needs.clone()
        };

        let node = JobNode {
            job: Arc::new(job.clone()),
            depend_on: job_needs.clone(),
            remaining_dependencies: job_needs.len(),
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

pub async fn run_step(
    ctx: &WatchContext,
    step: &Cmd,
    env: &Option<HashMap<String, String>>,
    output_strategy: &OutpuStrategy,
    pipe_registry: Arc<Mutex<PipeRegistry>>,
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
                output_strategy,
                pipe_registry,
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
    output_strategy: &OutpuStrategy,
    pipe_registry: Arc<Mutex<PipeRegistry>>,
) -> Result<CommandOutput, anyhow::Error> {
    match exec_timeout(
        parts.clone(),
        ctx,
        logger,
        default_timeout,
        env,
        output_strategy,
        pipe_registry,
    )
    .await
    {
        Ok(o) => Ok(o),
        Err(e) => Err(e),
    }
}
