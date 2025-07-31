use std::sync::Arc;


use crate::core::{manager::{start_socket_listener, supervisor_loop}, state::AppState};

mod cli;
mod config;
mod app;
mod git;
mod exec;
mod core;
mod ipc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = Arc::new(AppState::default());

    tokio::spawn(supervisor_loop(Arc::clone(&state), 30));

    start_socket_listener(state).await?;
    
    Ok(())
}