#![allow(clippy::bool_comparison)]

use clap::Parser;

use crate::cli::{Cli, builders::handle_watch};

mod cli;
mod config;
mod core;
mod daemon;
mod exec;
mod git;
mod log;
mod notifications;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    handle_watch(&cli).await?;
    Ok(())
}
