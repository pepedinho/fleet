use std::path::PathBuf;

use dirs::home_dir;
use tokio::{
    fs::{File, remove_file},
    io::{AsyncReadExt, AsyncSeekExt},
};

/// Per-watch logger, kept as a thin façade over `tracing`.
///
/// The file-based API (`new`, `fetchn`, `path_by_id`, ...) is retained because
/// the dashboard (`fleet stats`) and the tests read the historical
/// `~/.fleet/logs/{id}.log` files directly. Writing happens in
/// `crate::log::file_layer::FileLayer`: each method emits a `tracing` event
/// carrying the `log_path`, and the layer appends the formatted line to that
/// file.
#[derive(Debug, Clone)]
pub struct Logger {
    path: String,
}

impl Logger {
    pub async fn new(path: &std::path::Path) -> anyhow::Result<Self> {
        let _file = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .await?;
        crate::log::tracing::init_tracing();
        Ok(Self {
            path: String::from(path.to_str().unwrap_or("")),
        })
    }

    pub fn path_by_id(id: &str) -> PathBuf {
        let home = home_dir().unwrap();

        let log_dir = home.join(".fleet").join("logs");
        log_dir.join(id.to_string() + ".log")
    }

    pub fn rm_logs_by_id(id: &str) -> anyhow::Result<()> {
        let path = Logger::path_by_id(id);

        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    pub async fn fetchn(id: &str, n: usize) -> anyhow::Result<Vec<String>> {
        let path = Logger::path_by_id(id);

        // Vérifier que le fichier existe
        if !tokio::fs::try_exists(&path).await? {
            return Err(anyhow::anyhow!("Failed to find log file"));
        }

        let mut file = File::open(&path).await?;
        let metadata = file.metadata().await?;
        let file_size = metadata.len();

        let mut buffer = vec![0; 8192];
        let mut collected = Vec::new();
        let mut carry = String::new();

        let mut pos = file_size as i64;

        while pos > 0 && collected.len() < n {
            let read_size = buffer.len().min(pos as usize);
            pos -= read_size as i64;

            file.seek(std::io::SeekFrom::Start(pos as u64)).await?;

            file.read_exact(&mut buffer[..read_size]).await?;

            let chunk = String::from_utf8_lossy(&buffer[..read_size]);

            let combined = format!("{chunk}{carry}");
            let mut parts: Vec<&str> = combined.split('\n').collect();

            carry = parts.remove(0).to_string();

            for line in parts.into_iter().rev() {
                if !line.is_empty() {
                    collected.push(line.to_string());
                    if collected.len() >= n {
                        break;
                    }
                }
            }
        }

        if !carry.is_empty() && collected.len() < n {
            collected.push(carry);
        }

        collected.reverse();

        Ok(collected)
    }

    pub fn placeholder() -> Logger {
        Logger {
            path: String::new(),
        }
    }

    pub async fn info(&self, msg: &str) -> anyhow::Result<()> {
        tracing::info!(target: "fleet", log_path = %self.path, message = %msg);
        Ok(())
    }

    pub async fn warning(&self, msg: &str) -> anyhow::Result<()> {
        tracing::warn!(target: "fleet", log_path = %self.path, message = %msg);
        Ok(())
    }

    pub async fn error(&self, msg: &str) -> anyhow::Result<()> {
        tracing::error!(target: "fleet", log_path = %self.path, message = %msg);
        Ok(())
    }

    pub async fn job_start(&self, msg: &str) -> anyhow::Result<()> {
        tracing::event!(
            target: "fleet",
            tracing::Level::INFO,
            log_path = %self.path,
            kind = "JOB START",
            message = %msg
        );
        Ok(())
    }

    pub async fn job_end(&self, msg: &str) -> anyhow::Result<()> {
        tracing::event!(
            target: "fleet",
            tracing::Level::INFO,
            log_path = %self.path,
            kind = "JOB END",
            message = %msg
        );
        Ok(())
    }

    pub async fn clean(&self) -> anyhow::Result<()> {
        let log_path = self.get_path()?;
        remove_file(&log_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to remove log_file {log_path} : {e}"))?;
        Ok(())
    }

    pub fn get_path(&self) -> Result<String, anyhow::Error> {
        if self.path.is_empty() {
            Err(anyhow::anyhow!("Failed to find log path"))
        } else {
            Ok(self.path.clone())
        }
    }
}
