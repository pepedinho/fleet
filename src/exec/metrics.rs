#![allow(dead_code)]
use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Utc};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self},
    io::AsyncWriteExt,
};

use crate::logging::Logger;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetrics {
    pub name: String,
    pub status: JobStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u128>,
    pub cpu_usage: f32,
    pub mem_usage_kb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecMetrics {
    pub project_id: String,
    pub project_name: String,

    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u128>,

    pub cpu_usage: f32,
    pub mem_usage_kb: u64,

    pub jobs: HashMap<String, JobMetrics>,

    #[serde(skip, default = "Logger::placeholder")]
    pub logger: Logger,
}

impl ExecMetrics {
    pub fn new(project_id: &str, project_name: &str, logger: Logger) -> Self {
        Self {
            project_id: project_id.to_string(),
            project_name: project_name.to_string(),
            started_at: Utc::now(),
            finished_at: None,
            duration_ms: None,
            cpu_usage: 0.0,
            mem_usage_kb: 0,
            jobs: std::collections::HashMap::new(),
            logger,
        }
    }

    /// call at the end of the pipeline
    pub fn finalize(&mut self) {
        let end = Utc::now();
        self.finished_at = Some(end);
        self.duration_ms = Some(
            end.signed_duration_since(self.started_at)
                .num_milliseconds() as u128,
        );
    }

    /// start job (just insert it with Running status)
    pub fn job_started(&mut self, name: &str) {
        self.jobs.insert(
            name.to_string(),
            JobMetrics {
                name: name.to_string(),
                status: JobStatus::Running,
                started_at: Utc::now(),
                finished_at: None,
                duration_ms: None,
                cpu_usage: 0.0,
                mem_usage_kb: 0,
            },
        );
    }

    pub fn job_finished(&mut self, name: &str, ok: bool) {
        if let Some(j) = self.jobs.get_mut(name) {
            let end = Utc::now();
            j.finished_at = Some(end);
            j.duration_ms =
                Some(end.signed_duration_since(j.started_at).num_milliseconds() as u128);
            j.status = if ok {
                JobStatus::Succeeded
            } else {
                JobStatus::Failed
            }
        }
    }

    // -----------------------
    // Metrics persistance
    // -----------------------

    pub async fn ensure_metrics_dir() -> anyhow::Result<PathBuf> {
        let home = home_dir().ok_or_else(|| anyhow::anyhow!("Failed to find HOME directory"))?;
        let dir = home.join(".fleet").join("metrics");
        if !fs::try_exists(&dir).await? {
            fs::create_dir_all(&dir).await?;
        }
        Ok(dir)
    }

    pub async fn open_metrics_file(project_id: &str) -> anyhow::Result<tokio::fs::File> {
        let dir = Self::ensure_metrics_dir().await?;
        let path = dir.join(format!("{project_id}.ndjson"));
        let file = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .await?;
        Ok(file)
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        let mut file = Self::open_metrics_file(&self.project_id).await?;
        let line = serde_json::to_string(self)?;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        Ok(())
    }
}
