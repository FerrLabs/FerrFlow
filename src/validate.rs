use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::config::{Config, FileFormat};
use crate::error_code::{self, ErrorCodeExt};
use crate::formats::get_handler;
use crate::git::{get_repo_root, open_repo};

// ---------------------------------------------------------------------------
// FileSource trait
// ---------------------------------------------------------------------------

pub trait FileSource {
    fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>>;
    fn path_exists(&self, path: &str) -> Result<bool>;
}

pub struct LocalSource {
    pub root: PathBuf,
}

impl FileSource for LocalSource {
    fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let full = self.root.join(path);
        match std::fs::read(&full) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn path_exists(&self, path: &str) -> Result<bool> {
        Ok(self.root.join(path).exists())
    }
}

// ---------------------------------------------------------------------------
// Remote sources
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum RemoteProvider {
    GitHub,
    GitLab,
}

pub fn parse_repo_spec(spec: &str) -> Result<(RemoteProvider, String, String)> {
    let spec = spec
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let parts: Vec<&str> = spec.split('/').collect();
    match parts.len() {
        2 => Ok((
            RemoteProvider::GitHub,
            parts[0].to_string(),
            parts[1].to_string(),
        )),
        3 => {
            let host = parts[0].to_lowercase();
            let provider = if host.contains("gitlab") {
                RemoteProvider::GitLab
            } else {
                RemoteProvider::GitHub
            };
            Ok((provider, parts[1].to_string(), parts[2].to_string()))
        }
        _ => Err(anyhow::anyhow!(
            "Invalid repo spec: {spec}. Expected owner/repo or host/owner/repo"
        ))
        .error_code(error_code::VALIDATE_INVALID_REPO_SPEC)?,
    }
}

pub struct GitHubSource {
    pub owner: String,
    pub repo: String,
    pub git_ref: Option<String>,
    pub token: Option<String>,
}

impl FileSource for GitHubSource {
    fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let mut url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            self.owner, self.repo, path
        );
        if let Some(ref r) = self.git_ref {
            url.push_str(&format!("?ref={r}"));
        }
        let mut req = ureq::get(&url).header("Accept", "application/vnd.github.v3.raw");
        if let Some(ref token) = self.token {
            req = req.header("Authorization", &format!("Bearer {token}"));
        }
        req = req.header("User-Agent", "ferrflow");
        match req.call() {
            Ok(mut resp) => {
                let body = resp.body_mut().read_to_vec()?;
                Ok(Some(body))
            }
            Err(ureq::Error::StatusCode(404)) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("GitHub API error for {path}: {e}"))
                .error_code(error_code::VALIDATE_GITHUB_API),
        }
    }

    fn path_exists(&self, path: &str) -> Result<bool> {
        Ok(self.read_file(path)?.is_some())
    }
}

pub struct GitLabSource {
    pub owner: String,
    pub repo: String,
    pub git_ref: Option<String>,
    pub token: Option<String>,
}

impl FileSource for GitLabSource {
    fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let project_id = format!("{}/{}", self.owner, self.repo);
        let encoded_project = project_id.replace('/', "%2F");
        let encoded_path = path.replace('/', "%2F");
        let mut url = format!(
            "https://gitlab.com/api/v4/projects/{encoded_project}/repository/files/{encoded_path}/raw"
        );
        if let Some(ref r) = self.git_ref {
            url.push_str(&format!("?ref={r}"));
        } else {
            url.push_str("?ref=main");
        }
        let mut req = ureq::get(&url);
        if let Some(ref token) = self.token {
            req = req.header("PRIVATE-TOKEN", token);
        }
        req = req.header("User-Agent", "ferrflow");
        match req.call() {
            Ok(mut resp) => {
                let body = resp.body_mut().read_to_vec()?;
                Ok(Some(body))
            }
            Err(ureq::Error::StatusCode(404)) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("GitLab API error for {path}: {e}"))
                .error_code(error_code::VALIDATE_GITLAB_API),
        }
    }

    fn path_exists(&self, path: &str) -> Result<bool> {
        Ok(self.read_file(path)?.is_some())
    }
}

