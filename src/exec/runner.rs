#![allow(dead_code)]
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use anyhow::Result;
use tokio::sync::Mutex;

use crate::{
    config::parser::{Cmd, Job, ProjectConfig},
    core::watcher::WatchContext,
    exec::{
        command::{exec_background, exec_timeout},
        container::contain_cmd,
    },
    logging::Logger,
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
    // Construire le graph de dépendances
    let graph_map = build_dependency_graph(&ctx.config)?;
    let graph = Arc::new(Mutex::new(graph_map));

    // Queue des jobs prêts à s'exécuter
    let ready_queue = Arc::new(Mutex::new(VecDeque::new()));

    // Initialiser la queue avec les jobs qui n'ont pas de dépendances
    {
        let graph_guard = graph.lock().await;
        let mut ready_guard = ready_queue.lock().await;
        for (name, node) in graph_guard.iter() {
            if node.remaining_dependencies == 0 {
                ready_guard.push_back(name.clone());
            }
        }
    }

    // Boucle principale
    loop {
        // Extraire tous les jobs prêts d'un coup
        let ready_jobs: Vec<String> = {
            let mut queue = ready_queue.lock().await;
            if queue.is_empty() {
                break; // plus aucun job à exécuter
            }
            queue.drain(..).collect()
        };

        // Lancer tous les jobs prêts en parallèle
        let mut handles: Vec<tokio::task::JoinHandle<std::result::Result<bool, anyhow::Error>>> =
            vec![];
        for job_name in ready_jobs {
            let graph_clone = Arc::clone(&graph);
            let ready_clone = Arc::clone(&ready_queue);
            let ctx_clone = Arc::clone(&ctx);

            let (job_arc, dependents) = {
                let g = graph.lock().await;
                let node = g.get(&job_name).unwrap();
                (Arc::clone(&node.job), node.dependents.clone())
            };

            let handle = tokio::spawn(async move {
                // exec all job step
                ctx_clone.logger.job_start(&job_name).await?;
                for step in &job_arc.steps {
                    if let Err(e) = run_step(&ctx_clone, step, &job_arc.env).await {
                        ctx_clone
                            .logger
                            .error(&format!("Job {} failed: {}", job_name, e))
                            .await?;
                        return Ok(false);
                    }
                }

                ctx_clone
                    .logger
                    .info(&format!("Job {} succeeded", job_name))
                    .await?;

                // Mettre à jour les dépendants
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

        // Attendre que tous les jobs lancés se terminent
        for h in handles {
            match h.await {
                Ok(_) => {}
                Err(e) => ctx.logger.error(&format!("pipeline failed: {e}")).await?,
            }
        }
    }

    Ok(())
}

async fn run_step(
    ctx: &WatchContext,
    step: &Cmd,
    env: &Option<HashMap<String, String>>,
) -> Result<()> {
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
        timeout_process(
            ctx,
            parts,
            &ctx.logger,
            env.clone(),
            ctx.config.timeout.unwrap_or(DEFAULT_TIMEOUT),
        )
        .await?;
    }
    Ok(())
}

// pub async fn run_update(ctx: &WatchContext) -> Result<(), anyhow::Error> {
//     let logger = Logger::new(&ctx.log_path()).await?;

//     logger.info("Update started").await?;
//     let update_commands = &ctx.config.update.steps;

//     if update_commands.is_empty() {
//         logger
//             .warning("No command to execute (check your fleet.yml file)")
//             .await?;
//         return Ok(());
//     }

//     let default_timeout = ctx.config.timeout.unwrap_or(DEFAULT_TIMEOUT);

//     for (i, cmd_line) in update_commands.iter().enumerate() {
//         logger
//             .info(&format!("Executing command {} : {}", i + 1, cmd_line.cmd))
//             .await?;
//         let parts = shell_words::split(&cmd_line.cmd)?;
//         if parts.is_empty() {
//             logger.info("Empty command, ignore ...").await?;
//             continue;
//         }

//         let env = ctx.config.update.env.clone();
//         let log_path = logger.get_path()?;

//         if cmd_line.container.is_some() {
//             let image = cmd_line.container.clone().unwrap();
//             if cmd_line.blocking {
//                 //blocking command => run in background and forget
//                 // background_process(ctx, parts, &logger, env).await?;
//                 contain_cmd(
//                     &image,
//                     parts,
//                     env,
//                     &ctx.project_dir,
//                     &log_path,
//                     &logger,
//                     None,
//                 )
//                 .await?;
//             } else {
//                 //classic command w timeout
//                 contain_cmd(
//                     &image,
//                     parts,
//                     env,
//                     &ctx.project_dir,
//                     &log_path,
//                     &logger,
//                     Some(default_timeout),
//                 )
//                 .await?;
//                 // timeout_process(ctx, parts, &logger, env, default_timeout).await?;
//             }
//         } else if cmd_line.blocking {
//             background_process(ctx, parts, &logger, env).await?;
//         } else {
//             timeout_process(ctx, parts, &logger, env, default_timeout).await?;
//         }
//     }

//     logger.info("=== Update finished successfully ===").await?;

//     Ok(())
// }

async fn background_process(
    ctx: &WatchContext,
    parts: Vec<String>,
    logger: &Logger,
    env: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    match exec_background(parts.clone(), ctx, logger, env).await {
        Ok(_) => {}
        Err(e) => {
            logger.error(&format!("Failed: {e}")).await?;
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
) -> Result<(), anyhow::Error> {
    match exec_timeout(parts.clone(), ctx, logger, default_timeout, env).await {
        Ok(_) => {}
        Err(e) => {
            logger.error(&format!("Failed: {e}")).await?;
        }
    };
    Ok(())
}

// pub async fn run_conflict_process(ctx: &WatchContext) -> Result<(), anyhow::Error> {
//     let logger = Logger::new(&ctx.log_path()).await?;

//     logger.info("Conflict process started").await?;
//     let conflict_commands = &ctx.config.on_conflict.steps;

//     if conflict_commands.is_empty() {
//         logger
//             .warning("No command to execute (check your fleet.yml file)")
//             .await?;
//         return Ok(());
//     }

//     let default_timeout = ctx.config.timeout.unwrap_or(DEFAULT_TIMEOUT);

//     for (i, cmd_line) in conflict_commands.iter().enumerate() {
//         logger
//             .info(&format!("Executing command {} : {}", i + 1, cmd_line.cmd))
//             .await?;
//         let parts = shell_words::split(&cmd_line.cmd)?;
//         if parts.is_empty() {
//             logger.info("Empty command, ignore ...").await?;
//             continue;
//         }

//         let env = ctx.config.update.env.clone();

//         if cmd_line.blocking {
//             //blocking command => run in background and forget
//             exec_background(parts, ctx, &logger, env).await?;
//         } else {
//             //classic command w timeout
//             exec_timeout(parts, ctx, &logger, default_timeout, env).await?;
//         }
//     }

//     logger
//         .info("=== Conflit process finished successfully ===")
//         .await?;

//     Ok(())
// }
