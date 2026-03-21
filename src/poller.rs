use std::collections::HashMap;

use anyhow::Result;

use crate::config::PollConfig;
use crate::git::RepoInfo;
use crate::history;
use crate::providers::github::{GitHubProvider, RecentRun};
use crate::queue::PushEvent;

/// Tracks last-seen run IDs per repo to avoid re-processing
pub struct GitHubPoller {
    config: PollConfig,
    /// repo_full_name -> last seen run ID
    last_seen: HashMap<String, u64>,
}

impl GitHubPoller {
    pub fn new(config: PollConfig) -> Self {
        Self {
            config,
            last_seen: HashMap::new(),
        }
    }

    /// Resolve the list of repos to poll (configured + auto-discovered)
    fn resolve_repos(&self) -> Vec<String> {
        let mut repos: Vec<String> = self.config.repos.clone();

        if self.config.auto_discover {
            if let Ok(discovered) = history::unique_repos() {
                for repo in discovered {
                    if !repos.contains(&repo) {
                        repos.push(repo);
                    }
                }
            }
        }

        repos
    }

    /// Poll all repos and return new push events for runs we haven't seen yet
    pub async fn poll(&mut self, provider: &GitHubProvider) -> Vec<PushEvent> {
        let repos = self.resolve_repos();
        let mut events = Vec::new();

        for repo_name in &repos {
            match self.poll_repo(provider, repo_name).await {
                Ok(Some(event)) => events.push(event),
                Ok(None) => {}
                Err(e) => {
                    eprintln!("  Poller: error checking {repo_name}: {e:#}");
                }
            }
        }

        events
    }

    async fn poll_repo(
        &mut self,
        provider: &GitHubProvider,
        repo_name: &str,
    ) -> Result<Option<PushEvent>> {
        let repo = parse_repo(repo_name)?;
        let runs = provider.list_recent_runs(&repo, 3).await?;

        if runs.is_empty() {
            return Ok(None);
        }

        let latest = &runs[0];
        let last_seen_id = self.last_seen.get(repo_name).copied().unwrap_or(0);

        if latest.id <= last_seen_id {
            // Already seen this run
            return Ok(None);
        }

        // Check if this run is new AND active (in_progress or just completed)
        // Skip runs that are already terminal and were never tracked
        // (they're old completed runs we're seeing for the first time)
        let is_active = is_run_active(latest);
        let is_first_poll = last_seen_id == 0;

        self.last_seen.insert(repo_name.to_string(), latest.id);

        if is_first_poll && !is_active {
            // First time seeing this repo — don't enqueue old completed runs
            return Ok(None);
        }

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        Ok(Some(PushEvent {
            repo: repo_name.to_string(),
            branch: latest.head_branch.clone(),
            commit: latest.head_sha.clone(),
            timestamp: ts,
        }))
    }
}

fn is_run_active(run: &RecentRun) -> bool {
    matches!(run.status.as_str(), "queued" | "in_progress" | "waiting" | "pending" | "requested")
}

fn parse_repo(name: &str) -> Result<RepoInfo> {
    let parts: Vec<&str> = name.splitn(2, '/').collect();
    if parts.len() != 2 {
        anyhow::bail!("invalid repo format: {name}");
    }
    Ok(RepoInfo {
        owner: parts[0].to_string(),
        repo: parts[1].to_string(),
    })
}
