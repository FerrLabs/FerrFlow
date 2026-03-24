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
    #[serde(rename = "gomod")]
    GoMod,
    Gradle,
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
        if root.join("build.gradle").exists() || root.join("build.gradle.kts").exists() {
            let path = if root.join("build.gradle.kts").exists() {
                "build.gradle.kts"
            } else {
                "build.gradle"
            };
            versioned_files.push(VersionedFile {
                path: path.to_string(),
                format: FileFormat::Gradle,
            });
        }
        if root.join("go.mod").exists() {
            versioned_files.push(VersionedFile {
                path: "go.mod".to_string(),
                format: FileFormat::GoMod,
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

fn prompt(question: &str, default: &str) -> String {
    use std::io::Write;
    if default.is_empty() {
        print!("{question}: ");
    } else {
        print!("{question} [{default}]: ");
    }
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed
    }
}

fn prompt_bool(question: &str, default: bool) -> bool {
    let hint = if default { "Y/n" } else { "y/N" };
    let answer = prompt(&format!("{question} [{hint}]"), "");
    match answer.to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default,
    }
}

const ALLOWED_FORMATS: &[&str] = &["toml", "json", "xml", "gradle", "gomod"];

fn prompt_format(indent: bool) -> String {
    let question = if indent {
        "  Version file format [toml/json/xml/gradle/gomod]"
    } else {
        "Version file format [toml/json/xml/gradle/gomod]"
    };
    loop {
        let input = prompt(question, "toml");
        let normalized = input.trim().to_lowercase();
        if ALLOWED_FORMATS.contains(&normalized.as_str()) {
            return normalized;
        }
        eprintln!(
            "Invalid format '{}'. Allowed values: toml, json, xml, gradle, gomod.",
            input
        );
    }
}

fn default_version_file(format: &str) -> &'static str {
    match format {
        "json" => "package.json",
        "xml" => "pom.xml",
        "gradle" => "build.gradle",
        "gomod" => "go.mod",
        _ => "Cargo.toml",
    }
}

fn collect_package(path_default: &str, monorepo: bool) -> String {
    let dir_name = std::env::current_dir()
        .ok()
        .and_then(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "project".to_string());

    let name = if monorepo {
        prompt("  Package name", "")
    } else {
        prompt("Package name", &dir_name)
    };

    let path = prompt(if monorepo { "  Path" } else { "Path" }, path_default);

    let format = prompt_format(monorepo);

    let version_file_default = default_version_file(&format);
    let version_file_path = if path == "." {
        prompt(
            if monorepo {
                "  Version file path"
            } else {
                "Version file path"
            },
            version_file_default,
        )
    } else {
        prompt(
            if monorepo {
                "  Version file path"
            } else {
                "Version file path"
            },
            &format!("{path}/{version_file_default}"),
        )
    };

    let changelog_default = if path == "." {
        "CHANGELOG.md".to_string()
    } else {
        format!("{path}/CHANGELOG.md")
    };
    let changelog = prompt(
        if monorepo {
            "  Changelog path"
        } else {
            "Changelog path"
        },
        &changelog_default,
    );

    format!(
        "\n[[package]]\nname = \"{name}\"\npath = \"{path}\"\nchangelog = \"{changelog}\"\n\n[[package.versioned_files]]\npath = \"{version_file_path}\"\nformat = \"{format}\"\n"
    )
}

pub fn init() -> Result<()> {
    let config_path = PathBuf::from(CONFIG_FILE);
    if config_path.exists() {
        anyhow::bail!("ferrflow.toml already exists");
    }

    let mut output = String::from("[workspace]\n");

    let monorepo = prompt_bool("Is this a monorepo?", false);

    if monorepo {
        println!("Add packages (leave name empty to finish):");
        let mut count = 0;
        loop {
            let name = prompt("  Package name", "");
            if name.is_empty() {
                if count == 0 {
                    eprintln!("At least one package is required.");
                    continue;
                }
                break;
            }
            let path = prompt("  Path", &name);
            let format = prompt_format(true);
            let version_file_default = default_version_file(&format);
            let version_file_path = if path == "." {
                prompt("  Version file path", version_file_default)
            } else {
                prompt(
                    "  Version file path",
                    &format!("{path}/{version_file_default}"),
                )
            };
            let changelog = prompt("  Changelog path", &format!("{path}/CHANGELOG.md"));
            output.push_str(&format!(
                "\n[[package]]\nname = \"{name}\"\npath = \"{path}\"\nchangelog = \"{changelog}\"\n\n[[package.versioned_files]]\npath = \"{version_file_path}\"\nformat = \"{format}\"\n"
            ));
            count += 1;
        }
    } else {
        output.push_str(&collect_package(".", false));
    }

    std::fs::write(&config_path, &output)?;
    println!("Created ferrflow.toml");
    println!("Run: ferrflow check");
    Ok(())
}
