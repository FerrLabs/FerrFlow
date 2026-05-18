#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct CheckCommit {
    pub hash: String,
    pub message: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct CheckPackage {
    pub name: String,
    pub current_version: String,
    pub next_version: String,
    pub bump_type: String,
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    pub prerelease: bool,
    pub commits: Vec<CheckCommit>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct CheckResult {
    pub packages: Vec<CheckPackage>,
}
