use std::{collections::HashMap, fs::File, process::Stdio, sync::Arc};

use anyhow::Result;
use tempfile::NamedTempFile;
use tokio::{process::Command, sync::Mutex};

pub mod command;
pub mod container;
pub mod metrics;
pub mod runner;

pub enum OutpuStrategy {
    ToFiles {
        stdout: File,
        stderr: File,
    },
    ToPipeOut {
        target: String,
        stdout: File,
        stderr: File,
    },
    ToPipeIn {
        target: String,
        stdout: File,
        stderr: File,
    },
}

pub enum CMDManage {
    Default,
    PipeIn,
    PipeOut,
}
#[derive(Debug)]
pub struct PipeRegistry {
    pub pipes_register: HashMap<String, NamedTempFile>,
}

impl OutpuStrategy {
    async fn configure(
        &self,
        cmd: &mut Command,
        current: Vec<std::string::String>,
        reg: Arc<Mutex<PipeRegistry>>,
    ) -> Result<CMDManage> {
        match self {
            OutpuStrategy::ToFiles { stdout, stderr } => {
                cmd.stdout(Stdio::from(stdout.try_clone()?));
                cmd.stderr(Stdio::from(stderr.try_clone()?));
                Ok(CMDManage::Default)
            }
            OutpuStrategy::ToPipeOut { target, .. } if current == shell_words::split(target)? => {
                // write in output file
                println!("debug: pipe: step '{target}' has been piped !");
                let tmpfile = tempfile::NamedTempFile::new()?;
                cmd.stdout(Stdio::from(tmpfile.reopen()?));
                cmd.stderr(Stdio::from(tmpfile.reopen()?));
                reg.lock()
                    .await
                    .pipes_register
                    .insert(target.into(), tmpfile);
                println!("debug, pipe added to registry : \n{:#?}", reg.lock().await);
                Ok(CMDManage::PipeOut)
            }
            OutpuStrategy::ToPipeOut {
                target: _,
                stdout,
                stderr,
            } => {
                cmd.stdout(Stdio::from(stdout.try_clone()?));
                cmd.stderr(Stdio::from(stderr.try_clone()?));
                Ok(CMDManage::Default)
            }
            OutpuStrategy::ToPipeIn {
                target,
                stdout,
                stderr,
            } if current == shell_words::split(target)? => {
                // read in tmp file
                println!("debug: try to get output as stdin: target: {target}");
                if let Some(pipe_path) = {
                    let registry: tokio::sync::MutexGuard<'_, PipeRegistry> = reg.lock().await;
                    registry
                        .pipes_register
                        .get(target)
                        .map(|tmpfile| tmpfile.path().to_path_buf())
                } {
                    println!("debug: cmd '{:?}' reading from pipe file", current);
                    let file = File::open(pipe_path)?;
                    cmd.stdin(Stdio::from(file));
                } else {
                    cmd.stdin(Stdio::null());
                }
                cmd.stdout(Stdio::from(stdout.try_clone()?));
                cmd.stderr(Stdio::from(stderr.try_clone()?));
                Ok(CMDManage::PipeIn)
            }
            OutpuStrategy::ToPipeIn { stdout, stderr, .. } => {
                cmd.stdout(Stdio::from(stdout.try_clone()?));
                cmd.stderr(Stdio::from(stderr.try_clone()?));
                Ok(CMDManage::Default)
            }
        }
    }
}
