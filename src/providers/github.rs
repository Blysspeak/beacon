use std::time::Duration;

use anyhow::{Result, bail};
use chrono::Utc;
use reqwest::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;

use crate::git::RepoInfo;
use crate::providers::{DeployStatus, PROVIDER_GITHUB, Provider, Status};

const API_TIMEOUT: Duration = Duration::from_secs(15);
const USER_AGENT_VALUE: &str = "beacon-deploy-watch/0.1";

pub struct GitHubProvider {
    client: Client,
}

impl GitHubProvider {
    pub fn new(token: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(API_TIMEOUT)
            .default_headers({
                let mut h = reqwest::header::HeaderMap::new();
                h.insert(AUTHORIZATION, format!("Bearer {token}").parse()?);
                h.insert(USER_AGENT, USER_AGENT_VALUE.parse()?);
                h.insert(ACCEPT, "application/vnd.github+json".parse()?);
                h
            })
            .build()?;

        Ok(Self { client })
    }

    async fn fetch_failed_jobs(&self, repo: &RepoInfo, run_id: u64) -> Result<Vec<String>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/runs/{run_id}/jobs",
            repo.owner, repo.repo,
        );

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Ok(vec![]);
        }

        let data: GhJobsResponse = resp.json().await?;

        Ok(data
            .jobs
            .iter()
            .filter(|j| j.conclusion.as_deref() != Some("success"))
            .filter(|j| j.conclusion.is_some())
            .map(|j| {
                format!(
                    "{} ({})",
                    j.name,
                    j.conclusion.as_deref().unwrap_or("unknown")
                )
            })
            .collect())
    }
}

impl Provider for GitHubProvider {
    async fn get_run_status(
        &self,
        repo: &RepoInfo,
        branch: &str,
        commit: &str,
    ) -> Result<DeployStatus> {
        // URL-encode branch name (handles feature/foo, spaces, etc.)
        let branch_encoded = urlencoded(branch);
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/runs?branch={branch_encoded}&head_sha={commit}&per_page=5",
            repo.owner, repo.repo,
        );

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            match code.as_u16() {
                401 => bail!("GitHub auth failed. Check your token (GITHUB_TOKEN or `gh auth login`)"),
                403 => bail!("GitHub API rate limit or forbidden. Body: {body}"),
                404 => bail!("Repository {}/{} not found or no access", repo.owner, repo.repo),
                _ => bail!("GitHub API error ({code}): {body}"),
            }
        }

        let data: GhRunsResponse = resp.json().await?;

        if data.workflow_runs.is_empty() {
            return Ok(DeployStatus::not_found(repo, branch, commit));
        }

        let run = &data.workflow_runs[0];

        let status = match (run.status.as_str(), run.conclusion.as_deref()) {
            ("completed", Some("success")) => Status::Success,
            ("completed", _) => Status::Failed,
            _ => Status::InProgress,
        };

        let failed_jobs = if status == Status::Failed {
            self.fetch_failed_jobs(repo, run.id).await.unwrap_or_default()
        } else {
            vec![]
        };

        Ok(DeployStatus {
            status,
            provider: PROVIDER_GITHUB.to_string(),
            repo: repo.full_name(),
            branch: branch.to_string(),
            commit: commit.to_string(),
            timestamp: Utc::now(),
            url: Some(run.html_url.clone()),
            workflow_name: Some(run.name.clone()),
            failed_jobs,
            logs_tail: None,
        })
    }
}

pub fn resolve_token() -> Result<String> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            return Ok(token);
        }
    }

    let output = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !token.is_empty() {
                return Ok(token);
            }
        }
    }

    bail!("GitHub token not found. Set GITHUB_TOKEN env var or run `gh auth login`")
}

/// Percent-encode a string for use in URL query parameters.
fn urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

/// A recent workflow run (for polling mode — no commit filter)
#[derive(Debug, Clone)]
pub struct RecentRun {
    pub id: u64,
    pub name: String,
    pub head_sha: String,
    pub head_branch: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub html_url: String,
}

impl GitHubProvider {
    /// List recent workflow runs for a repo (no commit SHA filter).
    /// Used by the poller to discover new runs.
    pub async fn list_recent_runs(
        &self,
        repo: &RepoInfo,
        per_page: u8,
    ) -> Result<Vec<RecentRun>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/runs?per_page={per_page}",
            repo.owner, repo.repo,
        );

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("GitHub API error ({code}) for {}/{}: {body}", repo.owner, repo.repo);
        }

        let data: GhRunsResponseFull = resp.json().await?;

        Ok(data
            .workflow_runs
            .into_iter()
            .map(|r| RecentRun {
                id: r.id,
                name: r.name,
                head_sha: r.head_sha,
                head_branch: r.head_branch,
                status: r.status,
                conclusion: r.conclusion,
                html_url: r.html_url,
            })
            .collect())
    }
}

// --- GitHub API response types ---

#[derive(Deserialize)]
struct GhRunsResponse {
    workflow_runs: Vec<GhRun>,
}

#[derive(Deserialize)]
struct GhRunsResponseFull {
    workflow_runs: Vec<GhRunFull>,
}

#[derive(Deserialize)]
struct GhRun {
    id: u64,
    name: String,
    status: String,
    conclusion: Option<String>,
    html_url: String,
}

#[derive(Deserialize)]
struct GhRunFull {
    id: u64,
    name: String,
    head_sha: String,
    head_branch: String,
    status: String,
    conclusion: Option<String>,
    html_url: String,
}

#[derive(Deserialize)]
struct GhJobsResponse {
    jobs: Vec<GhJob>,
}

#[derive(Deserialize)]
struct GhJob {
    name: String,
    conclusion: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencode_simple() {
        assert_eq!(urlencoded("main"), "main");
    }

    #[test]
    fn urlencode_slash() {
        assert_eq!(urlencoded("feature/test"), "feature%2Ftest");
    }

    #[test]
    fn urlencode_spaces() {
        assert_eq!(urlencoded("my branch"), "my%20branch");
    }
}
