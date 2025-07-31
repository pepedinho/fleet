use clap::Parser;

use crate::{app::handle_watch, cli::{Cli, Commands}};

mod cli;
mod config;
mod app;
mod git;
mod core;
mod exec;
mod ipc;


#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Watch { branch } => {
            let _ = handle_watch(branch).await;
        }
        _ => {}
    }
    Ok(())
}
