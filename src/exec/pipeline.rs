use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use anyhow::Result;
use tokio::sync::Mutex;

use crate::{
    config::parser::check_dependency_graph,
    core::watcher::WatchContext,
    exec::{
        PipeRegistry,
        metrics::ExecMetrics,
        runner::{JobNode, build_dependency_graph, run_step},
    },
    notifications::sender::{discord_send_failure, discord_send_succes},
};

pub async fn run_pipeline(ctx: Arc<WatchContext>) -> Result<()> {
    let metrics = Arc::new(tokio::sync::Mutex::new(ExecMetrics::new(
        &ctx.id,
        &ctx.repo.name,
        ctx.logger.clone(),
    )));

    let pipe_registry = Arc::new(Mutex::new(PipeRegistry {
        pipes_register: HashMap::new(),
    }));

    check_dependency_graph(&ctx.config)?;
    let graph_map = build_dependency_graph(&ctx.config)?;
    let graph: Arc<Mutex<HashMap<String, JobNode>>> = Arc::new(Mutex::new(graph_map));

    let ready_queue = Arc::new(Mutex::new(VecDeque::new()));
    initialize_ready_queue(&graph, &ready_queue).await;

    loop {
        let ready_jobs = drain_ready_queue(&ready_queue).await;
        if ready_jobs.is_empty() {
            break;
        }

        // Parallel execution of ready jobs
        let handles: Vec<_> = ready_jobs
            .into_iter()
            .map(|job_name| {
                let graph_clone = Arc::clone(&graph);
                let ready_clone = Arc::clone(&ready_queue);
                let ctx_clone = Arc::clone(&ctx);
                let metrics_clone = Arc::clone(&metrics);
                let pipe_registry_clone = Arc::clone(&pipe_registry);

                tokio::spawn(run_job(
                    job_name,
                    graph_clone,
                    ready_clone,
                    ctx_clone,
                    metrics_clone,
                    pipe_registry_clone,
                ))
            })
            .collect();

        wait_jobs(handles, &metrics, &ctx).await?;
    }

    finalize_pipeline(&metrics, &ctx).await?;
    Ok(())
}

/// Init job queue with job ready to execute based on graph
async fn initialize_ready_queue(
    graph: &Arc<Mutex<HashMap<String, JobNode>>>,
    ready_queue: &Arc<Mutex<VecDeque<String>>>,
) {
    let graph_guard = graph.lock().await;
    let mut ready_guard = ready_queue.lock().await;
    for (name, node) in graph_guard.iter() {
        if node.remaining_dependencies == 0 {
            ready_guard.push_back(name.clone());
        }
    }
}

/// Empty and retrieve ready jobs from the queue
async fn drain_ready_queue(ready_queue: &Arc<Mutex<VecDeque<String>>>) -> Vec<String> {
    let mut queue = ready_queue.lock().await;
    queue.drain(..).collect()
}

/// Runs a single job with step and dependency management
async fn run_job(
    job_name: String,
    graph: Arc<Mutex<HashMap<String, JobNode>>>,
    ready_queue: Arc<Mutex<VecDeque<String>>>,
    ctx: Arc<WatchContext>,
    metrics: Arc<Mutex<ExecMetrics>>,
    pipe_registry: Arc<Mutex<PipeRegistry>>,
) -> Result<bool> {
    let (job_arc, dependents) = {
        let g = graph.lock().await;
        let node = g.get(&job_name).unwrap();
        (Arc::clone(&node.job), node.dependents.clone())
    };

    // init job metrics
    {
        let mut m = metrics.lock().await;
        m.job_started(&job_name);
    }
    ctx.logger.job_start(&job_name).await?;

    let output_strategy = ctx.config.drop_strategy(&job_name, &ctx)?;
    for step in &job_arc.steps {
        if let Err(e) = run_step(
            &ctx,
            step,
            &job_arc.env,
            &output_strategy,
            Arc::clone(&pipe_registry),
        )
        .await
        {
            handle_job_failure(&ctx, &metrics, &job_name, e).await?;
            return Err(anyhow::anyhow!("Job failed: {job_name}"));
        }
    }

    // set job  as finished in metrics
    {
        let mut m = metrics.lock().await;
        m.job_finished(&job_name, true);
    }

    ctx.logger
        .info(&format!("Job {job_name} succeeded"))
        .await?;
    update_dependents(&graph, &ready_queue, &dependents).await;
    ctx.logger.job_end(&job_name).await?;
    Ok(true)
}

async fn update_dependents(
    graph: &Arc<Mutex<HashMap<String, JobNode>>>,
    ready_queue: &Arc<Mutex<VecDeque<String>>>,
    dependents: &[String],
) {
    let mut g = graph.lock().await;
    for dep_name in dependents {
        let dep_node = g.get_mut(dep_name).unwrap();
        if dep_node.remaining_dependencies != usize::MAX {
            dep_node.remaining_dependencies -= 1;
            if dep_node.remaining_dependencies == 0 {
                ready_queue.lock().await.push_back(dep_name.clone());
            }
        }
    }
}

/// Manage job failure (log, métrics, notifications)
async fn handle_job_failure(
    ctx: &Arc<WatchContext>,
    metrics: &Arc<Mutex<ExecMetrics>>,
    job_name: &str,
    error: anyhow::Error,
) -> Result<()> {
    let mut m = metrics.lock().await;
    m.job_finished(job_name, false);
    if ctx
        .config
        .pipeline
        .notifications
        .on
        .contains(&"failure".to_string())
    {
        let err = error.to_string();
        let lines: Vec<&str> = err.lines().collect();
        let first_line = lines.first().unwrap_or(&"");
        let second_line = lines.get(1).unwrap_or(&"");
        discord_send_failure(
            ctx,
            &format!(
                "**Job** `{job_name}` **failed**
                {first_line}
                **Error:** `{second_line}`",
            ),
            &m,
        )
        .await?;
    }

    ctx.logger.error(&format!("Job {job_name} failed")).await?;
    Ok(())
}

async fn wait_jobs(
    handles: Vec<tokio::task::JoinHandle<Result<bool, anyhow::Error>>>,
    metrics: &Arc<Mutex<ExecMetrics>>,
    ctx: &Arc<WatchContext>,
) -> Result<()> {
    for h in handles {
        match h.await {
            Ok(inner) => match inner {
                Ok(_) => {}
                Err(e) => {
                    let mut m = metrics.lock().await;
                    m.finalize();
                    m.save().await.ok();
                    ctx.logger.error(&format!("Pipeline failed: {e}")).await?;
                    return Err(anyhow::anyhow!("Pipeline failed: {e}"));
                }
            },
            Err(e) => {
                let mut m = metrics.lock().await;
                m.finalize();
                m.save().await.ok();
                ctx.logger.error(&format!("Pipeline failed: {e}")).await?;
                return Err(anyhow::anyhow!("Pipeline failed: {e}"));
            }
        }
    }
    Ok(())
}

async fn finalize_pipeline(
    metrics: &Arc<Mutex<ExecMetrics>>,
    ctx: &Arc<WatchContext>,
) -> Result<()> {
    let mut m = metrics.lock().await;
    m.finalize();
    m.save().await?;
    discord_send_succes(ctx, &m).await?;
    Ok(())
}
