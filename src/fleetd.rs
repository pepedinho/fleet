use std::{fs, os::unix::net::UnixListener, path::Path, sync::Arc};

use clap::Parser;

use crate::{app::handle_watch, cli::{Cli, Commands}, core::{manager::{start_socket_listener, supervisor_loop}, state::{self, AppState}}, ipc::server::handle_request};

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