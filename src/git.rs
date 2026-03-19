use anyhow::{Context, Result, bail};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub owner: String,
    pub repo: String,
}

impl RepoInfo {
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

pub fn detect_repo() -> Result<RepoInfo> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .context("failed to run git")?;

    if !output.status.success() {
        bail!("not a git repo or no 'origin' remote");
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    parse_remote_url(&url)
}

pub fn current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .context("failed to get current branch")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn head_commit() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .context("failed to get HEAD commit")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_remote_url(url: &str) -> Result<RepoInfo> {
    // SSH: git@github.com:owner/repo.git
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let path = rest.strip_suffix(".git").unwrap_or(rest);
        return parse_owner_repo(path);
    }

    // HTTPS: https://github.com/owner/repo.git
    if let Some(rest) = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
    {
        let path = rest.strip_suffix(".git").unwrap_or(rest);
        return parse_owner_repo(path);
    }

    bail!("unsupported remote URL format: {url}");
}

fn parse_owner_repo(path: &str) -> Result<RepoInfo> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        bail!("cannot parse owner/repo from: {path}");
    }
    Ok(RepoInfo {
        owner: parts[0].to_string(),
        repo: parts[1].to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ssh_url() {
        let info = parse_remote_url("git@github.com:Blysspeak/beacon.git").unwrap();
        assert_eq!(info.owner, "Blysspeak");
        assert_eq!(info.repo, "beacon");
    }

    #[test]
    fn parse_https_url() {
        let info = parse_remote_url("https://github.com/Blysspeak/beacon.git").unwrap();
        assert_eq!(info.owner, "Blysspeak");
        assert_eq!(info.repo, "beacon");
    }

    #[test]
    fn parse_https_no_git_suffix() {
        let info = parse_remote_url("https://github.com/Blysspeak/beacon").unwrap();
        assert_eq!(info.owner, "Blysspeak");
        assert_eq!(info.repo, "beacon");
    }
}
