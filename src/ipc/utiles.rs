#![allow(dead_code)]

use anyhow::Result;
pub fn extract_repo_path(remote: &str) -> Result<String> {
    // 1) Nettoyage basique
    let s = remote.trim();
    if s.is_empty() {
        return Err(anyhow::anyhow!("empty remote"));
    }

    // 2) Cas URLs avec schéma : "...://host[:port]/path..."
    if let Some(scheme_pos) = s.find("://") {
        let after_scheme = &s[scheme_pos + 3..];

        // Cherche le premier '/' après host[:port]
        let slash_idx = after_scheme
            .find('/')
            .ok_or_else(|| anyhow::anyhow!("No '/' found after scheme in remote URL"))?;

        // Path inclut le '/' initial
        let mut path = &after_scheme[slash_idx..];

        // Couper query/fragment
        if let Some(cut) = path.find(['?', '#']) {
            path = &path[..cut];
        }

        return normalize_git_path(path);
    }

    // 3) Cas SCP-like : "user@host:path"
    if let Some(colon_idx) = s.rfind(':') {
        let path = &s[colon_idx + 1..];
        return normalize_git_path(path);
    }

    // 4) Cas sans schéma, forme "host/owner/repo(.git)"
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
    // Enlever les '/' en tête et en fin
    let mut path = p.trim_matches('/').to_string();

    // Supprimer query/fragment résiduels
    if let Some(cut) = path.find(['?', '#']) {
        path.truncate(cut);
    }

    // Supprimer '/' de fin
    while path.ends_with('/') {
        path.pop();
    }

    // Supprimer suffixe ".git"
    if let Some(stripped) = path.strip_suffix(".git") {
        path = stripped.to_string();
    }

    // Re-trim de fin au cas où
    while path.ends_with('/') {
        path.pop();
    }

    // Vérifier qu'il y a au moins deux segments (ex: owner/repo)
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 2 {
        return Err(anyhow::anyhow!("Incorrect remote path: {}", path));
    }

    Ok(segments.join("/"))
}
