use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    Watch {
        #[arg(short = 'b', long, default_value = None)]
        branch: Option<String>,
    },

    Ps {
        #[arg(short = 'a', long)]
        all: bool,
    },

    Stop {
        id: String,
    },

    Up {
        id: String,
    },

    Rm {
        id: String
    },
}