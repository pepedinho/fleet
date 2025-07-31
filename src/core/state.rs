use std::{collections::HashMap, sync::{Arc}};

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::core::watcher::WatchContext;

#[derive(Default)]
pub struct AppState {
    pub watches: RwLock<HashMap<Uuid, WatchContext>>,
}

pub type SharedState = Arc<AppState>;

// impl AppState {
//     pub fn default() -> Self {
//         Self {
//             watches: RwLock::new(HashMap::new()),
//         }
//     }
// }