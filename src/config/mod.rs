use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error_code::{self, ErrorCodeExt};

mod format;
#[cfg(feature = "cli")]
mod init;
#[cfg(feature = "cli")]
mod loader_js;
mod package;
mod types;
mod workspace;

#[allow(unused_imports)]
pub use format::{ConfigFileFormat, ConfigFormatHandler, format_handler};
#[cfg(feature = "cli")]
pub use init::init;
pub use package::{FileFormat, FloatingTagLevel, PackageConfig, VersionedFile, VersioningStrategy};
pub use types::{
    BranchChannelConfig, ChannelValue, ForgeKind, HooksConfig, OnFailure, OrphanedTagStrategy,
    PrereleaseIdentifier, ReleaseCommitMode, ReleaseCommitScope,
};
#[allow(unused_imports)]
pub use workspace::{WorkspaceConfig, default_commit_skip_markers};

use format::{CONFIG_FORMATS, DotfileFormat, Json5Format, JsonFormat, TomlFormat};
#[cfg(feature = "cli")]
use loader_js::{JS_CONFIG_FILENAME, TS_CONFIG_FILENAME, load_js_ts_config};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub workspace: WorkspaceConfig,
    #[serde(default, rename = "package")]
    pub packages: Vec<PackageConfig>,
}

impl Config {
    pub fn load(repo_root: &Path, explicit_path: Option<&Path>) -> Result<Self> {
        if let Some(path) = explicit_path {
            let resolved_path = if path.is_relative() {
                repo_root.join(path)
            } else {
                path.to_path_buf()
            };
            return Self::load_explicit(&resolved_path);
        }

        // Build ordered search list: json > json5 > toml > ts > js > .ferrflow
        let mut search: Vec<&str> = CONFIG_FORMATS.iter().map(|h| h.filename()).collect();

        #[cfg(feature = "cli")]
        {
            // Insert ts/js before .ferrflow (last element)
            let dotfile_pos = search.len() - 1;
            search.insert(dotfile_pos, TS_CONFIG_FILENAME);
            search.insert(dotfile_pos + 1, JS_CONFIG_FILENAME);
        }

        let mut found: Vec<PathBuf> = Vec::new();
        for filename in &search {
            let path = repo_root.join(filename);
            if path.exists() {
                found.push(path);
            }
        }

        if found.is_empty() {
            return Ok(Self::auto_detect(repo_root));
        }

        if found.len() > 1 {
            let names: Vec<String> = found
                .iter()
                .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .collect();
            Err(anyhow::anyhow!(
                "multiple config files found: {}\nUse --config <path> to specify which one to use.",
                names.join(", ")
            ))
            .error_code(error_code::CONFIG_MULTIPLE_FILES)?;
        }

        Self::load_from_path(&found[0])
    }

    fn load_from_path(path: &Path) -> Result<Self> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        #[cfg(feature = "cli")]
        if ext == "ts" || ext == "js" {
            return load_js_ts_config(path);
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))
            .error_code(error_code::CONFIG_READ_FAILED)?;

        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let handler: &dyn ConfigFormatHandler = match ext {
            "json5" => &Json5Format,
            "toml" => &TomlFormat,
            "json" => &JsonFormat,
            _ if filename == ".ferrflow" => &DotfileFormat,
            _ => &JsonFormat,
        };

        handler.parse(&content)
    }

    fn load_explicit(path: &Path) -> Result<Self> {
        if !path.exists() {
            Err(anyhow::anyhow!("Config file not found: {}", path.display()))
                .error_code(error_code::CONFIG_NOT_FOUND)?;
        }
        Self::load_from_path(path)
    }

    fn auto_detect(root: &Path) -> Self {
        let mut versioned_files = Vec::new();

        if root.join("Cargo.toml").exists() {
            versioned_files.push(VersionedFile {
                path: "Cargo.toml".to_string(),
                format: FileFormat::Toml,
                selector: None,
            });
        }
        if root.join("build.gradle").exists() || root.join("build.gradle.kts").exists() {
            let path = if root.join("build.gradle.kts").exists() {
                "build.gradle.kts"
            } else {
                "build.gradle"
            };
            versioned_files.push(VersionedFile {
                path: path.to_string(),
                format: FileFormat::Gradle,
                selector: None,
            });
        }
        if root.join("Chart.yaml").exists() {
            versioned_files.push(VersionedFile {
                path: "Chart.yaml".to_string(),
                format: FileFormat::Helm,
                selector: None,
            });
        }
        if root.join("go.mod").exists() {
            versioned_files.push(VersionedFile {
                path: "go.mod".to_string(),
                format: FileFormat::GoMod,
                selector: None,
            });
        }
        if root.join("package.json").exists() {
            versioned_files.push(VersionedFile {
                path: "package.json".to_string(),
                format: FileFormat::Json,
                selector: None,
            });
        }
        if root.join("pom.xml").exists() {
            versioned_files.push(VersionedFile {
                path: "pom.xml".to_string(),
                format: FileFormat::Xml,
                selector: None,
            });
        }
        for name in &["VERSION", "VERSION.txt"] {
            if root.join(name).exists() {
                versioned_files.push(VersionedFile {
                    path: name.to_string(),
                    format: FileFormat::Txt,
                    selector: None,
                });
                break;
            }
        }
        if root.join("pyproject.toml").exists() {
            versioned_files.push(VersionedFile {
                path: "pyproject.toml".to_string(),
                format: FileFormat::Toml,
                selector: None,
            });
        }

        let name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string();

        Config {
            workspace: WorkspaceConfig::default(),
            packages: if versioned_files.is_empty() {
                vec![]
            } else {
                vec![PackageConfig {
                    name,
                    path: ".".to_string(),
                    versioned_files,
                    changelog: Some("CHANGELOG.md".to_string()),
                    shared_paths: Vec::new(),
                    depends_on: vec![],
                    versioning: None,
                    tag_template: None,
                    hooks: None,
                    floating_tags: None,
                }]
            },
        }
    }

    pub fn is_monorepo(&self) -> bool {
        self.packages.len() > 1
    }
}

#[cfg(test)]
mod tests;
