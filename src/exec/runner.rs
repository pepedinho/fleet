use std::process::Command;

use crate::config::parser::ProjectConfig;


pub fn run_update(config: &ProjectConfig) -> Result<(), anyhow::Error> {
    println!("▶ Update project...");

    for (i, command_line) in config.update.iter().enumerate() {
        println!("➡️  [Cmd {}] {}", i + 1, command_line);
        let mut parts: Vec<String> = shell_words::split(&command_line)
            .map_err(|e| anyhow::anyhow!("Error during command parsing '{}' : {}", command_line, e))?;

        if parts.is_empty() {
            continue;
        }

        let program = parts.remove(0);
        let status = Command::new(program)
            .args(parts)
            .current_dir(".")
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "Command {} failed, error code : {}",
                command_line,
                status.code().unwrap_or(-1)
            ));
        }
    }

    println!("✅ Update done !");
    Ok(())
}