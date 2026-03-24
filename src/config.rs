use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "ferrflow.toml";

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub workspace: WorkspaceConfig,
    #[serde(default, rename = "package")]
    pub packages: Vec<PackageConfig>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct WorkspaceConfig {
    #[serde(default = "default_remote")]
    pub remote: String,
    #[serde(default = "default_branch")]
    pub branch: String,
}

fn default_remote() -> String {
    "origin".to_string()
}

fn default_branch() -> String {
    // Try to detect the default branch from the remote HEAD ref
    let detected = std::process::Command::new("git")
        .args(["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().trim_start_matches("origin/").to_string())
        .filter(|s| !s.is_empty());

    detected.unwrap_or_else(|| "main".to_string())
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PackageConfig {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub versioned_files: Vec<VersionedFile>,
    pub changelog: Option<String>,
    #[serde(default)]
    pub shared_paths: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VersionedFile {
    pub path: String,
    pub format: FileFormat,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Json,
    Toml,
    Xml,
}

impl Config {
    pub fn load(repo_root: &Path) -> Result<Self> {
        let config_path = repo_root.join(CONFIG_FILE);
        if !config_path.exists() {
            return Ok(Self::auto_detect(repo_root));
        }
        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;
        toml_edit::de::from_str(&content).with_context(|| "Failed to parse ferrflow.toml")
    }

    fn auto_detect(root: &Path) -> Self {
        let mut versioned_files = Vec::new();

        if root.join("Cargo.toml").exists() {
            versioned_files.push(VersionedFile {
                path: "Cargo.toml".to_string(),
                format: FileFormat::Toml,
            });
        }
        if root.join("package.json").exists() {
            versioned_files.push(VersionedFile {
                path: "package.json".to_string(),
                format: FileFormat::Json,
            });
        }
        if root.join("pom.xml").exists() {
            versioned_files.push(VersionedFile {
                path: "pom.xml".to_string(),
                format: FileFormat::Xml,
            });
        }
        if root.join("pyproject.toml").exists() {
            versioned_files.push(VersionedFile {
                path: "pyproject.toml".to_string(),
                format: FileFormat::Toml,
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
                }]
            },
        }
    }

    pub fn is_monorepo(&self) -> bool {
        self.packages.len() > 1
    }
}

pub fn init() -> Result<()> {
    let config_path = PathBuf::from(CONFIG_FILE);
    if config_path.exists() {
        anyhow::bail!("ferrflow.toml already exists");
    }

    let example = r#"# FerrFlow configuration
# https://github.com/FerrFlow/FerrFlow

[workspace]
remote = "origin"
branch = "main"

# Single package example:
[[package]]
name = "my-app"
path = "."
changelog = "CHANGELOG.md"

[[package.versioned_files]]
path = "Cargo.toml"
format = "toml"

# Monorepo example:
# [[package]]
# name = "api"
# path = "services/api"
# shared_paths = ["services/shared/"]
# changelog = "services/api/CHANGELOG.md"
#
# [[package.versioned_files]]
# path = "services/api/Cargo.toml"
# format = "toml"
#
# [[package]]
# name = "frontend"
# path = "frontend"
# changelog = "frontend/CHANGELOG.md"
#
# [[package.versioned_files]]
# path = "frontend/package.json"
# format = "json"
"#;

    std::fs::write(&config_path, example)?;
    println!("✓ Created ferrflow.toml");
    println!("  Edit it to configure your packages, then run: ferrflow check");
    Ok(())
}
