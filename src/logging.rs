#![allow(dead_code)]
use std::sync::Arc;

use anyhow::Ok;
use chrono::Local;
use tokio::{io::AsyncWriteExt, sync::Mutex};

#[derive(Clone)]
pub struct Logger {
    file: Arc<Mutex<tokio::fs::File>>,
}

impl Logger {
    pub async fn new(path: &std::path::Path) -> anyhow::Result<Self> {
        let file = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .await?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }

    pub async fn log(&self, msg: &str) -> anyhow::Result<()> {
        let mut f = self.file.lock().await;
        let now = Local::now();
        let full_msg = format!("[{}] {}\n", now.format("%Y-%m-%d %H:%M:%S"), msg);
        f.write_all(full_msg.as_bytes()).await?;
        f.flush().await?;
        Ok(())
    }

    pub async fn info(&self, msg: &str) -> anyhow::Result<()> {
        self.log(&format!("INFO: {msg}")).await
    }

    pub async fn warning(&self, msg: &str) -> anyhow::Result<()> {
        self.log(&format!("WARNING: {msg}")).await
    }

    pub async fn error(&self, msg: &str) -> anyhow::Result<()> {
        self.log(&format!("ERROR: {msg}")).await
    }
}
