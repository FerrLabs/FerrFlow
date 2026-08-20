use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(in crate::monorepo) enum VersionSource {
    Tag { tag: String },
    File { file: String },
    TagOverFile { tag: String, file: String },
    FileOverTag { file: String, tag: String },
    Bootstrap,
}

impl VersionSource {
    pub(in crate::monorepo) fn resolve(
        tag: Option<(String, String)>,
        file: Option<(String, String)>,
        winner: &str,
    ) -> Self {
        match (tag, file) {
            (Some((tag_name, tag_version)), Some((file_path, _))) => {
                if winner == tag_version {
                    Self::TagOverFile {
                        tag: tag_name,
                        file: file_path,
                    }
                } else {
                    Self::FileOverTag {
                        file: file_path,
                        tag: tag_name,
                    }
                }
            }
            (Some((tag_name, _)), None) => Self::Tag { tag: tag_name },
            (None, Some((file_path, _))) => Self::File { file: file_path },
            (None, None) => Self::Bootstrap,
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
            Self::Bootstrap => write!(f, "bootstrapped"),
        }
    }
}

#[cfg(test)]
mod tests;
