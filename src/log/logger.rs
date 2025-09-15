use std::{io::SeekFrom, path::PathBuf, sync::Arc};

use anyhow::Ok;
use chrono::Local;
use dirs::home_dir;
use tokio::{
    fs::{File, remove_file},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::Mutex,
};

#[derive(Debug, Clone)]
pub struct Logger {
    pub file: Arc<Mutex<tokio::fs::File>>,
    path: String,
    color_enable: bool,
}

const RESET: &str = "\x1b[0m";
const BG_BLUE: &str = "\x1b[44m"; // info
const BG_ORANGE: &str = "\x1b[48;5;208m"; // warning 
const BG_RED: &str = "\x1b[41m";
const BG_GREEN: &str = "\x1b[42m"; // job start 
const BG_MAGENTA: &str = "\x1b[45m"; // job end 
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

            file.seek(SeekFrom::Start(pos as u64)).await?;

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
            file: Arc::new(Mutex::new(tokio::fs::File::from_std(
                std::fs::File::create("/dev/null").unwrap(),
            ))),
            path: String::new(),
            color_enable: false,
        }
    }

    fn paint_level(&self, level: &str) -> String {
        if !self.color_enable {
            return level.to_string();
        }
        match level {
            "INFO" => format!("{BG_BLUE}{FG_BOLD_WHITE} {level} {RESET}"),
            "WARNING" => format!("{BG_ORANGE}{FG_BOLD_WHITE} {level} {RESET}"),
            "ERROR" => format!("{BG_RED}{FG_BOLD_WHITE} {level} {RESET}"),
            "JOB START" => format!("{BG_GREEN}{FG_BOLD_WHITE} {level} {RESET}"),
            "JOB END" => format!("{BG_MAGENTA}{FG_BOLD_WHITE} {level} {RESET}"),
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

    pub async fn job_start(&self, msg: &str) -> anyhow::Result<()> {
        self.log("JOB START", msg).await
    }

    pub async fn job_end(&self, msg: &str) -> anyhow::Result<()> {
        self.log("JOB END", msg).await
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
