use anyhow::{Context, Result};
use std::fs;

use crate::config;
use crate::providers::DeployStatus;

fn status_path() -> Result<std::path::PathBuf> {
    Ok(config::beacon_dir()?.join("last_deploy.json"))
}

pub fn write(status: &DeployStatus) -> Result<()> {
    let path = status_path()?;
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(status)?;
    fs::write(&tmp, &json).context("failed to write deploy status")?;
    fs::rename(&tmp, &path).context("failed to save deploy status")?;
    Ok(())
}

pub fn read_last() -> Result<Option<DeployStatus>> {
    let path = status_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path).context("failed to read deploy status")?;
    let status: DeployStatus =
        serde_json::from_str(&data).context("failed to parse last_deploy.json")?;
    Ok(Some(status))
}
