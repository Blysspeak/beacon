use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use tokio::task::JoinHandle;

use crate::git::RepoInfo;
use crate::providers::github::GitHubProvider;
use crate::providers::{Provider, Status};
use crate::{config, history, mailbox, poller, queue, telegram};

const QUEUE_INTERVAL: Duration = Duration::from_secs(2);

struct TrackedJob {
    handle: JoinHandle<()>,
    _repo: String,
    _commit: String,
}

pub async fn run() -> Result<()> {
    let cfg = config::load()?;

    // Resolve GitHub token once at startup
    let token = crate::providers::github::resolve_token()?;
    let remote_cfg = cfg.remote;

    // Set up GitHub poller if configured (or default: auto-discover from history)
    let poll_config = cfg.poll.unwrap_or_default();
    let poll_interval = Duration::from_secs(poll_config.interval_secs);
    let mut gh_poller = poller::GitHubPoller::new(poll_config);
    let poll_provider = GitHubProvider::new(&token)?;
    let mut last_poll = std::time::Instant::now() - poll_interval; // poll immediately on start

    eprintln!("Beacon daemon started. Watching queue + polling GitHub ({}s interval)", poll_interval.as_secs());

    let mut active: HashMap<String, TrackedJob> = HashMap::new();

    loop {
        // 1. Check queue for push events (from hooks)
        let mut events = queue::dequeue_all().unwrap_or_default();

        // 2. Check GitHub poller on interval
        if last_poll.elapsed() >= poll_interval {
            let polled = gh_poller.poll(&poll_provider).await;
            if !polled.is_empty() {
                eprintln!("  Poller discovered {} new run(s)", polled.len());
            }
            // Don't enqueue events for repos already being tracked
            for event in polled {
                if !active.contains_key(&event.repo) || active.get(&event.repo).map_or(false, |j| j.handle.is_finished()) {
                    events.push(event);
                }
            }
            last_poll = std::time::Instant::now();
        }

        // 3. Start tracking new events
        for event in events {
            let repo_key = event.repo.clone();
            let commit_short = if event.commit.len() > 7 { &event.commit[..7] } else { &event.commit };

            // Cancel existing tracker for this repo
            if let Some(old) = active.remove(&repo_key) {
                old.handle.abort();
                eprintln!("  Replaced tracker for {}", repo_key);
            }

            let token = token.clone();
            let remote = remote_cfg.clone();
            let repo_key_clone = repo_key.clone();
            let commit_for_log = event.commit.clone();

            eprintln!("  Tracking {} @ {} ({commit_short})", event.repo, event.branch);

            let event_clone = event.clone();
            let handle = tokio::spawn(async move {
                if let Err(e) = track_deploy(event_clone, &token, remote.as_ref()).await {
                    eprintln!("  Error tracking {}: {e:#}", repo_key_clone);
                }
            });

            active.insert(repo_key, TrackedJob {
                handle,
                _repo: event.repo.clone(),
                _commit: commit_for_log,
            });
        }

        // Clean up completed tasks
        active.retain(|_key, job| !job.handle.is_finished());

        tokio::time::sleep(QUEUE_INTERVAL).await;
    }
}

async fn track_deploy(
    event: queue::PushEvent,
    token: &str,
    remote: Option<&config::RemoteConfig>,
) -> Result<()> {
    let parts: Vec<&str> = event.repo.splitn(2, '/').collect();
    if parts.len() != 2 {
        anyhow::bail!("invalid repo format: {}", event.repo);
    }

    let repo = RepoInfo {
        owner: parts[0].to_string(),
        repo: parts[1].to_string(),
    };

    let provider = GitHubProvider::new(token)?;

    // Use the existing watcher logic but without terminal output
    let phase1_interval = Duration::from_secs(5);
    let phase1_duration = Duration::from_secs(120);
    let phase2_interval = Duration::from_secs(15);
    let max_duration = Duration::from_secs(30 * 60);
    let not_found_timeout = Duration::from_secs(120);

    let start = std::time::Instant::now();

    loop {
        let elapsed = start.elapsed();

        if elapsed > max_duration {
            eprintln!("  Timeout for {} after 30m", event.repo);
            return Ok(());
        }

        match provider
            .get_run_status(&repo, &event.branch, &event.commit)
            .await
        {
            Ok(status) => {
                // Don't write not_found to mailbox
                if status.status != Status::NotFound {
                    mailbox::write(&status)?;
                }

                if status.is_terminal() {
                    // Write to history
                    if let Err(e) = history::append(&status) {
                        eprintln!("  History write failed: {e:#}");
                    }

                    eprintln!(
                        "  {} {} — {:?}",
                        if status.status == Status::Success { "✓" } else { "✗" },
                        event.repo,
                        status.status,
                    );

                    // Send Telegram notification
                    if let Some(remote) = remote {
                        if let Err(e) = telegram::send_deploy_status(remote, &status).await {
                            eprintln!("  Telegram failed for {}: {e:#}", event.repo);
                        }
                    }

                    return Ok(());
                }

                // Give up if no run found after 2 minutes
                if status.status == Status::NotFound && elapsed > not_found_timeout {
                    eprintln!("  No CI found for {} — giving up", event.repo);
                    return Ok(());
                }
            }
            Err(e) => {
                eprintln!("  Poll error for {}: {e:#}", event.repo);
            }
        }

        let interval = if elapsed < phase1_duration {
            phase1_interval
        } else {
            phase2_interval
        };
        tokio::time::sleep(interval).await;
    }
}
