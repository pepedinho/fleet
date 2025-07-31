use clap::Parser;

use crate::cli::{Cli, Commands};

mod cli;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Watch { branch } => {
            println!("watch repo on branch => {:?}", branch);
        }
        _ => {}
    }
}
