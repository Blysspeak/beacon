use anyhow::Result;
use std::time::{Duration, Instant};

use crate::git::RepoInfo;
use crate::providers::{DeployStatus, Provider, Status};
use crate::{mailbox, output};

const PHASE1_INTERVAL: Duration = Duration::from_secs(5);
const PHASE1_DURATION: Duration = Duration::from_secs(120);
const PHASE2_INTERVAL: Duration = Duration::from_secs(15);
const MAX_DURATION: Duration = Duration::from_secs(30 * 60);
const NOT_FOUND_TIMEOUT: Duration = Duration::from_secs(120);

/// Foreground watch loop with terminal output (for `beacon watch`)
pub async fn watch(
    provider: impl Provider,
    repo: &RepoInfo,
    branch: &str,
    commit: &str,
) -> Result<DeployStatus> {
    output::print_watch_start(&repo.full_name(), branch);

    let start = Instant::now();
    let mut last_status = DeployStatus::not_found(repo, branch, commit);

    loop {
        let elapsed = start.elapsed();

        if elapsed > MAX_DURATION {
            eprintln!("\n\n  Timeout after 30 minutes. Use `beacon status` to check later.\n");
            return Ok(last_status);
        }

        match provider.get_run_status(repo, branch, commit).await {
            Ok(status) => {
                if status.status != Status::NotFound {
                    mailbox::write(&status)?;
                }
                output::print_progress(&status, elapsed);

                if status.is_terminal() {
                    let _ = crate::history::append(&status);
                    eprint!("\r{}\r", " ".repeat(80));
                    output::print_status(&status);
                    return Ok(status);
                }

                if status.status == Status::NotFound && start.elapsed() > NOT_FOUND_TIMEOUT {
                    eprint!("\r{}\r", " ".repeat(80));
                    eprintln!("  No CI workflow found. Repo may not have GitHub Actions.");
                    return Ok(last_status);
                }

                last_status = status;
            }
            Err(e) => {
                eprintln!("\r  Warning: poll failed: {e:#}");
            }
        }

        let interval = if start.elapsed() < PHASE1_DURATION {
            PHASE1_INTERVAL
        } else {
            PHASE2_INTERVAL
        };
        tokio::time::sleep(interval).await;
    }
}
