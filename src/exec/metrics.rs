#![allow(dead_code)]
use core::f32;
use std::{collections::HashMap, path::PathBuf, time::Duration};

use chrono::{DateTime, Utc};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self},
    io::AsyncWriteExt,
    time::sleep,
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
    pub mem_usage: f32,
    pub mem_usage_kb: u64,
    pub max_cpu: f32,
    pub max_mem: f32,
    #[serde(skip)]
    pub buf: Vec<(f32, u64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecMetrics {
    pub project_id: String,
    pub project_name: String,

    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u128>,

    pub cpu_usage: f32,
    pub mem_usage: f32,
    pub mem_usage_kb: u64,
    pub max_cpu: f32,
    pub max_mem: f32,

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
            mem_usage: 0.0,
            mem_usage_kb: 0,
            max_cpu: 0.0,
            max_mem: 0.0,
            jobs: std::collections::HashMap::new(),
            logger,
        }
    }

    pub fn sys_push(&mut self, name: &str, cpu: f32, mem: u64) {
        if let Some(j) = self.jobs.get_mut(name) {
            j.buf.push((cpu, mem));
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
        let v: Vec<(f32, f32, u64)> = self
            .jobs
            .values()
            .map(|v| (v.cpu_usage, v.mem_usage, v.mem_usage_kb))
            .collect();

        if !v.is_empty() {
            let mut sys = sysinfo::System::new_all();
            sys.refresh_memory();
            let cpu_sum: f32 = v.iter().map(|(cpu, _, _)| *cpu).sum();
            let cpu_count = v.len() as f32;
            let average_cpu = cpu_sum / cpu_count;

            let max_cpu = v
                .iter()
                .map(|(cpu, _, _)| *cpu)
                .fold(0.0_f32, |a, b| a.max(b));

            let mem_usage_kb: Vec<u64> = v.iter().map(|(_, _, mem)| *mem).collect();
            let avg_mem_usage_kb = mem_usage_kb.iter().sum::<u64>();

            let mem_percentages: Vec<f32> = v.iter().map(|(_, mem, _)| *mem).collect(); // déjà en %
            let average_mem_percent: f32 =
                mem_percentages.iter().sum::<f32>() / mem_percentages.len() as f32;
            let max_mem_percent: f32 = mem_percentages.iter().fold(0.0, |a, b| a.max(*b));

            self.cpu_usage = average_cpu;
            self.mem_usage_kb = avg_mem_usage_kb;
            self.mem_usage = average_mem_percent;
            self.max_cpu = max_cpu;
            self.max_mem = max_mem_percent;
        } else {
            self.cpu_usage = 0.0;
            self.mem_usage_kb = 0;
            self.max_cpu = 0.0;
            self.max_mem = 0.0;
        }
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
                mem_usage: 0.0,
                mem_usage_kb: 0,
                max_cpu: 0.0,
                max_mem: 0.0,
                buf: Vec::new(),
            },
        );
    }

    pub fn job_finished(&mut self, name: &str, ok: bool) {
        if let Some(j) = self.jobs.get_mut(name) {
            if !j.buf.is_empty() {
                let mut sys = sysinfo::System::new_all();
                sys.refresh_memory();
                let total_memory_kb = sys.total_memory() as f32;
                let cpu_sum: f32 = j.buf.iter().map(|(value, _)| *value).sum();
                let count = j.buf.len() as f32;

                let average_cpu = cpu_sum / count;
                let max_cpu = j.buf.iter().map(|(v, _)| *v).fold(0.0_f32, |a, b| a.max(b));

                let mem_usage_kb: Vec<u64> = j.buf.iter().map(|(_, mem)| *mem).collect();
                let avg_mem_usage_kb = mem_usage_kb.iter().sum::<u64>();

                let mem_percentages: Vec<f32> = j
                    .buf
                    .iter()
                    .map(|(_, mem)| (*mem as f32 / total_memory_kb) * 100.0)
                    .collect();
                let average_mem_percent: f32 =
                    mem_percentages.iter().sum::<f32>() / mem_percentages.len() as f32;
                let max_mem_percent: f32 = mem_percentages.iter().fold(0.0, |a, b| a.max(*b));

                j.cpu_usage = average_cpu;
                j.mem_usage_kb = avg_mem_usage_kb;
                j.mem_usage = average_mem_percent;
                j.max_mem = max_mem_percent;
                j.max_cpu = max_cpu;
            }
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

    pub fn get_metrics_path_by_id(id: &str) -> anyhow::Result<PathBuf> {
        let home = home_dir().ok_or_else(|| anyhow::anyhow!("Failed to find HOME directory"))?;
        let dir = home.join(".fleet").join("metrics").join(id);
        Ok(dir)
    }

    pub fn rm_metrics_by_id(id: &str) -> anyhow::Result<()> {
        std::fs::remove_file(ExecMetrics::get_metrics_path_by_id(id)?)?;
        Ok(())
    }

    pub async fn open_metrics_file(project_id: &str) -> anyhow::Result<tokio::fs::File> {
        let dir = Self::ensure_metrics_dir().await?;
        let path = dir.join(format!("{project_id}.ndjson"));
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
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

pub async fn monitor_process(pid: u32) -> (f32, u64) {
    let mut sys = sysinfo::System::new_all();
    let mut samples_cpu = vec![];
    let mut max_mem = 0;

    loop {
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        if let Some(proc) = sys.process(sysinfo::Pid::from(pid as usize)) {
            samples_cpu.push(proc.cpu_usage());
            max_mem = max_mem.max(proc.memory());
        } else {
            // Process finished
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }

    let avg_cpu = if samples_cpu.is_empty() {
        0.0
    } else {
        samples_cpu.iter().sum::<f32>() / samples_cpu.len() as f32
    };

    (avg_cpu, max_mem)
}
