use std::time::Duration;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::config::RemoteConfig;
use crate::providers::DeployStatus;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Serialize)]
struct NotifyPayload {
    token: String,
    deploy: DeployStatus,
}

#[derive(Serialize)]
struct TestPayload {
    token: String,
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?)
}

pub async fn send_deploy_status(cfg: &RemoteConfig, status: &DeployStatus) -> Result<()> {
    let url = format!("{}/notify", cfg.api_url);
    let payload = NotifyPayload {
        token: cfg.token.clone(),
        deploy: status.clone(),
    };

    let resp = client()?.post(&url).json(&payload).send().await?;

    if !resp.status().is_success() {
        let code = resp.status();
        let err = resp.text().await.unwrap_or_default();
        bail!("Beacon API error ({code}): {err}");
    }

    Ok(())
}

pub async fn send_test(cfg: &RemoteConfig) -> Result<()> {
    let url = format!("{}/test", cfg.api_url);
    let payload = TestPayload {
        token: cfg.token.clone(),
    };

    let resp = client()?.post(&url).json(&payload).send().await?;

    if !resp.status().is_success() {
        let code = resp.status();
        let err = resp.text().await.unwrap_or_default();
        bail!("Beacon API error ({code}): {err}");
    }

    Ok(())
}
