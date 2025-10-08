use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use anyhow::{Context, Ok, Result};

use crate::config::{Job, ProjectConfig};

pub fn check_dependency_graph(config: &ProjectConfig) -> Result<()> {
    let pipeline = &config.pipeline;

    for (name, job) in pipeline.jobs.iter() {
        if job.needs.contains(name) {
            return Err(anyhow::anyhow!("Job '{}' cannot depend on itself", name));
        }
        for dep in &job.needs {
            if !pipeline.jobs.contains_key(dep) {
                return Err(anyhow::anyhow!(
                    "Job '{}' depends on unknown job '{}'",
                    name,
                    dep
                ));
            }
        }
    }

    fn visit(
        name: &str,
        pipeline: &HashMap<String, Job>,
        temp: &mut HashSet<String>,
        perm: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> anyhow::Result<()> {
        if perm.contains(name) {
            return Ok(());
        }
        if !temp.insert(name.to_string()) {
            let cycle_start_index: usize = path.iter().position(|n| n == name).unwrap_or(0);
            let cycle_path: Vec<_> = path[cycle_start_index..]
                .iter()
                .chain(std::iter::once(&name.to_string()))
                .cloned()
                .collect();
            return Err(anyhow::anyhow!(
                "Cycle detected: [{}]",
                cycle_path.join(" -> ")
            ));
        }

        path.push(name.to_string());
        if let Some(job) = pipeline.get(name) {
            for dep in &job.needs {
                visit(dep, pipeline, temp, perm, path)?;
            }
        }
        temp.remove(name);
        perm.insert(name.to_string());
        path.pop();
        Ok(())
    }
    let mut temp = HashSet::new();
    let mut perm = HashSet::new();
    let mut path = Vec::new();
    for name in pipeline.jobs.keys() {
        visit(name, &pipeline.jobs, &mut temp, &mut perm, &mut path)?;
    }
    Ok(())
}

pub fn load_config(path: &Path) -> Result<ProjectConfig> {
    let content: String =
        fs::read_to_string(path).with_context(|| format!("Error reading config file {path:?}"))?;

    let mut config: ProjectConfig =
        serde_yaml::from_str(&content).with_context(|| "Error parsing YAML configuration file")?;

    // resolve secret env variable for each job
    for (_name, job) in config.pipeline.jobs.iter_mut() {
        if let Some(env_map) = job.env.as_mut() {
            for (key, value) in env_map.iter_mut() {
                if value == "$" {
                    *value = std::env::var(key).unwrap_or_default(); // if not found default value is ""
                }
            }
        }
    }

    check_dependency_graph(&config)?;

    // dbg!(&config);

    Ok(config)
}
