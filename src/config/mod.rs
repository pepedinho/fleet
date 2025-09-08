pub mod parser;
use std::{collections::HashMap, fs::OpenOptions};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{core::watcher::WatchContext, exec::OutpuStrategy};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Cmd {
    pub cmd: String,
    #[serde(default)]
    pub blocking: bool,
    #[serde(default)]
    pub container: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Job {
    #[serde(default)]
    pub needs: Vec<String>,
    #[serde(default)]
    pub pipe: String,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    pub steps: Vec<Cmd>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct Pipeline {
    pub notifications: Notification,
    pub jobs: HashMap<String, Job>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct ProjectConfig {
    pub pipeline: Pipeline,

    #[serde(default)]
    pub branch: Option<String>,

    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct ConfChannel {
    pub service: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct Notification {
    pub on: Vec<String>,
    pub channels: Vec<ConfChannel>,
}

fn find_pipe_dependance(ctx: &WatchContext, job_name: &str) -> Option<Cmd> {
    for (n, j) in &ctx.config.pipeline.jobs {
        if !j.pipe.is_empty() && n == job_name {
            println!("debug: job {n} has linked as output for job {}", j.pipe);
            let target = j.steps.last().cloned();
            println!("[2]'{target:?}' has design as target");
            return target;
        }
    }
    None
}

impl ProjectConfig {
    pub fn drop_strategy(
        &self,
        job_name: &str,
        ctx: &WatchContext,
        last: &Option<&Cmd>,
    ) -> Result<OutpuStrategy> {
        let log_path = ctx.log_path();
        let stdout_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        let stderr_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        for (n, j) in &self.pipeline.jobs {
            if !j.pipe.is_empty() && j.pipe == job_name {
                println!("debug: pipe link: job {n} is now linked with job {job_name}");
                let cmd = ctx
                    .config
                    .pipeline
                    .jobs
                    .get(job_name)
                    .unwrap()
                    .steps
                    .last()
                    .unwrap();
                let target = String::from(&j.steps.last().unwrap().cmd);
                println!("[1]'{target}' has design as target");
                return Ok(OutpuStrategy::ToPipeOut {
                    cmd: cmd.cmd.clone(),
                    stdout: stdout_file,
                    stderr: stderr_file,
                    target: target,
                });
            }
        }
        if let Some(depend) = find_pipe_dependance(ctx, job_name) {
            // job who depend another
            return Ok(OutpuStrategy::ToPipeIn {
                stdout: stdout_file,
                stderr: stderr_file,
                target: depend.cmd,
            });
        }
        println!("debug: no piped job: {job_name}: '{last:?}'");
        Ok(OutpuStrategy::ToFiles {
            stdout: stdout_file,
            stderr: stderr_file,
        })
    }
}
