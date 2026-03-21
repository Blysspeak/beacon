use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};

use crate::config;
use crate::providers::DeployStatus;

fn history_path() -> Result<std::path::PathBuf> {
    Ok(config::beacon_dir()?.join("history.jsonl"))
}

pub fn append(status: &DeployStatus) -> Result<()> {
    let path = history_path()?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .context("failed to open history file")?;

    let json = serde_json::to_string(status)?;
    writeln!(file, "{json}")?;

    // Signal waybar
    crate::mailbox::refresh_waybar();

    Ok(())
}

pub struct HistoryFilter {
    pub limit: usize,
    pub repo: Option<String>,
}

pub fn read(filter: &HistoryFilter) -> Result<Vec<DeployStatus>> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }

    let file = fs::File::open(&path).context("failed to open history")?;
    let reader = BufReader::new(file);

    let mut entries: Vec<DeployStatus> = reader
        .lines()
        .filter_map(|line| line.ok())
        .filter_map(|line| serde_json::from_str::<DeployStatus>(&line).ok())
        .collect();

    // Newest first
    entries.reverse();

    // Filter by repo
    if let Some(repo) = &filter.repo {
        entries.retain(|e| e.repo.contains(repo.as_str()));
    }

    // Limit
    entries.truncate(filter.limit);

    Ok(entries)
}

/// Get unique repo names from history (for auto-discovery in poller)
pub fn unique_repos() -> Result<Vec<String>> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }

    let file = fs::File::open(&path)?;
    let reader = BufReader::new(file);

    let mut seen = std::collections::HashSet::new();
    for line in reader.lines().flatten() {
        if let Ok(entry) = serde_json::from_str::<DeployStatus>(&line) {
            seen.insert(entry.repo);
        }
    }

    Ok(seen.into_iter().collect())
}

/// Read recent deploys grouped by repo (for waybar)
pub fn recent_by_repo(minutes: u64) -> Result<Vec<DeployStatus>> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }

    let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes as i64);
    let file = fs::File::open(&path)?;
    let reader = BufReader::new(file);

    let mut entries: Vec<DeployStatus> = reader
        .lines()
        .filter_map(|line| line.ok())
        .filter_map(|line| serde_json::from_str::<DeployStatus>(&line).ok())
        .filter(|e| e.timestamp > cutoff)
        .collect();

    // Keep only latest per repo
    entries.reverse();
    let mut seen = std::collections::HashSet::new();
    entries.retain(|e| seen.insert(e.repo.clone()));

    Ok(entries)
}
