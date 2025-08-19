use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Ok, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct UpdateCommand {
    pub cmd: String,
    #[serde(default)]
    pub blocking: bool,
    #[serde(default)]
    pub container: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct CommandSection {
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    pub steps: Vec<UpdateCommand>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ProjectConfig {
    pub update: CommandSection,

    #[serde(default)]
    pub on_conflict: CommandSection,

    #[serde(default)]
    pub post_update: Vec<String>,

    #[serde(default)]
    pub branch: Option<String>,

    #[serde(default)]
    pub timeout: Option<u64>,
}

pub fn load_config(path: &Path) -> Result<ProjectConfig> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Error reading config file {path:?}"))?;

    let mut config: ProjectConfig =
        serde_yaml::from_str(&content).with_context(|| "Error parsing YAML configuration file")?;

    if let Some(ref mut env_map) = config.update.env {
        for (key, value) in env_map.iter_mut() {
            if value == "$" {
                *value = std::env::var(key).unwrap_or_else(|_| String::from(""));
            }
        }
    }

    Ok(config)
}
