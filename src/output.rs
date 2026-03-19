use colored::Colorize;
use std::time::Duration;

use crate::providers::{DeployStatus, Status};

pub fn print_status(status: &DeployStatus) {
    let badge = match status.status {
        Status::Success => " SUCCESS ".on_green().white().bold().to_string(),
        Status::Failed => " FAILED ".on_red().white().bold().to_string(),
        Status::InProgress => " IN PROGRESS ".on_yellow().black().bold().to_string(),
        Status::NotFound => " NOT FOUND ".on_bright_black().white().bold().to_string(),
    };

    println!();
    println!("  {badge}");
    println!();
    println!("  {}  {}", "Repo:".dimmed(), status.repo);
    println!("  {}  {}", "Branch:".dimmed(), status.branch);
    println!("  {}  {}", "Commit:".dimmed(), status.commit);

    if let Some(name) = &status.workflow_name {
        println!("  {}  {name}", "Workflow:".dimmed());
    }

    if let Some(url) = &status.url {
        println!("  {}  {}", "URL:".dimmed(), url.underline());
    }

    if !status.failed_jobs.is_empty() {
        println!();
        println!("  {}:", "Failed jobs".red().bold());
        for job in &status.failed_jobs {
            println!("    {} {job}", "×".red());
        }
    }

    if let Some(logs) = &status.logs_tail {
        println!();
        println!("  {}:", "Logs".dimmed());
        for line in logs.lines().take(10) {
            println!("    {line}");
        }
    }

    println!();
}

pub fn print_progress(status: &DeployStatus, elapsed: Duration) {
    let secs = elapsed.as_secs();
    let mins = secs / 60;
    let secs = secs % 60;

    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let frame = spinner[(elapsed.as_millis() / 100) as usize % spinner.len()];

    let time = if mins > 0 {
        format!("{mins}m{secs:02}s")
    } else {
        format!("{secs}s")
    };

    let status_text = match status.status {
        Status::InProgress => "deploying...".yellow().to_string(),
        Status::Success => "success".green().to_string(),
        Status::Failed => "failed".red().to_string(),
        Status::NotFound => "waiting for run...".dimmed().to_string(),
    };

    eprint!(
        "\r  {frame} {} @ {} ({time}) {status_text}  ",
        status.repo.bold(),
        status.branch.cyan(),
    );
}

pub fn print_history(entries: &[DeployStatus]) {
    if entries.is_empty() {
        println!("\n  No deploy history yet.\n");
        return;
    }

    println!();
    for entry in entries {
        let icon = match entry.status {
            Status::Success => "✓".green().to_string(),
            Status::Failed => "✗".red().to_string(),
            Status::InProgress => "◉".yellow().to_string(),
            Status::NotFound => "?".dimmed().to_string(),
        };

        let short_repo = entry.repo.split('/').last().unwrap_or(&entry.repo);
        let commit = if entry.commit.len() > 7 { &entry.commit[..7] } else { &entry.commit };
        let workflow = entry.workflow_name.as_deref().unwrap_or("-");
        let ago = time_ago(&entry.timestamp);

        println!(
            "  {icon}  {:<20} {:<18} {:<7}  {:<20} {}",
            short_repo.bold(),
            workflow.dimmed(),
            commit.dimmed(),
            entry.branch.cyan(),
            ago.dimmed(),
        );
    }
    println!();
}

fn time_ago(ts: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(*ts);
    let secs = diff.num_seconds();

    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

pub fn print_watch_start(repo: &str, branch: &str) {
    println!(
        "\n  {} Watching deploy for {} @ {}\n",
        "⟐".cyan().bold(),
        repo.bold(),
        branch.cyan(),
    );
}
