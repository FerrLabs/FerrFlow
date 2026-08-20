use serde::Serialize;

use super::super::version_source::VersionSource;

#[derive(Serialize, Default)]
pub(super) struct ReleaseJson {
    pub released: Vec<ReleasedPackage>,
    pub skipped: Vec<SkippedPackage>,
    pub git: GitInfo,
    pub dry_run: bool,
}

#[derive(Serialize)]
pub(super) struct ReleasedPackage {
    pub package: String,
    pub previous_version: String,
    pub new_version: String,
    pub bump_type: String,
    pub tag: String,
    pub commit_count: usize,
    pub prerelease: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_source: Option<VersionSource>,
    pub forge_release_url: Option<String>,
    pub forge_release_id: Option<u64>,
}

#[derive(Serialize)]
pub(super) struct SkippedPackage {
    pub package: String,
    pub reason: String,
}

#[derive(Serialize, Default)]
pub(super) struct GitInfo {
    pub commit: String,
    pub tags_pushed: Vec<String>,
    pub branch: String,
}

impl ReleaseJson {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}
