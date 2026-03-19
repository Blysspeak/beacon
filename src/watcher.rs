use anyhow::Result;
use std::time::{Duration, Instant};

use crate::git::RepoInfo;
use crate::providers::{DeployStatus, Provider};
use crate::{mailbox, output};

const PHASE1_INTERVAL: Duration = Duration::from_secs(5);
const PHASE1_DURATION: Duration = Duration::from_secs(120);
const PHASE2_INTERVAL: Duration = Duration::from_secs(15);
const MAX_DURATION: Duration = Duration::from_secs(30 * 60);
const NOT_FOUND_TIMEOUT: Duration = Duration::from_secs(120);

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
                // Don't overwrite a terminal result (success/failed) with not_found
                // This prevents a push to repo-without-CI from erasing the last real result
                if status.status != crate::providers::Status::NotFound {
                    mailbox::write(&status)?;
                }
                output::print_progress(&status, elapsed);

                if status.is_terminal() {
                    eprint!("\r{}\r", " ".repeat(80));
                    output::print_status(&status);
                    return Ok(status);
                }

                // Give up if no workflow run found after 2 minutes (repo has no CI)
                if status.status == crate::providers::Status::NotFound
                    && start.elapsed() > NOT_FOUND_TIMEOUT
                {
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

pub fn daemonize() -> Result<()> {
    let exe = std::env::current_exe()?;

    // Re-run ourselves without --daemon to get foreground watch
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    args.retain(|a| a != "--daemon");

    let child = std::process::Command::new(exe)
        .args(&args)
        .stdout(std::process::Stdio::null())
        .stderr(std::fs::File::create(
            crate::config::beacon_dir()?.join("daemon.log"),
        )?)
        .stdin(std::process::Stdio::null())
        .spawn()?;

    let pid = child.id();
    let pid_path = crate::config::beacon_dir()?.join("watcher.pid");
    std::fs::write(&pid_path, pid.to_string())?;

    println!("  Background watcher started (PID {pid})");
    println!("  Logs: ~/.beacon/daemon.log");

    Ok(())
}
