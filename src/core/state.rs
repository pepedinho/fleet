#![allow(dead_code)]
use std::{collections::HashMap, path::PathBuf};

use crate::{core::watcher::WatchContext, logging::Logger};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};

#[derive(Default)]
pub struct AppState {
    pub watches: RwLock<HashMap<String, WatchContext>>,
}

impl AppState {
    pub async fn load_from_disk() -> Result<Self, anyhow::Error> {
        let registry = load_watches().await?;
        let mut watches: HashMap<String, WatchContext> = HashMap::new();

        for mut ctx in registry.projects {
            let logger = Logger::new(&ctx.log_path()).await?;
            ctx.logger = logger;
            watches.insert(ctx.id.clone(), ctx);
        }

        Ok(Self {
            watches: RwLock::new(watches),
        })
    }

    pub async fn save_to_disk(&self) -> Result<()> {
        let guard = self.watches.read().await;
        let registry = WatchRegistry {
            projects: guard.values().cloned().collect(),
        };
        save_watches(&registry).await
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct WatchRegistry {
    pub projects: Vec<WatchContext>,
}

pub fn get_watch_path() -> PathBuf {
    let path = dirs::data_local_dir()
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));
    path.join("fleetd").join("watches.json")
}

pub async fn init_watch_file() -> Result<()> {
    let path = get_watch_path();
    println!("watch file at: {}", path.to_str().unwrap());
    if !fs::try_exists(&path).await? {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let empty = WatchRegistry::default();
        let json = serde_json::to_string_pretty(&empty)?;
        fs::write(path, json).await?;
    }
    Ok(())
}

pub async fn load_watches() -> Result<WatchRegistry> {
    let path = get_watch_path();
    let data = fs::read_to_string(&path).await?;
    let watches = serde_json::from_str(&data)?;
    Ok(watches)
}

pub async fn save_watches(watches: &WatchRegistry) -> Result<()> {
    let path = get_watch_path();
    let json = serde_json::to_string_pretty(watches)?;
    fs::write(path, json).await?;
    Ok(())
}

pub async fn add_watch(ctx: &WatchContext) -> Result<()> {
    let mut watches = load_watches().await?;

    if let Some(existing) = watches
        .projects
        .iter_mut()
        .find(|p| p.project_dir == ctx.project_dir)
    {
        *existing = ctx.clone();
    } else {
        watches.projects.push(ctx.clone());
    }

    save_watches(&watches).await?;
    Ok(())
}

pub async fn remove_watch_by_id(id: &str) -> Result<()> {
    let mut watches = load_watches().await?;
    watches.projects.retain(|p| p.id != id); // garder seulement les projet avec un id different de celui a supprimé
    save_watches(&watches).await?;
    Ok(())
}

pub async fn get_id_by_name(name: &str) -> Result<Option<String>> {
    let watches = load_watches().await?;
    Ok(watches
        .projects
        .iter()
        .find(|p| p.repo.name == name)
        .map(|p| p.id.clone()))
}

pub async fn get_name_by_id(id: &str) -> Result<Option<String>> {
    let watches = load_watches().await?;
    Ok(watches
        .projects
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.repo.name.clone()))
}
