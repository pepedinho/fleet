use std::sync::Arc;

use crate::core::{
    manager::{start_socket_listener, supervisor_loop},
    state::{init_watch_file, AppState}, watcher::WatchContext,
};

mod app;
mod cli;
mod config;
mod core;
mod exec;
mod git;
mod ipc;
mod logging;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_watch_file().await?;
    let state = Arc::new(AppState::load_from_disk().await?); // vide a chaque redemarage (probleme a resoudre)
    WatchContext::init_logs().await?;

    tokio::spawn(supervisor_loop(Arc::clone(&state), 30));

    start_socket_listener(state).await?;

    Ok(())
}
