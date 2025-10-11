use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write,
    path::Path,
};

use anyhow::{Context, Result};

use crate::{
    config::{Job, ProjectConfig, stdin_is_tty},
    log::logger::{LogLevel, Logger},
};

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

    let mut skipped_missing_variables = HashSet::new();

    // resolve secret env variable for each job
    for (job_name, job) in config.pipeline.jobs.iter_mut() {
        let env_map = job.env.as_mut();
        if env_map.is_none() {
            continue;
        }

        for (name, value) in env_map.unwrap().iter_mut() {
            if !value.starts_with("$") {
                continue;
            }

            let env_key = if !&value[1..].is_empty() {
                &value[1..]
            } else {
                name
            };

            let extraction_result = std::env::var(env_key);
            if let Ok(env_value) = extraction_result {
                *value = env_value;
                continue;
            }

            Logger::write(
                &format!(r#""${}" not found for job "{job_name}""#, env_key),
                LogLevel::Warning,
            );

            if skipped_missing_variables.contains(env_key) {
                *value = "".to_string();
                continue;
            }

            if stdin_is_tty() {
                if ask_continue_anyway()? {
                    skipped_missing_variables.insert(env_key.to_string());
                    *value = "".to_string();
                    continue;
                } else {
                    return Err(extraction_result.unwrap_err().into());
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Missing env variable '{}' and no TTY to ask user",
                    env_key
                ));
            }
        }
    }
    // dbg!(&config);
    check_dependency_graph(&config)?;
    Ok(config)
}

fn ask_continue_anyway() -> Result<bool> {
    loop {
        eprint!("Continue anyway ? [y/N] ");
        std::io::stdout().flush()?;

        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer)?;

        let input = buffer.chars().next();

        match input {
            Some('y' | 'Y') => return Ok(true),
            Some('n' | 'N') => return Ok(false),

            _ => {
                eprintln!("Invalid input, please retry.");
                continue;
            }
        }
    }
}
