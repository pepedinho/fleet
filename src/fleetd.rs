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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = Arc::new(AppState::default());
    init_watch_file().await?;
    WatchContext::init_logs().await?;

    tokio::spawn(supervisor_loop(Arc::clone(&state), 30));

    start_socket_listener(state).await?;

    Ok(())
}
