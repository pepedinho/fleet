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
    pub branches: Option<Vec<String>>,

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
    #[serde(default)]
    pub thumbnail: Option<String>,
}

fn find_pipe_dependance(ctx: &WatchContext, job_name: &str) -> Option<Cmd> {
    for (n, j) in &ctx.config.pipeline.jobs {
        if !j.pipe.is_empty() && n == job_name {
            let target = j.steps.last().cloned();
            return target;
        }
    }
    None
}

impl ProjectConfig {
    pub fn drop_strategy(&self, job_name: &str, ctx: &WatchContext) -> Result<OutpuStrategy> {
        let log_path = ctx.log_path();
        let stdout_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        let stderr_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        for j in self.pipeline.jobs.values() {
            if !j.pipe.is_empty() && j.pipe == job_name {
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
                    target,
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
        Ok(OutpuStrategy::ToFiles {
            stdout: stdout_file,
            stderr: stderr_file,
        })
    }
}
