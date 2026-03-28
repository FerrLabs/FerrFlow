use anyhow::{Context, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::telemetry;

// ---------------------------------------------------------------------------
// Config structs
// ---------------------------------------------------------------------------

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
    #[serde(default = "default_telemetry")]
    pub telemetry: bool,
    #[serde(default)]
    pub versioning: VersioningStrategy,
    pub tag_template: Option<String>,
}

fn default_telemetry() -> bool {
    true
}

fn default_remote() -> String {
    "origin".to_string()
}

fn default_branch() -> String {
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
    pub versioning: Option<VersioningStrategy>,
    pub tag_template: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum VersioningStrategy {
    #[default]
    Semver,
    Calver,
    CalverShort,
    CalverSeq,
    Sequential,
    Zerover,
}

impl PackageConfig {
    pub fn effective_versioning(&self, workspace: &WorkspaceConfig) -> VersioningStrategy {
        self.versioning.unwrap_or(workspace.versioning)
    }

    fn effective_template<'a>(
        &'a self,
        workspace: &'a WorkspaceConfig,
        is_monorepo: bool,
    ) -> &'a str {
        self.tag_template
            .as_deref()
            .or(workspace.tag_template.as_deref())
            .unwrap_or(if is_monorepo {
                "{name}@v{version}"
            } else {
                "v{version}"
            })
    }

    pub fn tag_for_version(
        &self,
        workspace: &WorkspaceConfig,
        is_monorepo: bool,
        version: &str,
    ) -> String {
        self.effective_template(workspace, is_monorepo)
            .replace("{name}", &self.name)
            .replace("{version}", version)
    }

    pub fn tag_prefix(&self, workspace: &WorkspaceConfig, is_monorepo: bool) -> String {
        let template = self.effective_template(workspace, is_monorepo);
        let prefix = template.split("{version}").next().unwrap_or(template);
        prefix.replace("{name}", &self.name)
    }
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

// ---------------------------------------------------------------------------
// Config file format enum (for CLI --format flag)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ConfigFileFormat {
    Json,
    Json5,
    Toml,
    Dotfile,
}

// ---------------------------------------------------------------------------
// ConfigFormatHandler trait + implementations
// ---------------------------------------------------------------------------

pub trait ConfigFormatHandler {
    fn filename(&self) -> &str;
    fn parse(&self, content: &str) -> Result<Config>;
    fn serialize(&self, config: &Config) -> Result<String>;
}

struct JsonFormat;
struct Json5Format;
struct TomlFormat;
struct DotfileFormat;

impl ConfigFormatHandler for JsonFormat {
    fn filename(&self) -> &str {
        "ferrflow.json"
    }
    fn parse(&self, content: &str) -> Result<Config> {
        serde_json::from_str(content).with_context(|| "Failed to parse ferrflow.json")
    }
    fn serialize(&self, config: &Config) -> Result<String> {
        let mut out = serde_json::to_string_pretty(config)?;
        out.push('\n');
        Ok(out)
    }
}

impl ConfigFormatHandler for Json5Format {
    fn filename(&self) -> &str {
        "ferrflow.json5"
    }
    fn parse(&self, content: &str) -> Result<Config> {
        json5::from_str(content).with_context(|| "Failed to parse ferrflow.json5")
    }
    fn serialize(&self, config: &Config) -> Result<String> {
        // json5 crate has no serializer; valid JSON is valid JSON5
        let mut out = serde_json::to_string_pretty(config)?;
        out.push('\n');
        Ok(out)
    }
}

impl ConfigFormatHandler for TomlFormat {
    fn filename(&self) -> &str {
        "ferrflow.toml"
    }
    fn parse(&self, content: &str) -> Result<Config> {
        toml_edit::de::from_str(content).with_context(|| "Failed to parse ferrflow.toml")
    }
    fn serialize(&self, config: &Config) -> Result<String> {
        toml_edit::ser::to_string_pretty(config).with_context(|| "Failed to serialize to TOML")
    }
}

impl ConfigFormatHandler for DotfileFormat {
    fn filename(&self) -> &str {
        ".ferrflow"
    }
    fn parse(&self, content: &str) -> Result<Config> {
        ConfigFormatHandler::parse(&JsonFormat, content)
            .with_context(|| "Failed to parse .ferrflow")
    }
    fn serialize(&self, config: &Config) -> Result<String> {
        ConfigFormatHandler::serialize(&JsonFormat, config)
            .with_context(|| "Failed to serialize .ferrflow")
    }
}

/// Ordered by priority: json > json5 > toml > .ferrflow
const CONFIG_FORMATS: &[&dyn ConfigFormatHandler] =
    &[&JsonFormat, &Json5Format, &TomlFormat, &DotfileFormat];