// ---------------------------------------------------------------------------
// Config loading from FileSource
// ---------------------------------------------------------------------------

const CONFIG_FILENAMES: &[&str] = &[
    "ferrflow.json",
    "ferrflow.json5",
    "ferrflow.toml",
    ".ferrflow",
];

fn parse_config_content(content: &[u8], filename: &str) -> Result<Config> {
    let text = std::str::from_utf8(content)
        .with_context(|| format!("Invalid UTF-8 in {filename}"))
        .error_code(error_code::VALIDATE_INVALID_UTF8)?;
    match filename {
        f if f.ends_with(".toml") => toml_edit::de::from_str(text)
            .with_context(|| format!("Failed to parse {filename}"))
            .error_code(error_code::VALIDATE_PARSE_FAILED),
        f if f.ends_with(".json5") => json5::from_str(text)
            .with_context(|| format!("Failed to parse {filename}"))
            .error_code(error_code::VALIDATE_PARSE_FAILED),
        _ => serde_json::from_str(text)
            .with_context(|| format!("Failed to parse {filename}"))
            .error_code(error_code::VALIDATE_PARSE_FAILED),
    }
}

pub fn load_config_from_source(
    source: &dyn FileSource,
    explicit_path: Option<&str>,
) -> Result<(Config, String)> {
    if let Some(path) = explicit_path {
        let content = source
            .read_file(path)?
            .ok_or_else(|| anyhow::anyhow!("Config file not found: {path}"))
            .error_code(error_code::VALIDATE_FILE_NOT_FOUND)?;
        let config = parse_config_content(&content, path)?;
        return Ok((config, path.to_string()));
    }
    for filename in CONFIG_FILENAMES {
        if let Some(content) = source.read_file(filename)? {
            let config = parse_config_content(&content, filename)?;
            return Ok((config, filename.to_string()));
        }
    }
    Err(anyhow::anyhow!(
        "No FerrFlow configuration file found. Looked for: {}",
        CONFIG_FILENAMES.join(", ")
    ))
    .error_code(error_code::VALIDATE_NO_CONFIG)?
}

// ---------------------------------------------------------------------------
// Validation types
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum ValidationLevel {
    Error,
    Warning,
    Suggestion,
}

#[derive(Debug)]
pub struct ValidationEntry {
    pub level: ValidationLevel,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub config_file: Option<String>,
    pub package_count: usize,
    pub errors: Vec<EntryOutput>,
    pub warnings: Vec<EntryOutput>,
    pub suggestions: Vec<EntryOutput>,
}

#[derive(Debug, Serialize)]
pub struct EntryOutput {
    pub path: String,
    pub message: String,
}

impl ValidationResult {
    pub fn from_entries(entries: Vec<ValidationEntry>) -> Self {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();
        for entry in entries {
            let output = EntryOutput {
                path: entry.path,
                message: entry.message,
            };
            match entry.level {
                ValidationLevel::Error => errors.push(output),
                ValidationLevel::Warning => warnings.push(output),
                ValidationLevel::Suggestion => suggestions.push(output),
            }
        }
        let valid = errors.is_empty();
        Self {
            valid,
            config_file: None,
            package_count: 0,
            errors,
            warnings,
            suggestions,
        }
    }
}

// ---------------------------------------------------------------------------
// Validation passes
// ---------------------------------------------------------------------------

fn check_duplicate_names(config: &Config) -> Vec<ValidationEntry> {
    let mut seen: HashMap<&str, &str> = HashMap::new();
    let mut entries = Vec::new();
    for pkg in &config.packages {
        if let Some(prev_path) = seen.insert(&pkg.name, &pkg.path) {
            entries.push(ValidationEntry {
                level: ValidationLevel::Error,
                path: "(config)".to_string(),
                message: format!(
                    "duplicate package name \"{}\" (paths: \"{}\", \"{}\")",
                    pkg.name, prev_path, pkg.path
                ),
            });
        }
    }
    entries
}

