use clap::Parser;

use crate::{app::handle_watch, cli::Cli};

mod app;
mod cli;
mod config;
mod core;
mod exec;
mod git;
mod ipc;
mod logging;
mod stats;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    handle_watch(&cli).await?;
    Ok(())
}
