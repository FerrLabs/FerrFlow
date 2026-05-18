use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ForgeKind {
    #[default]
    Auto,
    #[serde(alias = "GitHub")]
    Github,
    #[serde(alias = "GitLab")]
    Gitlab,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct HooksConfig {
    #[serde(alias = "preBump")]
    pub pre_bump: Option<String>,
    #[serde(alias = "postBump")]
    pub post_bump: Option<String>,
    #[serde(alias = "preCommit")]
    pub pre_commit: Option<String>,
    #[serde(alias = "prePublish")]
    pub pre_publish: Option<String>,
    #[serde(alias = "postPublish")]
    pub post_publish: Option<String>,
    #[serde(default, alias = "onFailure")]
    pub on_failure: Option<OnFailure>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum OnFailure {
    #[default]
    Abort,
    Continue,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum OrphanedTagStrategy {
    #[default]
    Warn,
    TreeHash,
    Message,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BranchChannelConfig {
    pub name: String,
    #[serde(default)]
    pub channel: ChannelValue,
    #[serde(default, alias = "prereleaseIdentifier")]
    pub prerelease_identifier: PrereleaseIdentifier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChannelValue {
    Stable(bool),
    Named(String),
}

impl Default for ChannelValue {
    fn default() -> Self {
        ChannelValue::Stable(false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PrereleaseIdentifier {
    #[default]
    Increment,
    Timestamp,
    ShortHash,
    TimestampHash,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReleaseCommitMode {
    #[default]
    Commit,
    Pr,
    None,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReleaseCommitScope {
    #[default]
    Grouped,
    PerPackage,
}
