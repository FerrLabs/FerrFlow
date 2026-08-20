use serde::{Deserialize, Serialize};
use std::fmt;

use super::util::pick_higher_semver;
use crate::config::VersionSourcePolicy;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(in crate::monorepo) enum VersionSource {
    Tag { tag: String },
    File { file: String },
    TagOverFile { tag: String, file: String },
    FileOverTag { file: String, tag: String },
    TagByPolicy { tag: String, file: String },
    FileByPolicy { file: String, tag: String },
    Bootstrap,
}

impl VersionSource {
    pub(in crate::monorepo) fn resolve(
        tag: Option<(String, String)>,
        file: Option<(String, String)>,
        policy: VersionSourcePolicy,
    ) -> (Option<String>, Self) {
        match (tag, file) {
            (Some((tag_name, tag_version)), Some((file_path, file_version))) => match policy {
                VersionSourcePolicy::Tag => (
                    Some(tag_version),
                    Self::TagByPolicy {
                        tag: tag_name,
                        file: file_path,
                    },
                ),
                VersionSourcePolicy::File => (
                    Some(file_version),
                    Self::FileByPolicy {
                        file: file_path,
                        tag: tag_name,
                    },
                ),
                VersionSourcePolicy::Highest => {
                    let winner = pick_higher_semver(&file_version, &tag_version);
                    let source = if winner == tag_version {
                        Self::TagOverFile {
                            tag: tag_name,
                            file: file_path,
                        }
                    } else {
                        Self::FileOverTag {
                            file: file_path,
                            tag: tag_name,
                        }
                    };
                    (Some(winner), source)
                }
            },
            (Some((tag_name, tag_version)), None) => {
                (Some(tag_version), Self::Tag { tag: tag_name })
            }
            (None, Some((file_path, file_version))) => {
                (Some(file_version), Self::File { file: file_path })
            }
            (None, None) => (None, Self::Bootstrap),
        }
    }
}

impl fmt::Display for VersionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tag { tag } => write!(f, "from tag {tag}"),
            Self::File { file } => write!(f, "from {file}"),
            Self::TagOverFile { tag, file } => write!(f, "from tag {tag}, over {file}"),
            Self::FileOverTag { file, tag } => write!(f, "from {file}, over tag {tag}"),
            Self::TagByPolicy { tag, file } => {
                write!(f, "from tag {tag}, {file} ignored by versionSource: tag")
            }
            Self::FileByPolicy { file, tag } => {
                write!(f, "from {file}, tag {tag} ignored by versionSource: file")
            }
            Self::Bootstrap => write!(f, "bootstrapped"),
        }
    }
}

#[cfg(test)]
mod tests;
