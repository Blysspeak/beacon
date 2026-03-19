use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushEvent {
    pub repo: String,
    pub branch: String,
    pub commit: String,
    pub timestamp: u64,
}

fn queue_dir() -> Result<std::path::PathBuf> {
    let dir = config::beacon_dir()?.join("queue");
    fs::create_dir_all(&dir).context("failed to create queue dir")?;
    Ok(dir)
}

pub fn enqueue(repo: &str, branch: &str, commit: &str) -> Result<()> {
    let dir = queue_dir()?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let event = PushEvent {
        repo: repo.to_string(),
        branch: branch.to_string(),
        commit: commit.to_string(),
        timestamp: ts as u64,
    };

    let filename = format!("{ts}.json");
    let path = dir.join(&filename);
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string(&event)?;
    fs::write(&tmp, &json)?;
    fs::rename(&tmp, &path)?;

    Ok(())
}

pub fn dequeue_all() -> Result<Vec<PushEvent>> {
    let dir = queue_dir()?;
    let mut events = Vec::new();

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(events),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            match fs::read_to_string(&path) {
                Ok(data) => {
                    if let Ok(event) = serde_json::from_str::<PushEvent>(&data) {
                        events.push(event);
                    }
                    let _ = fs::remove_file(&path);
                }
                Err(_) => {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    events.sort_by_key(|e| e.timestamp);
    Ok(events)
}
