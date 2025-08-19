#![allow(dead_code)]
use std::sync::Arc;

use anyhow::Ok;
use chrono::Local;
use tokio::{io::AsyncWriteExt, sync::Mutex};

#[derive(Clone)]
pub struct Logger {
    pub file: Arc<Mutex<tokio::fs::File>>,
    path: String,
    color_enable: bool,
}

const RESET: &str = "\x1b[0m";
const BG_BLUE: &str = "\x1b[44m"; // info
const BG_ORANGE: &str = "\x1b[48;5;208m"; // warning (orange vrai en 256 couleurs)
const BG_RED: &str = "\x1b[41m";
const FG_BOLD_WHITE: &str = "\x1b[97;1m";

impl Logger {
    pub async fn new(path: &std::path::Path) -> anyhow::Result<Self> {
        let file = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .await?;
        let no_color = std::env::var("FLEET_NO_COLOR").ok().as_deref() == Some("1");
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            path: String::from(path.to_str().unwrap_or("")),
            color_enable: !no_color,
        })
    }

    fn paint_level(&self, level: &str) -> String {
        if !self.color_enable {
            return level.to_string();
        }
        match level {
            "INFO" => format!("{BG_BLUE}{FG_BOLD_WHITE} {level} {RESET}"),
            "WARNING" => format!("{BG_ORANGE}{FG_BOLD_WHITE} {level} {RESET}"),
            "ERROR" => format!("{BG_RED}{FG_BOLD_WHITE} {level} {RESET}"),
            _ => level.to_string(),
        }
    }

    pub async fn log(&self, level: &str, msg: &str) -> anyhow::Result<()> {
        let mut f = self.file.lock().await;
        let now = Local::now();
        let line = format!(
            "[{}] {}: {}\n",
            now.format("%Y-%m-%d %H:%M:%S"),
            self.paint_level(level),
            msg
        );
        f.write_all(line.as_bytes()).await?;
        f.flush().await?;
        Ok(())
    }

    pub async fn info(&self, msg: &str) -> anyhow::Result<()> {
        self.log("INFO", msg).await
    }

    pub async fn warning(&self, msg: &str) -> anyhow::Result<()> {
        self.log("WARNING", msg).await
    }

    pub async fn error(&self, msg: &str) -> anyhow::Result<()> {
        self.log("ERROR", msg).await
    }

    pub fn get_path(&self) -> Result<String, anyhow::Error> {
        if self.path.is_empty() {
            Err(anyhow::anyhow!("Failed to find log path"))
        } else {
            Ok(self.path.clone())
        }
    }
}
