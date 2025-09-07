use std::{fs::File, process::Stdio};

use anyhow::Result;
use tokio::process::Command;

pub mod command;
pub mod container;
pub mod metrics;
pub mod runner;

pub enum OutpuStrategy {
    ToFiles {
        stdout: File,
        stderr: File,
    },
    ToPipe {
        stdout: File,
        stderr: File,
        target: String,
    },
}

impl OutpuStrategy {
    fn configure(&self, cmd: &mut Command, current: Vec<std::string::String>) -> Result<()> {
        match self {
            OutpuStrategy::ToFiles { stdout, stderr } => {
                cmd.stdout(Stdio::from(stdout.try_clone()?));
                cmd.stderr(Stdio::from(stderr.try_clone()?));
            }
            OutpuStrategy::ToPipe {
                stdout: _,
                stderr: _,
                target,
            } if current == shell_words::split(&target)? => {
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());
                println!("debug: pipe: step '{target}' has been piped !");
            }
            OutpuStrategy::ToPipe {
                stdout,
                stderr,
                target: _,
            } => {
                cmd.stdout(Stdio::from(stdout.try_clone()?));
                cmd.stderr(Stdio::from(stderr.try_clone()?));
            }
        }
        Ok(())
    }
}
