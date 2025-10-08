use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{Read, Write},
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
    let content =
        fs::read_to_string(path).with_context(|| format!("Error reading config file {path:?}"))?;

    let mut config: ProjectConfig =
        serde_yaml::from_str(&content).with_context(|| "Error parsing YAML configuration file")?;

    // resolve secret env variable for each job
    for (_job_name, job) in config.pipeline.jobs.iter_mut() {
        let env_map = job.env.as_mut();
        if env_map.is_none() {
            continue;
        }

        for (name, value) in env_map.unwrap().iter_mut() {
            if value.starts_with("$") == false {
                continue;
            }

            let env_key = &value[1..];

            let env_value = if env_key.is_empty() {
                std::env::var(name)
            } else {
                std::env::var(env_key)
            };

            if env_value.is_ok() {
                *value = env_value.unwrap();
                continue;
            }

            if missing_env_validation(env_key)? == true {
                *value = String::from("");
            } else {
                return Err(env_value.err().unwrap().into());
            }
        }
    }

    check_dependency_graph(&config)?;

    // dbg!(&config);

    Ok(config)
}

fn missing_env_validation(env_key: &str) -> Result<bool> {
    eprintln!("Warning: Environment variable \"{env_key}\" is not set");
    eprint!("Continue anyway ? [y/N]");
    std::io::stdout().flush()?;

    let mut buffer = [0_u8; 1];
    std::io::stdin().read_exact(&mut buffer)?;

    let input = buffer[0] as char;

    Ok(input == 'y' || input == 'Y')
}