fn check_duplicate_paths(config: &Config) -> Vec<ValidationEntry> {
    let mut seen: HashMap<String, &str> = HashMap::new();
    let mut entries = Vec::new();
    for pkg in &config.packages {
        let normalized = pkg.path.trim_end_matches('/').to_string();
        if let Some(prev_name) = seen.insert(normalized, &pkg.name) {
            entries.push(ValidationEntry {
                level: ValidationLevel::Error,
                path: "(config)".to_string(),
                message: format!(
                    "duplicate package path \"{}\" (packages: \"{}\", \"{}\")",
                    pkg.path, prev_name, pkg.name
                ),
            });
        }
    }
    entries
}

fn check_tag_templates(config: &Config) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    let is_monorepo = config.packages.len() > 1;

    let mut check_template = |template: &str, context: &str| {
        if !template.contains("{version}") {
            entries.push(ValidationEntry {
                level: ValidationLevel::Error,
                path: context.to_string(),
                message: format!("tag template \"{template}\" must contain {{version}}"),
            });
        }
        if is_monorepo && !template.contains("{name}") {
            entries.push(ValidationEntry {
                level: ValidationLevel::Warning,
                path: context.to_string(),
                message: format!(
                    "tag template \"{template}\" does not contain {{name}} — tags will collide in monorepo"
                ),
            });
        }
    };

    if let Some(ref tmpl) = config.workspace.tag_template {
        check_template(tmpl, "workspace.tagTemplate");
    }
    for pkg in &config.packages {
        if let Some(ref tmpl) = pkg.tag_template {
            check_template(tmpl, &format!("{}.tagTemplate", pkg.name));
        }
    }
    entries
}

fn check_package_paths(config: &Config, source: &dyn FileSource) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    for pkg in &config.packages {
        if pkg.path == "." {
            continue;
        }
        match source.path_exists(&pkg.path) {
            Ok(true) => {}
            Ok(false) => entries.push(ValidationEntry {
                level: ValidationLevel::Error,
                path: pkg.path.clone(),
                message: format!("package path \"{}\" does not exist", pkg.path),
            }),
            Err(e) => entries.push(ValidationEntry {
                level: ValidationLevel::Error,
                path: pkg.path.clone(),
                message: format!("cannot check path \"{}\": {e}", pkg.path),
            }),
        }
    }
    entries
}

fn check_versioned_files_exist(config: &Config, source: &dyn FileSource) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    for pkg in &config.packages {
        for vf in &pkg.versioned_files {
            match source.path_exists(&vf.path) {
                Ok(true) => {}
                Ok(false) => entries.push(ValidationEntry {
                    level: ValidationLevel::Error,
                    path: vf.path.clone(),
                    message: format!("versioned file \"{}\" does not exist", vf.path),
                }),
                Err(e) => entries.push(ValidationEntry {
                    level: ValidationLevel::Error,
                    path: vf.path.clone(),
                    message: format!("cannot check file \"{}\": {e}", vf.path),
                }),
            }
        }
    }
    entries
}

type PackageVersionMap = HashMap<String, Vec<(String, String)>>;

fn check_versioned_files(
    config: &Config,
    source: &dyn FileSource,
) -> (Vec<ValidationEntry>, PackageVersionMap) {
    let mut entries = Vec::new();
    let mut versions: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for pkg in &config.packages {
        for vf in &pkg.versioned_files {
            if vf.format == FileFormat::GoMod {
                entries.push(ValidationEntry {
                    level: ValidationLevel::Warning,
                    path: vf.path.clone(),
                    message:
                        "go.mod version is derived from git tags, cannot validate file content"
                            .to_string(),
                });
                continue;
            }

            let content = match source.read_file(&vf.path) {
                Ok(Some(c)) => c,
                Ok(None) => continue,
                Err(e) => {
                    entries.push(ValidationEntry {
                        level: ValidationLevel::Error,
                        path: vf.path.clone(),
                        message: format!("cannot read \"{}\": {e}", vf.path),
                    });
                    continue;
                }
            };

            let handler = get_handler(&vf.format);
            match handler.read_version_from_bytes(&content, &vf.path) {
                Ok(version) => {
                    versions
                        .entry(pkg.name.clone())
                        .or_default()
                        .push((vf.path.clone(), version));
                }
                Err(e) => entries.push(ValidationEntry {
                    level: ValidationLevel::Error,
                    path: vf.path.clone(),
                    message: format!("cannot read version from \"{}\": {e}", vf.path),
                }),
            }
        }
    }
    (entries, versions)
}

