use std::{collections::HashMap, path::PathBuf};

use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};
use uuid::Uuid;

use crate::core::watcher::WatchContext;

#[derive(Default)]
pub struct AppState {
    pub watches: RwLock<HashMap<Uuid, WatchContext>>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct WatchRegistry {
    pub projects: Vec<WatchContext>,
}

pub fn get_watch_path() -> PathBuf {
    let path = dirs::data_local_dir().unwrap_or_else(|| {
        std::env::current_dir().expect("Failed to get current directory")
    });
    path.join("fleetd").join("watches.json")
}

pub async fn init_watch_file() -> Result<()> {
    let path = get_watch_path();
    println!("create watch file at: {}", path.to_str().unwrap());
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
    watches.projects.push(ctx.clone());
    save_watches(&watches).await?;
    Ok(())
}

pub async fn remove_watch_by_id(id: Uuid) -> Result<()> {
    let mut watches = load_watches().await?;
    watches.projects.retain(|p| p.id != id); // garder seulement les projet avec un id different de celui a supprimé
    save_watches(&watches).await?;
    Ok(())
}

pub async fn get_id_by_name(name: &str) -> Result<Option<Uuid>> {
    let watches = load_watches().await?;
    Ok(watches.projects.iter()
        .find(|p| p.repo.name == name)
        .map(|p| p.id)
    )
}

pub async fn get_name_by_id(id: Uuid) -> Result<Option<String>> {
    let watches = load_watches().await?;
    Ok(watches.projects.iter()
        .find(|p| p.id == id)
        .map(|p| p.repo.name.clone())
    )
}