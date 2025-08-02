use std::{collections::HashMap};

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::core::watcher::WatchContext;

#[derive(Default)]
pub struct AppState {
    pub watches: RwLock<HashMap<Uuid, WatchContext>>,
}