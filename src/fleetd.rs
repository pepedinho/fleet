#![allow(clippy::bool_comparison)]

use std::sync::Arc;

use crate::core::{
    manager::{start_socket_listener, supervisor_loop},
    state::AppState,
    watcher::WatchContext,
};

mod cli;
mod config;
mod core;
mod daemon;
mod exec;
mod git;
mod log;
mod notifications;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    AppState::init_watch_file().await?;
    let state = Arc::new(AppState::load_from_disk().await?);
    WatchContext::init_logs().await?;

    tokio::spawn(supervisor_loop(Arc::clone(&state), 15));

    start_socket_listener(state).await?;

    Ok(())
}
