#![allow(dead_code)]

pub mod builders;
pub mod client;
pub mod stats;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    Run {
        id: String,
    },
    Watch,
    Ps {
        #[arg(short = 'a', long)]
        all: bool,
    },

    Stats,

    Stop {
        id: String,
    },

    Up {
        id: String,
    },

    Rm {
        id: String,
    },

    Logs {
        #[arg(short = 'f', long)]
        follow: bool,
        id_or_name: Option<String>,
    },
}