pub fn format_handler(fmt: ConfigFileFormat) -> &'static dyn ConfigFormatHandler {
    match fmt {
        ConfigFileFormat::Json => &JsonFormat,
        ConfigFileFormat::Json5 => &Json5Format,
        ConfigFileFormat::Toml => &TomlFormat,
        ConfigFileFormat::Dotfile => &DotfileFormat,
    }
}

// ---------------------------------------------------------------------------
// Config loading
// ---------------------------------------------------------------------------

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

        let mut found: Vec<(&dyn ConfigFormatHandler, PathBuf)> = Vec::new();

        for handler in CONFIG_FORMATS {
            let path = repo_root.join(handler.filename());
            if path.exists() {
                found.push((*handler, path));
            }
        }

        if found.is_empty() {
            return Ok(Self::auto_detect(repo_root));
        }

        if found.len() > 1 {
            let names: Vec<&str> = found.iter().map(|(h, _)| h.filename()).collect();
            anyhow::bail!(
                "multiple config files found: {}\nUse --config <path> to specify which one to use.",
                names.join(", ")
            );
        }

        let (handler, path) = &found[0];
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        handler.parse(&content)
    }

    fn load_explicit(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::Error::new(e).context(format!("Config file not found: {}", path.display()))
            } else {
                anyhow::Error::new(e)
                    .context(format!("Failed to read config file: {}", path.display()))
            }
        })?;

        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let handler: &dyn ConfigFormatHandler = match ext {
            "json5" => &Json5Format,
            "toml" => &TomlFormat,
            "json" => &JsonFormat,
            _ if filename == ".ferrflow" => &DotfileFormat,
            _ => &JsonFormat,
        };

        handler.parse(&content)
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
                    versioning: None,
                    tag_template: None,
                }]
            },
        }
    }

    pub fn is_monorepo(&self) -> bool {
        self.packages.len() > 1
    }
}

// ---------------------------------------------------------------------------
// Interactive helpers
// ---------------------------------------------------------------------------

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

const ALLOWED_CONFIG_FORMATS: &[&str] = &["json", "json5", "toml", "dotfile"];

fn prompt_config_format() -> ConfigFileFormat {
    let question = "Config file format [json/json5/toml/dotfile]";
    loop {
        let input = prompt(question, "json");
        let normalized = input.trim().to_lowercase();
        if ALLOWED_CONFIG_FORMATS.contains(&normalized.as_str()) {
            return match normalized.as_str() {
                "json5" => ConfigFileFormat::Json5,
                "toml" => ConfigFileFormat::Toml,
                "dotfile" | ".ferrflow" => ConfigFileFormat::Dotfile,
                _ => ConfigFileFormat::Json,
            };
        }
        eprintln!(
            "Invalid format '{}'. Allowed values: json, json5, toml, dotfile.",
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

fn parse_file_format(s: &str) -> FileFormat {
    match s {
        "json" => FileFormat::Json,
        "xml" => FileFormat::Xml,
        "gradle" => FileFormat::Gradle,
        "gomod" => FileFormat::GoMod,
        _ => FileFormat::Toml,
    }
}

fn collect_package(path_default: &str, monorepo: bool) -> PackageConfig {
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

    let format_str = prompt_format(monorepo);

    let version_file_default = default_version_file(&format_str);
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

    PackageConfig {
        name,
        path,
        versioned_files: vec![VersionedFile {
            path: version_file_path,
            format: parse_file_format(&format_str),
        }],
        changelog: Some(changelog),
        shared_paths: Vec::new(),
        versioning: None,
        tag_template: None,
    }
}

// ---------------------------------------------------------------------------
// Init command
// ---------------------------------------------------------------------------

pub fn init(format: Option<ConfigFileFormat>) -> Result<()> {
    // Check if any config file already exists
    for handler in CONFIG_FORMATS {
        let path = PathBuf::from(handler.filename());
        if path.exists() {
            anyhow::bail!("{} already exists", handler.filename());
        }
    }

    let fmt = format.unwrap_or_else(prompt_config_format);
    let handler = format_handler(fmt);

    let monorepo = prompt_bool("Is this a monorepo?", false);

    let packages = if monorepo {
        println!("Add packages (leave name empty to finish):");
        let mut pkgs = Vec::new();
        loop {
            let pkg = collect_package("", true);
            if pkg.name.is_empty() {
                if pkgs.is_empty() {
                    eprintln!("At least one package is required.");
                    continue;
                }
                break;
            }
            pkgs.push(pkg);
        }
        pkgs
    } else {
        vec![collect_package(".", false)]
    };

    let config = Config {
        workspace: WorkspaceConfig::default(),
        packages,
    };

    let content = handler.serialize(&config)?;
    let filename = handler.filename();
    std::fs::write(filename, &content)?;
    println!("Created {filename}");
    println!("Run: ferrflow check");

    if config.workspace.telemetry {
        telemetry::send_event("init", None, None, None);
    }

    Ok(())
}
