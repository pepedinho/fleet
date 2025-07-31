
use std::{fs, path::Path};

use anyhow::{Context, Ok, Result};
use serde::Deserialize;
#[derive(Debug, Deserialize, Default, Clone)]
pub struct ProjectConfig {
    pub update: Vec<String>,

    #[serde(default)]
    pub on_conflict: Vec<String>,

    #[serde(default)]
    pub post_update: Vec<String>,

    #[serde(default)]
    pub branch: Option<String>
}

pub fn load_config(path: &Path) -> Result<ProjectConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Error reading config file {:?}", path))?;

    let config: ProjectConfig = serde_yaml::from_str(&content)
        .with_context(|| "Error parsing YAML configuration file")?;
 
    Ok(config)
}