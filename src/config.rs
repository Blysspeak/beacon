use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_API_URL: &str = "https://beacon.blysspeak.space";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<RemoteConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll: Option<PollConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollConfig {
    /// Repos to watch (owner/repo format). Empty = auto-discover from history.
    #[serde(default)]
    pub repos: Vec<String>,
    /// Poll interval in seconds (default 60)
    #[serde(default = "default_poll_interval")]
    pub interval_secs: u64,
    /// Auto-discover repos from deploy history
    #[serde(default = "default_true")]
    pub auto_discover: bool,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            repos: vec![],
            interval_secs: 60,
            auto_discover: true,
        }
    }
}

fn default_poll_interval() -> u64 {
    60
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub token: String,
    #[serde(default = "default_api_url")]
    pub api_url: String,
}

fn default_api_url() -> String {
    std::env::var("BEACON_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string())
}

pub fn beacon_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let dir = PathBuf::from(home).join(".beacon");
    fs::create_dir_all(&dir).context("failed to create ~/.beacon")?;
    Ok(dir)
}

fn config_path() -> Result<PathBuf> {
    Ok(beacon_dir()?.join("config.json"))
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let data = fs::read_to_string(&path).context("failed to read config")?;
    serde_json::from_str(&data).context("failed to parse config.json")
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path()?;
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(config)?;
    fs::write(&tmp, &json).context("failed to write config")?;
    fs::rename(&tmp, &path).context("failed to save config")?;
    Ok(())
}
