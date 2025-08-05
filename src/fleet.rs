use clap::Parser;

use crate::{
    app::handle_watch,
    cli::Cli,
};

mod app;
mod cli;
mod config;
mod core;
mod exec;
mod git;
mod ipc;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    handle_watch(&cli).await?;
    // match cli.command {
    //     Commands::Watch { branch } => {
    //         let _ = handle_watch(branch).await;
    //     }
    //     Commands::Ps { all } => {
    //         let _ = handle_watch(None).await;
    //     }
    //     _ => {}
    // }
    Ok(())
}
