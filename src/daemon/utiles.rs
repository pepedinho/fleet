#![allow(dead_code)]

use anyhow::Result;
pub fn extract_repo_path(remote: &str) -> Result<String> {
    let s = remote.trim();
    if s.is_empty() {
        return Err(anyhow::anyhow!("empty remote"));
    }

    if let Some(scheme_pos) = s.find("://") {
        let after_scheme = &s[scheme_pos + 3..];

        let slash_idx = after_scheme
            .find('/')
            .ok_or_else(|| anyhow::anyhow!("No '/' found after scheme in remote URL"))?;

        let mut path = &after_scheme[slash_idx..];

        if let Some(cut) = path.find(['?', '#']) {
            path = &path[..cut];
        }

        return normalize_git_path(path);
    }
    if let Some(colon_idx) = s.rfind(':') {
        let path = &s[colon_idx + 1..];
        return normalize_git_path(path);
    }
    if s.contains('/')
        && !s.contains(' ')
        && let Some(slash_idx) = s.find('/')
    {
        let path = &s[slash_idx..];
        return normalize_git_path(path);
    }

    Err(anyhow::anyhow!("Failed to extract repo remote path"))
}

fn normalize_git_path(p: &str) -> Result<String> {
    let mut path = p.trim_matches('/').to_string();

    if let Some(cut) = path.find(['?', '#']) {
        path.truncate(cut);
    }
    while path.ends_with('/') {
        path.pop();
    }
    if let Some(stripped) = path.strip_suffix(".git") {
        path = stripped.to_string();
    }
    while path.ends_with('/') {
        path.pop();
    }
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 2 {
        return Err(anyhow::anyhow!("Incorrect remote path: {}", path));
    }

    Ok(segments.join("/"))
}