fn check_version_consistency(
    versions: &HashMap<String, Vec<(String, String)>>,
) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    for (pkg_name, file_versions) in versions {
        if file_versions.len() < 2 {
            continue;
        }
        let first_version = &file_versions[0].1;
        for (file_path, version) in &file_versions[1..] {
            if version != first_version {
                entries.push(ValidationEntry {
                    level: ValidationLevel::Error,
                    path: pkg_name.clone(),
                    message: format!(
                        "version mismatch: \"{}\" has \"{}\", \"{}\" has \"{}\"",
                        file_versions[0].0, first_version, file_path, version
                    ),
                });
            }
        }
    }
    entries
}

fn check_changelog_paths(config: &Config, source: &dyn FileSource) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    for pkg in &config.packages {
        if let Some(ref changelog) = pkg.changelog {
            match source.path_exists(changelog) {
                Ok(true) => {}
                Ok(false) => entries.push(ValidationEntry {
                    level: ValidationLevel::Warning,
                    path: changelog.clone(),
                    message: format!("changelog \"{}\" does not exist yet", changelog),
                }),
                Err(e) => entries.push(ValidationEntry {
                    level: ValidationLevel::Warning,
                    path: changelog.clone(),
                    message: format!("cannot check changelog \"{}\": {e}", changelog),
                }),
            }
        }
    }
    entries
}

fn check_shared_paths(config: &Config, source: &dyn FileSource) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    for pkg in &config.packages {
        for sp in &pkg.shared_paths {
            match source.path_exists(sp) {
                Ok(true) => {}
                Ok(false) => entries.push(ValidationEntry {
                    level: ValidationLevel::Warning,
                    path: sp.clone(),
                    message: format!("shared path \"{}\" does not exist", sp),
                }),
                Err(e) => entries.push(ValidationEntry {
                    level: ValidationLevel::Warning,
                    path: sp.clone(),
                    message: format!("cannot check shared path \"{}\": {e}", sp),
                }),
            }
        }
    }
    entries
}

fn check_suggestions(config: &Config) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    for pkg in &config.packages {
        if pkg.versioned_files.is_empty() {
            entries.push(ValidationEntry {
                level: ValidationLevel::Suggestion,
                path: pkg.name.clone(),
                message: "no versionedFiles declared, ferrflow will use auto-detection".to_string(),
            });
        }
    }
    if config.workspace.tag_template.is_none() {
        let default = if config.packages.len() > 1 {
            "{name}@v{version}"
        } else {
            "v{version}"
        };
        entries.push(ValidationEntry {
            level: ValidationLevel::Suggestion,
            path: "workspace.tagTemplate".to_string(),
            message: format!("not set, using default \"{default}\""),
        });
    }
    entries
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

fn output_result(result: &ValidationResult, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
    } else {
        print_text_result(result);
    }
    if !result.valid {
        std::process::exit(1);
    }
    Ok(())
}

