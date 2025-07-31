use clap::Parser;

use crate::{app::handle_watch, cli::{Cli, Commands}};

mod cli;
mod config;
mod app;
mod git;
mod core;
mod exec;
mod ipc;


fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Watch { branch } => {
            let _ = handle_watch(branch);
        }
        _ => {}
    }
}
