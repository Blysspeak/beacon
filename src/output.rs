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

pub fn print_watch_start(repo: &str, branch: &str) {
    println!(
        "\n  {} Watching deploy for {} @ {}\n",
        "⟐".cyan().bold(),
        repo.bold(),
        branch.cyan(),
    );
}