fn print_text_result(result: &ValidationResult) {
    println!();
    println!("{}", "ferrflow validate".bold());
    println!();

    if let Some(ref cf) = result.config_file {
        println!("  {} config parsed ({})", "✓".green(), cf);
    }
    if result.package_count > 0 {
        println!(
            "  {} {} package{} found",
            "✓".green(),
            result.package_count,
            if result.package_count == 1 { "" } else { "s" }
        );
    }

    for e in &result.errors {
        println!("  {} {}: {}", "✗".red(), e.path, e.message);
    }
    for w in &result.warnings {
        println!("  {} {}: {}", "⚠".yellow(), w.path, w.message);
    }
    for s in &result.suggestions {
        println!("  {} {}: {}", "◆".cyan(), s.path, s.message);
    }

    if result.errors.is_empty() && result.warnings.is_empty() && result.suggestions.is_empty() {
        println!("  {} no issues found", "✓".green());
    }

    println!();
    let parts: Vec<String> = [
        (result.errors.len(), "error"),
        (result.warnings.len(), "warning"),
        (result.suggestions.len(), "suggestion"),
    ]
    .iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, label)| format!("{n} {label}{}", if *n > 1 { "s" } else { "" }))
    .collect();

    if parts.is_empty() {
        println!("  {}", "all checks passed".green().bold());
    } else {
        println!("  {}", parts.join(", "));
    }
    println!();
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(
    config_path: Option<&Path>,
    json: bool,
    repo: Option<&str>,
    git_ref: Option<&str>,
) -> Result<()> {
    if git_ref.is_some() && repo.is_none() {
        Err(anyhow::anyhow!("--ref requires --repo"))
            .error_code(error_code::VALIDATE_REF_REQUIRES_REPO)?;
    }

    let source: Box<dyn FileSource> = if let Some(repo_spec) = repo {
        let (provider, owner, repo_name) = parse_repo_spec(repo_spec)?;
        match provider {
            RemoteProvider::GitHub => Box::new(GitHubSource {
                owner,
                repo: repo_name,
                git_ref: git_ref.map(|s| s.to_string()),
                token: std::env::var("FERRFLOW_TOKEN")
                    .ok()
                    .or_else(|| std::env::var("GITHUB_TOKEN").ok()),
            }),
            RemoteProvider::GitLab => Box::new(GitLabSource {
                owner,
                repo: repo_name,
                git_ref: git_ref.map(|s| s.to_string()),
                token: std::env::var("FERRFLOW_TOKEN")
                    .ok()
                    .or_else(|| std::env::var("GITLAB_TOKEN").ok()),
            }),
        }
    } else {
        let repo = open_repo(&std::env::current_dir()?)?;
        let root = get_repo_root(&repo)?;
        Box::new(LocalSource { root })
    };

    let config_path_str = config_path.and_then(|p| p.to_str());

    let (config, config_filename) = match load_config_from_source(source.as_ref(), config_path_str)
    {
        Ok(result) => result,
        Err(e) => {
            let mut result = ValidationResult::from_entries(vec![ValidationEntry {
                level: ValidationLevel::Error,
                path: "(config)".to_string(),
                message: e.to_string(),
            }]);
            result.config_file = None;
            return output_result(&result, json);
        }
    };

    let mut entries = Vec::new();
    entries.extend(check_duplicate_names(&config));
    entries.extend(check_duplicate_paths(&config));
    entries.extend(check_tag_templates(&config));
    entries.extend(check_package_paths(&config, source.as_ref()));
    entries.extend(check_versioned_files_exist(&config, source.as_ref()));
    let (file_entries, versions) = check_versioned_files(&config, source.as_ref());
    entries.extend(file_entries);
    entries.extend(check_version_consistency(&versions));
    entries.extend(check_changelog_paths(&config, source.as_ref()));
    entries.extend(check_shared_paths(&config, source.as_ref()));
    entries.extend(check_suggestions(&config));

    let mut result = ValidationResult::from_entries(entries);
    result.config_file = Some(config_filename);
    result.package_count = config.packages.len();
    output_result(&result, json)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PackageConfig, VersionedFile, WorkspaceConfig};
    use std::fs;
    use tempfile::TempDir;

    fn make_config(packages: Vec<PackageConfig>) -> Config {
        Config {
            workspace: WorkspaceConfig::default(),
            packages,
        }
    }

    fn make_package(name: &str, path: &str) -> PackageConfig {
        PackageConfig {
            name: name.to_string(),
            path: path.to_string(),
            versioned_files: vec![],
            changelog: None,
            shared_paths: vec![],
            depends_on: vec![],
            versioning: None,
            tag_template: None,
            floating_tags: None,
            hooks: None,
        }
    }

    // -- LocalSource --

    #[test]
    fn local_source_read_existing_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("test.txt"), "hello").unwrap();
        let source = LocalSource {
            root: tmp.path().to_path_buf(),
        };
        assert_eq!(
            source.read_file("test.txt").unwrap(),
            Some(b"hello".to_vec())
        );
    }

    #[test]
    fn local_source_read_missing_file() {
        let tmp = TempDir::new().unwrap();
        let source = LocalSource {
            root: tmp.path().to_path_buf(),
        };
        assert_eq!(source.read_file("nope.txt").unwrap(), None);
    }

    #[test]
    fn local_source_path_exists() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file.txt"), "x").unwrap();
        fs::create_dir(tmp.path().join("subdir")).unwrap();
        let source = LocalSource {
            root: tmp.path().to_path_buf(),
        };
        assert!(source.path_exists("file.txt").unwrap());
        assert!(source.path_exists("subdir").unwrap());
        assert!(!source.path_exists("nope.txt").unwrap());
    }

    // -- parse_repo_spec --

    #[test]
    fn parse_repo_spec_github_short() {
        let (p, o, r) = parse_repo_spec("owner/repo").unwrap();
        assert_eq!(p, RemoteProvider::GitHub);
        assert_eq!(o, "owner");
        assert_eq!(r, "repo");
    }

    #[test]
    fn parse_repo_spec_github_full() {
        let (p, o, r) = parse_repo_spec("github.com/owner/repo").unwrap();
        assert_eq!(p, RemoteProvider::GitHub);
        assert_eq!(o, "owner");
        assert_eq!(r, "repo");
    }

    #[test]
    fn parse_repo_spec_gitlab() {
        let (p, o, r) = parse_repo_spec("gitlab.com/owner/repo").unwrap();
        assert_eq!(p, RemoteProvider::GitLab);
        assert_eq!(o, "owner");
        assert_eq!(r, "repo");
    }

    #[test]
    fn parse_repo_spec_invalid() {
        assert!(parse_repo_spec("just-a-name").is_err());
    }

    // -- ValidationResult --

    #[test]
    fn validation_result_valid_when_no_errors() {
        let result = ValidationResult::from_entries(vec![ValidationEntry {
            level: ValidationLevel::Warning,
            path: "test".to_string(),
            message: "just a warning".to_string(),
        }]);
        assert!(result.valid);
    }

    #[test]
    fn validation_result_invalid_when_errors() {
        let result = ValidationResult::from_entries(vec![ValidationEntry {
            level: ValidationLevel::Error,
            path: "test".to_string(),
            message: "broken".to_string(),
        }]);
        assert!(!result.valid);
    }

    // -- load_config_from_source --

    #[test]
    fn load_config_local() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("ferrflow.json"),
            r#"{"package": [{"name": "app", "path": "."}]}"#,
        )
        .unwrap();
        let source = LocalSource {
            root: tmp.path().to_path_buf(),
        };
        let (config, filename) = load_config_from_source(&source, None).unwrap();
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.packages[0].name, "app");
        assert_eq!(filename, "ferrflow.json");
    }

    #[test]
    fn load_config_priority_order() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("ferrflow.json"),
            r#"{"package": [{"name": "json", "path": "."}]}"#,
        )
        .unwrap();
        fs::write(
            tmp.path().join(".ferrflow"),
            r#"{"package": [{"name": "dotfile", "path": "."}]}"#,
        )
        .unwrap();
        let source = LocalSource {
            root: tmp.path().to_path_buf(),
        };
        let (config, _) = load_config_from_source(&source, None).unwrap();
        assert_eq!(config.packages[0].name, "json");
    }

    #[test]
    fn load_config_not_found() {
        let tmp = TempDir::new().unwrap();
        let source = LocalSource {
            root: tmp.path().to_path_buf(),
        };
        assert!(load_config_from_source(&source, None).is_err());
    }

    #[test]
    fn load_config_explicit_path() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("custom.json"),
            r#"{"package": [{"name": "custom", "path": "."}]}"#,
        )
        .unwrap();
        let source = LocalSource {
            root: tmp.path().to_path_buf(),
        };
        let (config, filename) = load_config_from_source(&source, Some("custom.json")).unwrap();
        assert_eq!(config.packages[0].name, "custom");
        assert_eq!(filename, "custom.json");
    }

    // -- Validation passes --

    #[test]
    fn pass_duplicate_names() {
        let config = make_config(vec![
            make_package("app", "packages/a"),
            make_package("app", "packages/b"),
        ]);
        let entries = check_duplicate_names(&config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, ValidationLevel::Error);
        assert!(entries[0].message.contains("app"));
    }

    #[test]
    fn pass_no_duplicate_names() {
        let config = make_config(vec![
            make_package("api", "packages/api"),
            make_package("web", "packages/web"),
        ]);
        assert!(check_duplicate_names(&config).is_empty());
    }

    #[test]
    fn pass_duplicate_paths() {
        let config = make_config(vec![
            make_package("a", "packages/app"),
            make_package("b", "packages/app"),
        ]);
        let entries = check_duplicate_paths(&config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, ValidationLevel::Error);
    }

    #[test]
    fn pass_tag_template_missing_version() {
        let mut config = make_config(vec![make_package("app", ".")]);
        config.workspace.tag_template = Some("{name}-release".to_string());
        let entries = check_tag_templates(&config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, ValidationLevel::Error);
        assert!(entries[0].message.contains("{version}"));
    }

    #[test]
    fn pass_tag_template_missing_name_monorepo() {
        let mut config = make_config(vec![
            make_package("api", "packages/api"),
            make_package("web", "packages/web"),
        ]);
        config.workspace.tag_template = Some("v{version}".to_string());
        let entries = check_tag_templates(&config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, ValidationLevel::Warning);
        assert!(entries[0].message.contains("{name}"));
    }

    #[test]
    fn pass_package_paths_exist() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("packages/api")).unwrap();
        let source = LocalSource {
            root: tmp.path().to_path_buf(),
        };
        let config = make_config(vec![make_package("api", "packages/api")]);
        assert!(check_package_paths(&config, &source).is_empty());
    }

    #[test]
    fn pass_package_paths_missing() {
        let tmp = TempDir::new().unwrap();
        let source = LocalSource {
            root: tmp.path().to_path_buf(),
        };
        let config = make_config(vec![make_package("api", "packages/api")]);
        let entries = check_package_paths(&config, &source);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, ValidationLevel::Error);
    }

    #[test]
    fn pass_versioned_files_parseable() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("packages/api")).unwrap();
        fs::write(
            tmp.path().join("packages/api/package.json"),
            r#"{"name": "api", "version": "1.0.0"}"#,
        )
        .unwrap();
        let source = LocalSource {
            root: tmp.path().to_path_buf(),
        };
        let mut pkg = make_package("api", "packages/api");
        pkg.versioned_files = vec![VersionedFile {
            path: "packages/api/package.json".to_string(),
            format: FileFormat::Json,
        }];
        let config = make_config(vec![pkg]);
        let (entries, versions) = check_versioned_files(&config, &source);
        assert!(entries.is_empty());
        assert_eq!(versions["api"].len(), 1);
        assert_eq!(versions["api"][0].1, "1.0.0");
    }

    #[test]
    fn pass_version_consistency_mismatch() {
        let mut versions = HashMap::new();
        versions.insert(
            "app".to_string(),
            vec![
                ("package.json".to_string(), "1.0.0".to_string()),
                ("Cargo.toml".to_string(), "1.1.0".to_string()),
            ],
        );
        let entries = check_version_consistency(&versions);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, ValidationLevel::Error);
        assert!(entries[0].message.contains("1.0.0"));
        assert!(entries[0].message.contains("1.1.0"));
    }

    #[test]
    fn pass_version_consistency_ok() {
        let mut versions = HashMap::new();
        versions.insert(
            "app".to_string(),
            vec![
                ("package.json".to_string(), "1.0.0".to_string()),
                ("Cargo.toml".to_string(), "1.0.0".to_string()),
            ],
        );
        assert!(check_version_consistency(&versions).is_empty());
    }

    #[test]
    fn run_ref_without_repo_errors() {
        let result = run(None, false, None, Some("main"));
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("--ref"));
    }
}
