pub mod github;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::git::RepoInfo;

pub const PROVIDER_GITHUB: &str = "github_actions";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployStatus {
    pub status: Status,
    pub provider: String,
    pub repo: String,
    pub branch: String,
    pub commit: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub failed_jobs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs_tail: Option<String>,
}

impl DeployStatus {
    pub fn not_found(repo: &RepoInfo, branch: &str, commit: &str) -> Self {
        Self {
            status: Status::NotFound,
            provider: PROVIDER_GITHUB.to_string(),
            repo: repo.full_name(),
            branch: branch.to_string(),
            commit: commit.to_string(),
            timestamp: Utc::now(),
            url: None,
            workflow_name: None,
            failed_jobs: vec![],
            logs_tail: None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.status, Status::Success | Status::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    InProgress,
    Success,
    Failed,
    NotFound,
}

pub trait Provider {
    async fn get_run_status(
        &self,
        repo: &RepoInfo,
        branch: &str,
        commit: &str,
    ) -> Result<DeployStatus>;
}
