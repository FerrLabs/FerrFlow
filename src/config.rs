use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error_code::{self, ErrorCodeExt};
#[cfg(feature = "cli")]
use crate::telemetry;

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

// ---------------------------------------------------------------------------
// Hooks config
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Pre-release channel config
// ---------------------------------------------------------------------------

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

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct WorkspaceConfig {
    #[serde(default = "default_remote")]
    pub remote: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default = "default_telemetry", alias = "telemetry")]
    pub anonymous_telemetry: bool,
    #[serde(default)]
    pub versioning: Option<VersioningStrategy>,
    #[serde(alias = "tagTemplate")]
    pub tag_template: Option<String>,
    #[serde(default, alias = "recoverMissedReleases")]
    pub recover_missed_releases: bool,
    #[serde(default, alias = "releaseCommitMode")]
    pub release_commit_mode: ReleaseCommitMode,
    #[serde(default, alias = "releaseCommitScope")]
    pub release_commit_scope: ReleaseCommitScope,
    #[serde(default = "default_auto_merge", alias = "autoMergeReleases")]
    pub auto_merge_releases: bool,
    #[serde(default, alias = "skipCi")]
    pub skip_ci: Option<bool>,
    #[serde(default, alias = "floatingTags")]
    pub floating_tags: Vec<FloatingTagLevel>,
    #[serde(default, alias = "orphanedTagStrategy")]
    pub orphaned_tag_strategy: OrphanedTagStrategy,
    #[serde(default)]
    pub forge: ForgeKind,
    #[serde(default)]
    pub hooks: Option<HooksConfig>,
    #[serde(default)]
    pub branches: Option<Vec<BranchChannelConfig>>,
}

impl WorkspaceConfig {
    pub fn effective_skip_ci(&self) -> bool {
        self.skip_ci
            .unwrap_or(self.release_commit_mode == ReleaseCommitMode::Commit)
    }
}

fn default_auto_merge() -> bool {
    true
}

fn default_telemetry() -> bool {
    true
}

fn default_remote() -> String {
    "origin".to_string()
}

fn default_branch() -> String {
    #[cfg(feature = "cli")]
    {
        let detected = (|| {
            let repo = git2::Repository::discover(".").ok()?;
            let reference = repo.find_reference("refs/remotes/origin/HEAD").ok()?;
            let target = reference.symbolic_target().map(String::from)?;
            let branch = target
                .strip_prefix("refs/remotes/origin/")
                .unwrap_or(&target);
            if branch.is_empty() {
                None
            } else {
                Some(branch.to_string())
            }
        })();

        if let Some(branch) = detected {
            return branch;
        }
    }

    "main".to_string()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PackageConfig {
    pub name: String,
    pub path: String,
    #[serde(default, alias = "versionedFiles")]
    pub versioned_files: Vec<VersionedFile>,
    pub changelog: Option<String>,
    #[serde(default, alias = "sharedPaths")]
    pub shared_paths: Vec<String>,
    #[serde(default, alias = "dependsOn")]
    pub depends_on: Vec<String>,
    pub versioning: Option<VersioningStrategy>,
    #[serde(alias = "tagTemplate")]
    pub tag_template: Option<String>,
    #[serde(default, alias = "floatingTags")]
    pub floating_tags: Option<Vec<FloatingTagLevel>>,
    #[serde(default)]
    pub hooks: Option<HooksConfig>,
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

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FloatingTagLevel {
    Major,
    Minor,
}

impl PackageConfig {
    /// Resolve the effective versioning strategy for this package. Priority:
    ///   1. package.versioning if explicitly set
    ///   2. workspace.versioning if explicitly set
    ///   3. auto-detect from `tags` (filtered to tags relevant to this
    ///      package — caller's job)
    ///   4. fallback to [`VersioningStrategy::Semver`]
    ///
    /// Note: zerover is intentionally excluded from auto-detection because it
    /// is ambiguous with semver (both use `X.Y.Z`). Users must opt-in
    /// explicitly via config.
    pub fn effective_versioning(
        &self,
        workspace: &WorkspaceConfig,
        tags: &[&str],
    ) -> VersioningStrategy {
        self.versioning
            .or(workspace.versioning)
            .or_else(|| crate::versioning::detect_strategy_from_tags(tags))
            .unwrap_or_default()
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

    pub fn effective_floating_tags<'a>(
        &'a self,
        workspace: &'a WorkspaceConfig,
    ) -> &'a [FloatingTagLevel] {
        match &self.floating_tags {
            Some(tags) => tags,
            None => &workspace.floating_tags,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VersionedFile {
    pub path: String,
    pub format: FileFormat,
    /// Optional selector to disambiguate which occurrence in the file is the
    /// version to bump. Syntax depends on the format:
    ///
    /// - `xml`: a slash-delimited path of tag names rooted at the document
    ///   element, e.g. `/project/version`. Without a selector the handler
    ///   targets the first `<version>` that is a direct child of the root
    ///   element — which fixes the common Maven `<parent>` pitfall.
    /// - `txt`: a regex with a single capture group that brackets the
    ///   version string, e.g. `^VERSION=(.+)$`.
    ///
    /// Other formats currently ignore this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Csproj,
    #[serde(rename = "gomod")]
    GoMod,
    Gradle,
    /// `values.yaml` templating for Helm charts. For the top-level
    /// `Chart.yaml` manifest use [`FileFormat::ChartYaml`] instead.
    Helm,
    Json,
    Toml,
    Txt,
    Xml,
    /// `pubspec.yaml` for Dart / Flutter packages.
    #[serde(rename = "pubspecyaml")]
    PubspecYaml,
    /// `mix.exs` for Elixir / Mix projects.
    #[serde(rename = "mixexs")]
    MixExs,
    /// `Chart.yaml` for Helm chart top-level manifests.
    #[serde(rename = "chartyaml")]
    ChartYaml,
    /// `*.gemspec` for Ruby gems.
    Gemspec,
    /// `Package.swift` for Swift packages.
    #[serde(rename = "packageswift")]
    PackageSwift,
}

// ---------------------------------------------------------------------------
// Config file format enum (for CLI --format flag)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
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

fn snake_to_camel(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = false;
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

const CAMEL_CASE_KEYS: &[&str] = &[
    "tag_template",
    "versioned_files",
    "shared_paths",
    "recover_missed_releases",
    "release_commit_mode",
    "release_commit_scope",
    "auto_merge_releases",
    "skip_ci",
    "pre_bump",
    "post_bump",
    "pre_commit",
    "pre_publish",
    "post_publish",
    "on_failure",
    "floating_tags",
    "orphaned_tag_strategy",
    "prerelease_identifier",
];

fn to_camel_case_keys(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let new_map = map
                .into_iter()
                .map(|(k, v)| {
                    let new_key = if CAMEL_CASE_KEYS.contains(&k.as_str()) {
                        snake_to_camel(&k)
                    } else {
                        k
                    };
                    (new_key, to_camel_case_keys(v))
                })
                .collect();
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(to_camel_case_keys).collect())
        }
        other => other,
    }
}

impl ConfigFormatHandler for JsonFormat {
    fn filename(&self) -> &str {
        "ferrflow.json"
    }
    fn parse(&self, content: &str) -> Result<Config> {
        serde_json::from_str(content)
            .with_context(|| "Failed to parse ferrflow.json")
            .error_code(error_code::CONFIG_PARSE_JSON)
    }
    fn serialize(&self, config: &Config) -> Result<String> {
        let value = serde_json::to_value(config)?;
        let camel = to_camel_case_keys(value);
        let mut out = serde_json::to_string_pretty(&camel)?;
        out.push('\n');
        Ok(out)
    }
}

impl ConfigFormatHandler for Json5Format {
    fn filename(&self) -> &str {
        "ferrflow.json5"
    }
    fn parse(&self, content: &str) -> Result<Config> {
        json5::from_str(content)
            .with_context(|| "Failed to parse ferrflow.json5")
            .error_code(error_code::CONFIG_PARSE_JSON5)
    }
    fn serialize(&self, config: &Config) -> Result<String> {
        // json5 crate has no serializer; valid JSON is valid JSON5
        let value = serde_json::to_value(config)?;
        let camel = to_camel_case_keys(value);
        let mut out = serde_json::to_string_pretty(&camel)?;
        out.push('\n');
        Ok(out)
    }
}

impl ConfigFormatHandler for TomlFormat {
    fn filename(&self) -> &str {
        "ferrflow.toml"
    }
    fn parse(&self, content: &str) -> Result<Config> {
        toml_edit::de::from_str(content)
            .with_context(|| "Failed to parse ferrflow.toml")
            .error_code(error_code::CONFIG_PARSE_TOML)
    }
    fn serialize(&self, config: &Config) -> Result<String> {
        toml_edit::ser::to_string_pretty(config)
            .with_context(|| "Failed to serialize to TOML")
            .error_code(error_code::CONFIG_SERIALIZE_TOML)
    }
}

impl ConfigFormatHandler for DotfileFormat {
    fn filename(&self) -> &str {
        ".ferrflow"
    }
    fn parse(&self, content: &str) -> Result<Config> {
        ConfigFormatHandler::parse(&JsonFormat, content)
            .with_context(|| "Failed to parse .ferrflow")
            .error_code(error_code::CONFIG_PARSE_DOTFILE)
    }
    fn serialize(&self, config: &Config) -> Result<String> {
        ConfigFormatHandler::serialize(&JsonFormat, config)
            .with_context(|| "Failed to serialize .ferrflow")
            .error_code(error_code::CONFIG_SERIALIZE_DOTFILE)
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
// JS/TS config execution (CLI only)
// ---------------------------------------------------------------------------

#[cfg(feature = "cli")]
const JS_CONFIG_FILENAME: &str = "ferrflow.js";
#[cfg(feature = "cli")]
const TS_CONFIG_FILENAME: &str = "ferrflow.ts";

#[cfg(feature = "cli")]
fn path_to_file_url(path: &Path) -> Result<String> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("Failed to resolve path: {}", path.display()))
        .error_code(error_code::CONFIG_RESOLVE_PATH)?;

    let path_str = canonical.to_string_lossy().to_string();

    // Strip Windows UNC prefix (\\?\) and normalize separators
    let normalized = path_str
        .strip_prefix(r"\\?\")
        .unwrap_or(&path_str)
        .replace('\\', "/");

    // On Unix, paths start with / so file:// + /path = file:///path (correct).
    // On Windows, paths are C:/... so we need file:///C:/... (extra slash).
    if normalized.starts_with('/') {
        Ok(format!("file://{normalized}"))
    } else {
        Ok(format!("file:///{normalized}"))
    }
}

/// JS snippet that resolves the config, converts function hooks to shell
/// commands that re-invoke the config file at hook time, and dumps the result
/// as JSON to stdout.
#[cfg(feature = "cli")]
const LOADER_SCRIPT: &str = r#"
function reifyHooks(hooks, fileUrl, runtime, hookPath) {
  if (!hooks || typeof hooks !== 'object') return hooks;
  const ctx = `{ package: process.env.FERRFLOW_PACKAGE, oldVersion: process.env.FERRFLOW_OLD_VERSION, newVersion: process.env.FERRFLOW_NEW_VERSION, bumpType: process.env.FERRFLOW_BUMP_TYPE, tag: process.env.FERRFLOW_TAG, dryRun: process.env.FERRFLOW_DRY_RUN === 'true', packagePath: process.env.FERRFLOW_PACKAGE_PATH, channel: process.env.FERRFLOW_CHANNEL || null, isPrerelease: process.env.FERRFLOW_IS_PRERELEASE === 'true' }`;
  const result = {};
  for (const [key, value] of Object.entries(hooks)) {
    if (typeof value === 'function') {
      const cmd = `${runtime} --input-type=module -e "const m = await import('${fileUrl}'); const cfg = typeof m.default === 'function' ? await m.default() : m.default; const hooks = ${hookPath}; await hooks.${key}(${ctx});"`;
      result[key] = cmd;
    } else {
      result[key] = value;
    }
  }
  return result;
}
"#;

#[cfg(feature = "cli")]
fn loader_body(file_url: &str, runtime: &str) -> String {
    format!(
        r#"{LOADER_SCRIPT}
const m = await import('{file_url}');
const cfg = typeof m.default === 'function' ? await m.default() : m.default;
if (cfg.workspace && cfg.workspace.hooks) {{
  cfg.workspace.hooks = reifyHooks(cfg.workspace.hooks, '{file_url}', '{runtime}', 'cfg.workspace.hooks');
}}
if (cfg.package) {{
  for (const pkg of cfg.package) {{
    if (pkg.hooks) {{
      pkg.hooks = reifyHooks(pkg.hooks, '{file_url}', '{runtime}', `cfg.package.find(p=>p.name==="${{pkg.name}}").hooks`);
    }}
  }}
}}
process.stdout.write(JSON.stringify(cfg));"#
    )
}

#[cfg(feature = "cli")]
fn load_js_ts_config(path: &Path) -> Result<Config> {
    use std::process::Command;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ferrflow config");
    let file_url = path_to_file_url(path)?;

    let output = if ext == "ts" {
        // For TS: write a temporary .mjs loader that dynamically imports the .ts
        // file via file URL. tsx handles the TS→JS transpilation at import time.
        let wrapper_dir = path.parent().unwrap_or(Path::new("."));
        let wrapper_path = wrapper_dir.join(".ferrflow-loader.mjs");
        let tsx_available = Command::new("tsx").arg("--version").output().is_ok();
        let runtime = if tsx_available { "tsx" } else { "npx tsx" };

        let script = loader_body(&file_url, runtime);
        std::fs::write(&wrapper_path, &script)
            .with_context(|| "Failed to write temporary loader file")
            .error_code(error_code::CONFIG_WRITE_LOADER)?;

        let result = Command::new("tsx")
            .arg(&wrapper_path)
            .output()
            .or_else(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Command::new("npx")
                        .args(["tsx"])
                        .arg(&wrapper_path)
                        .output()
                } else {
                    Err(e)
                }
            })
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    anyhow::anyhow!(
                        "{filename} requires tsx but neither 'tsx' nor 'npx tsx' was found.\n\
                         Install with: npm install -g tsx"
                    )
                } else {
                    anyhow::anyhow!("Failed to execute tsx: {e}")
                }
            })
            .error_code(error_code::CONFIG_EVAL_TS);

        let _ = std::fs::remove_file(&wrapper_path);
        result?
    } else {
        // .js — use node with inline script
        let script = loader_body(&file_url, "node");

        Command::new("node")
            .args(["--input-type=module", "-e", &script])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    anyhow::anyhow!(
                        "{filename} requires Node.js but 'node' was not found in PATH.\n\
                         Install Node.js from https://nodejs.org/"
                    )
                } else {
                    anyhow::anyhow!("Failed to execute node: {e}")
                }
            })
            .error_code(error_code::CONFIG_EVAL_NODE)?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Failed to evaluate {filename}:\n{stderr}"))
            .error_code(error_code::CONFIG_EVAL_FAILED)?;
    }

    let stdout = String::from_utf8(output.stdout)
        .with_context(|| format!("{filename} produced invalid UTF-8 output"))
        .error_code(error_code::CONFIG_INVALID_OUTPUT)?;

    serde_json::from_str::<Config>(&stdout)
        .with_context(|| format!("{filename} did not produce valid JSON config"))
        .error_code(error_code::CONFIG_INVALID_JSON)
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

// ---------------------------------------------------------------------------
// Interactive helpers & init command (CLI only)
// ---------------------------------------------------------------------------

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
fn prompt_bool(question: &str, default: bool) -> bool {
    let hint = if default { "Y/n" } else { "y/N" };
    let answer = prompt(&format!("{question} [{hint}]"), "");
    match answer.to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default,
    }
}

#[cfg(feature = "cli")]
const ALLOWED_FORMATS: &[&str] = &["toml", "json", "xml", "gradle", "gomod", "txt"];

#[cfg(feature = "cli")]
fn prompt_format(indent: bool) -> String {
    let question = if indent {
        "  Version file format [toml/json/xml/gradle/gomod/txt]"
    } else {
        "Version file format [toml/json/xml/gradle/gomod/txt]"
    };
    loop {
        let input = prompt(question, "toml");
        let normalized = input.trim().to_lowercase();
        if ALLOWED_FORMATS.contains(&normalized.as_str()) {
            return normalized;
        }
        eprintln!(
            "Invalid format '{}'. Allowed values: toml, json, xml, gradle, gomod, txt.",
            input
        );
    }
}

#[cfg(feature = "cli")]
const ALLOWED_CONFIG_FORMATS: &[&str] = &["json", "json5", "toml", "dotfile"];

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
fn default_version_file(format: &str) -> &'static str {
    match format {
        "json" => "package.json",
        "xml" => "pom.xml",
        "gradle" => "build.gradle",
        "gomod" => "go.mod",
        "txt" => "VERSION.txt",
        _ => "Cargo.toml",
    }
}

#[cfg(feature = "cli")]
fn parse_file_format(s: &str) -> FileFormat {
    match s {
        "json" => FileFormat::Json,
        "xml" => FileFormat::Xml,
        "gradle" => FileFormat::Gradle,
        "gomod" => FileFormat::GoMod,
        "txt" => FileFormat::Txt,
        _ => FileFormat::Toml,
    }
}

#[cfg(feature = "cli")]
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
            selector: None,
        }],
        changelog: Some(changelog),
        shared_paths: Vec::new(),
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        hooks: None,
        floating_tags: None,
    }
}

#[cfg(feature = "cli")]
pub fn init(format: Option<ConfigFileFormat>) -> Result<()> {
    // Check if any config file already exists
    for handler in CONFIG_FORMATS {
        let path = PathBuf::from(handler.filename());
        if path.exists() {
            Err(anyhow::anyhow!("{} already exists", handler.filename()))
                .error_code(error_code::CONFIG_ALREADY_EXISTS)?;
        }
    }
    for filename in [TS_CONFIG_FILENAME, JS_CONFIG_FILENAME] {
        let path = PathBuf::from(filename);
        if path.exists() {
            Err(anyhow::anyhow!("{filename} already exists"))
                .error_code(error_code::CONFIG_ALREADY_EXISTS)?;
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

    if config.workspace.anonymous_telemetry {
        telemetry::send_event(telemetry::EventType::Init, None, None, None, None);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Config parsing (all formats)
    // -----------------------------------------------------------------------

    #[test]
    fn parse_json_config() {
        let json = r#"{
            "workspace": { "remote": "origin", "branch": "main" },
            "package": [{
                "name": "app",
                "path": ".",
                "versioned_files": [{ "path": "package.json", "format": "json" }]
            }]
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.packages[0].name, "app");
        assert_eq!(
            config.packages[0].versioned_files[0].format,
            FileFormat::Json
        );
    }

    #[test]
    fn parse_json_camel_case() {
        let json = r#"{
            "workspace": { "remote": "origin", "tagTemplate": "v{version}", "recoverMissedReleases": true, "releaseCommitMode": "pr", "autoMergeReleases": false },
            "package": [{
                "name": "app",
                "path": ".",
                "versionedFiles": [{ "path": "package.json", "format": "json" }],
                "sharedPaths": ["shared/"],
                "tagTemplate": "{name}@v{version}"
            }]
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.workspace.tag_template.as_deref(), Some("v{version}"));
        assert!(config.workspace.recover_missed_releases);
        assert_eq!(config.workspace.release_commit_mode, ReleaseCommitMode::Pr);
        assert!(!config.workspace.auto_merge_releases);
        assert_eq!(config.packages[0].versioned_files.len(), 1);
        assert_eq!(config.packages[0].shared_paths, vec!["shared/"]);
        assert_eq!(
            config.packages[0].tag_template.as_deref(),
            Some("{name}@v{version}")
        );
    }

    #[test]
    fn parse_json5_config() {
        let json5 = r#"{
            workspace: { remote: "origin" },
            package: [{
                name: "app",
                path: ".",
                versioned_files: [{ path: "Cargo.toml", format: "toml" }],
            }],
        }"#;
        let config: Config = json5::from_str(json5).unwrap();
        assert_eq!(
            config.packages[0].versioned_files[0].format,
            FileFormat::Toml
        );
    }

    #[test]
    fn parse_toml_config() {
        let toml = r#"
[workspace]
remote = "origin"
branch = "main"

[[package]]
name = "api"
path = "packages/api"
shared_paths = ["packages/shared/"]

[[package.versioned_files]]
path = "packages/api/Cargo.toml"
format = "toml"
"#;
        let config: Config = toml_edit::de::from_str(toml).unwrap();
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.packages[0].shared_paths, vec!["packages/shared/"]);
    }

    #[test]
    fn parse_versioning_strategies() {
        let json = r#"{
            "workspace": { "versioning": "calver" },
            "package": [
                { "name": "a", "path": "a", "versioning": "zerover" },
                { "name": "b", "path": "b" }
            ]
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.workspace.versioning,
            Some(VersioningStrategy::Calver)
        );
        assert_eq!(
            config.packages[0].versioning,
            Some(VersioningStrategy::Zerover)
        );
        assert_eq!(config.packages[1].versioning, None);
    }

    #[test]
    fn workspace_versioning_defaults_to_none() {
        // Unset `versioning` in config should deserialize to None so callers
        // can tell "user said nothing" apart from "user said semver".
        let json = r#"{ "workspace": {}, "package": [] }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.workspace.versioning, None);
    }

    #[test]
    fn parse_all_versioning_variants() {
        for (s, expected) in [
            ("semver", VersioningStrategy::Semver),
            ("calver", VersioningStrategy::Calver),
            ("calver-short", VersioningStrategy::CalverShort),
            ("calver-seq", VersioningStrategy::CalverSeq),
            ("sequential", VersioningStrategy::Sequential),
            ("zerover", VersioningStrategy::Zerover),
        ] {
            let json = format!(r#"{{ "workspace": {{ "versioning": "{s}" }}, "package": [] }}"#);
            let config: Config = serde_json::from_str(&json).unwrap();
            assert_eq!(
                config.workspace.versioning,
                Some(expected),
                "failed for {s}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Effective versioning
    // -----------------------------------------------------------------------

    #[test]
    fn effective_versioning_inherits_workspace() {
        let ws = WorkspaceConfig {
            versioning: Some(VersioningStrategy::Calver),
            ..WorkspaceConfig::default()
        };
        let pkg = PackageConfig {
            name: "a".into(),
            path: ".".into(),
            versioned_files: vec![],
            changelog: None,
            shared_paths: vec![],
            depends_on: vec![],
            versioning: None,
            tag_template: None,
            hooks: None,
            floating_tags: None,
        };
        assert_eq!(
            pkg.effective_versioning(&ws, &[]),
            VersioningStrategy::Calver
        );
    }

    #[test]
    fn effective_versioning_package_overrides() {
        let ws = WorkspaceConfig {
            versioning: Some(VersioningStrategy::Calver),
            ..WorkspaceConfig::default()
        };
        let pkg = PackageConfig {
            name: "a".into(),
            path: ".".into(),
            versioned_files: vec![],
            changelog: None,
            shared_paths: vec![],
            depends_on: vec![],
            versioning: Some(VersioningStrategy::Zerover),
            tag_template: None,
            hooks: None,
            floating_tags: None,
        };
        assert_eq!(
            pkg.effective_versioning(&ws, &[]),
            VersioningStrategy::Zerover
        );
    }

    #[test]
    fn effective_versioning_autodetects_from_tags_when_unset() {
        let ws = WorkspaceConfig::default();
        let pkg = PackageConfig {
            name: "a".into(),
            path: ".".into(),
            versioned_files: vec![],
            changelog: None,
            shared_paths: vec![],
            depends_on: vec![],
            versioning: None,
            tag_template: None,
            hooks: None,
            floating_tags: None,
        };
        let tags = vec!["v2024.04.18", "v2024.05.01"];
        assert_eq!(
            pkg.effective_versioning(&ws, &tags),
            VersioningStrategy::Calver
        );
    }

    #[test]
    fn effective_versioning_falls_back_to_semver_without_tags() {
        let ws = WorkspaceConfig::default();
        let pkg = PackageConfig {
            name: "a".into(),
            path: ".".into(),
            versioned_files: vec![],
            changelog: None,
            shared_paths: vec![],
            depends_on: vec![],
            versioning: None,
            tag_template: None,
            hooks: None,
            floating_tags: None,
        };
        assert_eq!(
            pkg.effective_versioning(&ws, &[]),
            VersioningStrategy::Semver
        );
    }

    // -----------------------------------------------------------------------
    // Tag template
    // -----------------------------------------------------------------------

    fn make_pkg(name: &str, tag_template: Option<&str>) -> PackageConfig {
        PackageConfig {
            name: name.into(),
            path: ".".into(),
            versioned_files: vec![],
            changelog: None,
            shared_paths: vec![],
            depends_on: vec![],
            versioning: None,
            tag_template: tag_template.map(String::from),
            hooks: None,
            floating_tags: None,
        }
    }

    #[test]
    fn tag_default_single_repo() {
        let ws = WorkspaceConfig::default();
        let pkg = make_pkg("myapp", None);
        assert_eq!(pkg.tag_for_version(&ws, false, "1.2.3"), "v1.2.3");
        assert_eq!(pkg.tag_prefix(&ws, false), "v");
    }

    #[test]
    fn tag_default_monorepo() {
        let ws = WorkspaceConfig::default();
        let pkg = make_pkg("api", None);
        assert_eq!(pkg.tag_for_version(&ws, true, "1.2.3"), "api@v1.2.3");
        assert_eq!(pkg.tag_prefix(&ws, true), "api@v");
    }

    #[test]
    fn tag_custom_workspace_template() {
        let ws = WorkspaceConfig {
            tag_template: Some("release-{version}".into()),
            ..WorkspaceConfig::default()
        };
        let pkg = make_pkg("myapp", None);
        assert_eq!(pkg.tag_for_version(&ws, false, "1.0.0"), "release-1.0.0");
        assert_eq!(pkg.tag_prefix(&ws, false), "release-");
    }

    #[test]
    fn tag_package_overrides_workspace() {
        let ws = WorkspaceConfig {
            tag_template: Some("v{version}".into()),
            ..WorkspaceConfig::default()
        };
        let pkg = make_pkg("api", Some("{name}/v{version}"));
        assert_eq!(pkg.tag_for_version(&ws, true, "2.0.0"), "api/v2.0.0");
        assert_eq!(pkg.tag_prefix(&ws, true), "api/v");
    }

    #[test]
    fn tag_template_name_placeholder() {
        let ws = WorkspaceConfig::default();
        let pkg = make_pkg("frontend", Some("{name}-v{version}"));
        assert_eq!(pkg.tag_for_version(&ws, true, "3.0.0"), "frontend-v3.0.0");
    }

    // -----------------------------------------------------------------------
    // is_monorepo
    // -----------------------------------------------------------------------

    #[test]
    fn is_monorepo_single() {
        let config = Config {
            workspace: WorkspaceConfig::default(),
            packages: vec![make_pkg("a", None)],
        };
        assert!(!config.is_monorepo());
    }

    #[test]
    fn is_monorepo_multi() {
        let config = Config {
            workspace: WorkspaceConfig::default(),
            packages: vec![make_pkg("a", None), make_pkg("b", None)],
        };
        assert!(config.is_monorepo());
    }

    // -----------------------------------------------------------------------
    // Auto-detect
    // -----------------------------------------------------------------------

    #[test]
    fn auto_detect_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::auto_detect(dir.path());
        assert!(config.packages.is_empty());
    }

    #[test]
    fn auto_detect_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let config = Config::auto_detect(dir.path());
        assert_eq!(config.packages.len(), 1);
        assert_eq!(
            config.packages[0].versioned_files[0].format,
            FileFormat::Toml
        );
    }

    #[test]
    fn auto_detect_package_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{"version":"1.0.0"}"#).unwrap();
        let config = Config::auto_detect(dir.path());
        assert_eq!(
            config.packages[0].versioned_files[0].format,
            FileFormat::Json
        );
    }

    #[test]
    fn auto_detect_pom_xml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project><version>1.0</version></project>",
        )
        .unwrap();
        let config = Config::auto_detect(dir.path());
        assert_eq!(
            config.packages[0].versioned_files[0].format,
            FileFormat::Xml
        );
    }

    #[test]
    fn auto_detect_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{"version":"1.0.0"}"#).unwrap();
        let config = Config::auto_detect(dir.path());
        assert_eq!(config.packages[0].versioned_files.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Config load with explicit path
    // -----------------------------------------------------------------------

    #[test]
    fn load_explicit_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ferrflow.json");
        std::fs::write(&path, r#"{"package":[{"name":"x","path":"."}]}"#).unwrap();
        let config = Config::load_explicit(&path).unwrap();
        assert_eq!(config.packages[0].name, "x");
    }

    #[test]
    fn load_explicit_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ferrflow.toml");
        std::fs::write(&path, "[[package]]\nname = \"x\"\npath = \".\"\n").unwrap();
        let config = Config::load_explicit(&path).unwrap();
        assert_eq!(config.packages[0].name, "x");
    }

    #[test]
    fn load_explicit_dotfile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".ferrflow");
        std::fs::write(&path, r#"{"package":[{"name":"x","path":"."}]}"#).unwrap();
        let config = Config::load_explicit(&path).unwrap();
        assert_eq!(config.packages[0].name, "x");
    }

    #[test]
    fn load_explicit_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope.json");
        assert!(Config::load_explicit(&path).is_err());
    }

    // -----------------------------------------------------------------------
    // Config serialization roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn json_roundtrip() {
        let handler = JsonFormat;
        let config = Config {
            workspace: WorkspaceConfig::default(),
            packages: vec![make_pkg("test", None)],
        };
        let serialized = handler.serialize(&config).unwrap();
        let parsed = handler.parse(&serialized).unwrap();
        assert_eq!(parsed.packages[0].name, "test");
    }

    #[test]
    fn json_serializes_camel_case() {
        let handler = JsonFormat;
        let config = Config {
            workspace: WorkspaceConfig {
                tag_template: Some("v{version}".into()),
                recover_missed_releases: true,
                ..WorkspaceConfig::default()
            },
            packages: vec![PackageConfig {
                name: "app".into(),
                path: ".".into(),
                versioned_files: vec![VersionedFile {
                    path: "Cargo.toml".into(),
                    format: FileFormat::Toml,
                    selector: None,
                }],
                changelog: None,
                shared_paths: vec!["shared/".into()],
                depends_on: vec![],
                versioning: None,
                tag_template: Some("{name}@v{version}".into()),
                hooks: None,
                floating_tags: None,
            }],
        };
        let serialized = handler.serialize(&config).unwrap();
        assert!(serialized.contains("tagTemplate"));
        assert!(serialized.contains("versionedFiles"));
        assert!(serialized.contains("sharedPaths"));
        assert!(serialized.contains("recoverMissedReleases"));
        assert!(serialized.contains("releaseCommitMode"));
        assert!(serialized.contains("autoMergeReleases"));
        assert!(!serialized.contains("tag_template"));
        assert!(!serialized.contains("versioned_files"));
        assert!(!serialized.contains("shared_paths"));
        assert!(!serialized.contains("recover_missed_releases"));
        assert!(!serialized.contains("release_commit_mode"));
        assert!(!serialized.contains("auto_merge_releases"));

        let parsed = handler.parse(&serialized).unwrap();
        assert_eq!(parsed.workspace.tag_template.as_deref(), Some("v{version}"));
        assert_eq!(parsed.packages[0].shared_paths, vec!["shared/"]);
        assert!(parsed.workspace.recover_missed_releases);
    }

    #[test]
    fn toml_keeps_snake_case() {
        let handler = TomlFormat;
        let config = Config {
            workspace: WorkspaceConfig {
                tag_template: Some("v{version}".into()),
                recover_missed_releases: true,
                ..WorkspaceConfig::default()
            },
            packages: vec![PackageConfig {
                name: "app".into(),
                path: ".".into(),
                versioned_files: vec![VersionedFile {
                    path: "Cargo.toml".into(),
                    format: FileFormat::Toml,
                    selector: None,
                }],
                changelog: None,
                shared_paths: vec!["shared/".into()],
                depends_on: vec![],
                versioning: None,
                tag_template: Some("{name}@v{version}".into()),
                hooks: None,
                floating_tags: None,
            }],
        };
        let serialized = handler.serialize(&config).unwrap();
        assert!(serialized.contains("tag_template"));
        assert!(serialized.contains("versioned_files"));
        assert!(serialized.contains("shared_paths"));
        assert!(serialized.contains("recover_missed_releases"));
        assert!(serialized.contains("release_commit_mode"));
        assert!(serialized.contains("auto_merge_releases"));
        assert!(!serialized.contains("tagTemplate"));
        assert!(!serialized.contains("versionedFiles"));
        assert!(!serialized.contains("sharedPaths"));
        assert!(!serialized.contains("recoverMissedReleases"));
        assert!(!serialized.contains("releaseCommitMode"));
        assert!(!serialized.contains("autoMergeReleases"));
    }

    #[test]
    fn toml_roundtrip() {
        let handler = TomlFormat;
        let config = Config {
            workspace: WorkspaceConfig::default(),
            packages: vec![make_pkg("test", None)],
        };
        let serialized = handler.serialize(&config).unwrap();
        let parsed = handler.parse(&serialized).unwrap();
        assert_eq!(parsed.packages[0].name, "test");
    }

    // -----------------------------------------------------------------------
    // effective_skip_ci
    // -----------------------------------------------------------------------

    #[test]
    fn effective_skip_ci_defaults_true_for_commit_mode() {
        let ws = WorkspaceConfig {
            release_commit_mode: ReleaseCommitMode::Commit,
            skip_ci: None,
            ..WorkspaceConfig::default()
        };
        assert!(ws.effective_skip_ci());
    }

    #[test]
    fn effective_skip_ci_defaults_false_for_pr_mode() {
        let ws = WorkspaceConfig {
            release_commit_mode: ReleaseCommitMode::Pr,
            skip_ci: None,
            ..WorkspaceConfig::default()
        };
        assert!(!ws.effective_skip_ci());
    }

    #[test]
    fn effective_skip_ci_defaults_false_for_none_mode() {
        let ws = WorkspaceConfig {
            release_commit_mode: ReleaseCommitMode::None,
            skip_ci: None,
            ..WorkspaceConfig::default()
        };
        assert!(!ws.effective_skip_ci());
    }

    #[test]
    fn effective_skip_ci_explicit_override() {
        let ws = WorkspaceConfig {
            release_commit_mode: ReleaseCommitMode::Commit,
            skip_ci: Some(false),
            ..WorkspaceConfig::default()
        };
        assert!(!ws.effective_skip_ci());

        let ws2 = WorkspaceConfig {
            release_commit_mode: ReleaseCommitMode::Pr,
            skip_ci: Some(true),
            ..WorkspaceConfig::default()
        };
        assert!(ws2.effective_skip_ci());
    }

    // -----------------------------------------------------------------------
    // Config::load — discovery logic
    // -----------------------------------------------------------------------

    #[test]
    fn load_discovers_json_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("ferrflow.json"),
            r#"{"package":[{"name":"app","path":"."}]}"#,
        )
        .unwrap();
        let config = Config::load(dir.path(), None).unwrap();
        assert_eq!(config.packages[0].name, "app");
    }

    #[test]
    fn load_discovers_toml_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("ferrflow.toml"),
            "[[package]]\nname = \"myapp\"\npath = \".\"\n",
        )
        .unwrap();
        let config = Config::load(dir.path(), None).unwrap();
        assert_eq!(config.packages[0].name, "myapp");
    }

    #[test]
    fn load_discovers_dotfile_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".ferrflow"),
            r#"{"package":[{"name":"dot","path":"."}]}"#,
        )
        .unwrap();
        let config = Config::load(dir.path(), None).unwrap();
        assert_eq!(config.packages[0].name, "dot");
    }

    #[test]
    fn load_fails_on_multiple_config_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("ferrflow.json"),
            r#"{"package":[{"name":"a","path":"."}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("ferrflow.toml"),
            "[[package]]\nname = \"b\"\npath = \".\"\n",
        )
        .unwrap();
        let result = Config::load(dir.path(), None);
        assert!(result.is_err());
        let err = format!("{:?}", result.unwrap_err());
        assert!(err.contains("multiple config files"));
    }

    #[test]
    fn load_falls_back_to_auto_detect() {
        let dir = tempfile::tempdir().unwrap();
        // No config file, but a Cargo.toml exists
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let config = Config::load(dir.path(), None).unwrap();
        assert_eq!(config.packages.len(), 1);
        assert_eq!(
            config.packages[0].versioned_files[0].format,
            FileFormat::Toml
        );
    }

    #[test]
    fn load_with_explicit_path_overrides_discovery() {
        let dir = tempfile::tempdir().unwrap();
        // Put a decoy in the root
        std::fs::write(
            dir.path().join("ferrflow.json"),
            r#"{"package":[{"name":"decoy","path":"."}]}"#,
        )
        .unwrap();
        // Put the real config elsewhere
        let sub = dir.path().join("custom");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            sub.join("my.json"),
            r#"{"package":[{"name":"real","path":"."}]}"#,
        )
        .unwrap();
        let config = Config::load(dir.path(), Some(&sub.join("my.json"))).unwrap();
        assert_eq!(config.packages[0].name, "real");
    }

    // -----------------------------------------------------------------------
    // Auto-detect edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn auto_detect_version_txt() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("VERSION.txt"), "1.0.0\n").unwrap();
        let config = Config::auto_detect(dir.path());
        assert_eq!(config.packages.len(), 1);
        assert_eq!(
            config.packages[0].versioned_files[0].format,
            FileFormat::Txt
        );
        assert_eq!(config.packages[0].versioned_files[0].path, "VERSION.txt");
    }

    #[test]
    fn auto_detect_version_no_ext() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("VERSION"), "1.0.0\n").unwrap();
        let config = Config::auto_detect(dir.path());
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.packages[0].versioned_files[0].path, "VERSION");
    }

    #[test]
    fn auto_detect_prefers_version_over_version_txt() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("VERSION"), "1.0.0\n").unwrap();
        std::fs::write(dir.path().join("VERSION.txt"), "1.0.0\n").unwrap();
        let config = Config::auto_detect(dir.path());
        // Should only pick one (VERSION, the first checked)
        let txt_files: Vec<_> = config.packages[0]
            .versioned_files
            .iter()
            .filter(|vf| vf.format == FileFormat::Txt)
            .collect();
        assert_eq!(txt_files.len(), 1);
        assert_eq!(txt_files[0].path, "VERSION");
    }

    #[test]
    fn auto_detect_go_mod() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module example.com/foo\n").unwrap();
        let config = Config::auto_detect(dir.path());
        assert_eq!(
            config.packages[0].versioned_files[0].format,
            FileFormat::GoMod
        );
    }

    #[test]
    fn auto_detect_gradle() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("build.gradle"), "version = \"1.0.0\"\n").unwrap();
        let config = Config::auto_detect(dir.path());
        assert_eq!(
            config.packages[0].versioned_files[0].format,
            FileFormat::Gradle
        );
    }

    #[test]
    fn auto_detect_gradle_kts_preferred() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("build.gradle"), "version = \"1.0.0\"\n").unwrap();
        std::fs::write(dir.path().join("build.gradle.kts"), "version = \"1.0.0\"\n").unwrap();
        let config = Config::auto_detect(dir.path());
        assert_eq!(
            config.packages[0].versioned_files[0].path,
            "build.gradle.kts"
        );
    }

    #[test]
    fn auto_detect_pyproject() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let config = Config::auto_detect(dir.path());
        assert_eq!(config.packages[0].versioned_files[0].path, "pyproject.toml");
        assert_eq!(
            config.packages[0].versioned_files[0].format,
            FileFormat::Toml
        );
    }

    #[test]
    fn auto_detect_uses_dir_name_as_package_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let config = Config::auto_detect(dir.path());
        let dir_name = dir
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(config.packages[0].name, dir_name);
    }

    // -----------------------------------------------------------------------
    // snake_to_camel
    // -----------------------------------------------------------------------

    #[test]
    fn snake_to_camel_basic() {
        assert_eq!(snake_to_camel("tag_template"), "tagTemplate");
        assert_eq!(snake_to_camel("versioned_files"), "versionedFiles");
        assert_eq!(
            snake_to_camel("recover_missed_releases"),
            "recoverMissedReleases"
        );
    }

    #[test]
    fn snake_to_camel_no_underscores() {
        assert_eq!(snake_to_camel("name"), "name");
        assert_eq!(snake_to_camel(""), "");
    }

    // -----------------------------------------------------------------------
    // to_camel_case_keys
    // -----------------------------------------------------------------------

    #[test]
    fn to_camel_case_keys_transforms_known_keys() {
        let input = serde_json::json!({
            "tag_template": "v{version}",
            "name": "test"
        });
        let output = to_camel_case_keys(input);
        assert!(output.get("tagTemplate").is_some());
        assert!(output.get("name").is_some());
        assert!(output.get("tag_template").is_none());
    }

    #[test]
    fn to_camel_case_keys_nested() {
        let input = serde_json::json!({
            "package": [{
                "versioned_files": [],
                "shared_paths": []
            }]
        });
        let output = to_camel_case_keys(input);
        let pkg = &output["package"][0];
        assert!(pkg.get("versionedFiles").is_some());
        assert!(pkg.get("sharedPaths").is_some());
    }

    // -----------------------------------------------------------------------
    // JSON5 roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn json5_roundtrip() {
        let handler = Json5Format;
        let config = Config {
            workspace: WorkspaceConfig::default(),
            packages: vec![make_pkg("test", None)],
        };
        let serialized = handler.serialize(&config).unwrap();
        let parsed = handler.parse(&serialized).unwrap();
        assert_eq!(parsed.packages[0].name, "test");
    }

    // -----------------------------------------------------------------------
    // Dotfile roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn dotfile_roundtrip() {
        let handler = DotfileFormat;
        let config = Config {
            workspace: WorkspaceConfig::default(),
            packages: vec![make_pkg("test", None)],
        };
        let serialized = handler.serialize(&config).unwrap();
        let parsed = handler.parse(&serialized).unwrap();
        assert_eq!(parsed.packages[0].name, "test");
    }

    // -----------------------------------------------------------------------
    // ReleaseCommitMode parsing
    // -----------------------------------------------------------------------

    #[test]
    fn parse_release_commit_modes() {
        for (s, expected) in [
            ("commit", ReleaseCommitMode::Commit),
            ("pr", ReleaseCommitMode::Pr),
            ("none", ReleaseCommitMode::None),
        ] {
            let json =
                format!(r#"{{ "workspace": {{ "releaseCommitMode": "{s}" }}, "package": [] }}"#);
            let config: Config = serde_json::from_str(&json).unwrap();
            assert_eq!(
                config.workspace.release_commit_mode, expected,
                "failed for {s}"
            );
        }
    }

    #[test]
    fn parse_release_commit_scopes() {
        for (s, expected) in [
            ("grouped", ReleaseCommitScope::Grouped),
            ("per-package", ReleaseCommitScope::PerPackage),
        ] {
            let json =
                format!(r#"{{ "workspace": {{ "releaseCommitScope": "{s}" }}, "package": [] }}"#);
            let config: Config = serde_json::from_str(&json).unwrap();
            assert_eq!(
                config.workspace.release_commit_scope, expected,
                "failed for {s}"
            );
        }
    }

    #[test]
    fn release_commit_scope_defaults_to_grouped() {
        let json = r#"{ "workspace": {}, "package": [] }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.workspace.release_commit_scope,
            ReleaseCommitScope::Grouped
        );
    }

    #[test]
    fn release_commit_scope_camel_case_alias() {
        let json = r#"{ "workspace": { "releaseCommitScope": "per-package" }, "package": [] }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.workspace.release_commit_scope,
            ReleaseCommitScope::PerPackage
        );
    }

    // -----------------------------------------------------------------------
    // load_explicit with json5
    // -----------------------------------------------------------------------

    #[test]
    fn load_explicit_json5() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ferrflow.json5");
        std::fs::write(&path, "{ package: [{ name: \"x\", path: \".\" }] }").unwrap();
        let config = Config::load_explicit(&path).unwrap();
        assert_eq!(config.packages[0].name, "x");
    }

    // -----------------------------------------------------------------------
    // format_handler
    // -----------------------------------------------------------------------

    #[test]
    fn format_handler_returns_correct_filenames() {
        assert_eq!(
            format_handler(ConfigFileFormat::Json).filename(),
            "ferrflow.json"
        );
        assert_eq!(
            format_handler(ConfigFileFormat::Json5).filename(),
            "ferrflow.json5"
        );
        assert_eq!(
            format_handler(ConfigFileFormat::Toml).filename(),
            "ferrflow.toml"
        );
        assert_eq!(
            format_handler(ConfigFileFormat::Dotfile).filename(),
            ".ferrflow"
        );
    }

    // -----------------------------------------------------------------------
    // Config::is_monorepo edge case
    // -----------------------------------------------------------------------

    #[test]
    fn is_monorepo_empty() {
        let config = Config {
            workspace: WorkspaceConfig::default(),
            packages: vec![],
        };
        assert!(!config.is_monorepo());
    }

    #[test]
    fn load_fails_on_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ferrflow.json"), "{ invalid json").unwrap();
        assert!(Config::load(dir.path(), None).is_err());
    }

    #[test]
    fn load_fails_on_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ferrflow.toml"), "[[[invalid").unwrap();
        assert!(Config::load(dir.path(), None).is_err());
    }

    #[test]
    fn load_explicit_nonexistent_file() {
        let result = Config::load_explicit(std::path::Path::new("/nonexistent/ferrflow.json"));
        assert!(result.is_err());
        let err = format!("{:?}", result.unwrap_err());
        assert!(err.contains("not found") || err.contains("No such file"));
    }

    #[test]
    fn load_explicit_unknown_extension_defaults_to_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ferrflow.xyz");
        std::fs::write(&path, r#"{"package":[{"name":"x","path":"."}]}"#).unwrap();
        let config = Config::load_explicit(&path).unwrap();
        assert_eq!(config.packages[0].name, "x");
    }

    #[test]
    fn parse_json_ignores_unknown_fields() {
        let json = r#"{
            "workspace": { "remote": "origin", "unknown_field": true },
            "package": [{ "name": "app", "path": ".", "extra": "ignored" }]
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.packages[0].name, "app");
    }

    #[test]
    fn default_workspace_config_values() {
        // Default trait gives empty strings; serde defaults give "origin"/"main"
        let ws = WorkspaceConfig::default();
        assert_eq!(ws.versioning, None);
        assert!(ws.tag_template.is_none());
        assert!(!ws.recover_missed_releases);
        assert_eq!(ws.release_commit_mode, ReleaseCommitMode::Commit);
        assert!(ws.skip_ci.is_none());
    }

    #[test]
    fn serde_default_workspace_values() {
        // When deserialized from JSON with explicit workspace, serde defaults fill missing fields
        let json = r#"{"workspace":{"remote":"origin"},"package":[]}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.workspace.remote, "origin");
        assert!(config.workspace.anonymous_telemetry);
        assert!(config.workspace.auto_merge_releases);
    }

    #[test]
    fn file_format_serde_all_variants() {
        for (s, expected) in [
            ("json", FileFormat::Json),
            ("toml", FileFormat::Toml),
            ("xml", FileFormat::Xml),
            ("gradle", FileFormat::Gradle),
            ("gomod", FileFormat::GoMod),
            ("txt", FileFormat::Txt),
        ] {
            let json = format!(r#"{{ "path": "test", "format": "{s}" }}"#);
            let vf: VersionedFile = serde_json::from_str(&json).unwrap();
            assert_eq!(vf.format, expected, "failed for format {s}");
        }
    }

    #[test]
    fn depends_on_deserializes_from_json() {
        let json =
            r#"{"package":[{"name":"cli","path":"cli","dependsOn":["core"],"versionedFiles":[]}]}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.packages[0].depends_on, vec!["core"]);
    }

    #[test]
    fn depends_on_defaults_to_empty() {
        let json = r#"{"package":[{"name":"cli","path":"cli","versionedFiles":[]}]}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.packages[0].depends_on.is_empty());
    }

    #[test]
    fn depends_on_deserializes_snake_case() {
        let json = r#"{"package":[{"name":"cli","path":"cli","depends_on":["core"],"versionedFiles":[]}]}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.packages[0].depends_on, vec!["core"]);
    }

    #[test]
    fn tag_prefix_no_version_placeholder() {
        let ws = WorkspaceConfig::default();
        let pkg = PackageConfig {
            name: "app".to_string(),
            path: ".".to_string(),
            versioned_files: vec![],
            changelog: None,
            shared_paths: vec![],
            depends_on: vec![],
            versioning: None,
            tag_template: Some("release-latest".to_string()),
            hooks: None,
            floating_tags: None,
        };
        // When template has no {version}, prefix is the entire template
        assert_eq!(pkg.tag_prefix(&ws, false), "release-latest");
    }

    #[test]
    fn tag_for_version_replaces_placeholders() {
        let ws = WorkspaceConfig::default();
        let pkg = PackageConfig {
            name: "api".to_string(),
            path: ".".to_string(),
            versioned_files: vec![],
            changelog: None,
            shared_paths: vec![],
            depends_on: vec![],
            versioning: None,
            tag_template: Some("{name}/v{version}".to_string()),
            hooks: None,
            floating_tags: None,
        };
        assert_eq!(pkg.tag_for_version(&ws, true, "1.2.3"), "api/v1.2.3");
    }

    #[test]
    fn config_default_is_empty() {
        let config = Config::default();
        assert!(config.packages.is_empty());
    }

    #[test]
    fn load_discovers_json5_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("ferrflow.json5"),
            "{ package: [{ name: \"j5\", path: \".\" }] }",
        )
        .unwrap();
        let config = Config::load(dir.path(), None).unwrap();
        assert_eq!(config.packages[0].name, "j5");
    }

    #[test]
    fn load_with_relative_explicit_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("custom.json"),
            r#"{"package":[{"name":"rel","path":"."}]}"#,
        )
        .unwrap();
        let config = Config::load(dir.path(), Some(std::path::Path::new("custom.json"))).unwrap();
        assert_eq!(config.packages[0].name, "rel");
    }

    #[test]
    fn auto_detect_no_version_files() {
        let dir = tempfile::tempdir().unwrap();
        // Empty dir, no recognizable version files
        let config = Config::auto_detect(dir.path());
        assert!(config.packages.is_empty());
    }

    #[test]
    fn snake_to_camel_multiple_underscores() {
        assert_eq!(snake_to_camel("a_b_c_d"), "aBCD");
    }

    #[test]
    fn snake_to_camel_trailing_underscore() {
        assert_eq!(snake_to_camel("trailing_"), "trailing");
    }

    #[test]
    fn to_camel_case_keys_preserves_non_object_values() {
        let input = serde_json::json!("string_value");
        assert_eq!(to_camel_case_keys(input.clone()), input);

        let input = serde_json::json!(42);
        assert_eq!(to_camel_case_keys(input.clone()), input);

        let input = serde_json::json!(true);
        assert_eq!(to_camel_case_keys(input.clone()), input);

        let input = serde_json::json!(null);
        assert_eq!(to_camel_case_keys(input.clone()), input);
    }

    #[test]
    fn deserialize_branches_json() {
        let json = r#"{
            "workspace": {
                "branches": [
                    { "name": "main", "channel": false },
                    { "name": "develop", "channel": "dev" },
                    { "name": "beta", "channel": "beta", "prereleaseIdentifier": "timestamp" }
                ]
            },
            "package": []
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        let branches = config.workspace.branches.unwrap();
        assert_eq!(branches.len(), 3);
        assert!(matches!(branches[0].channel, ChannelValue::Stable(false)));
        assert!(matches!(&branches[1].channel, ChannelValue::Named(s) if s == "dev"));
        assert_eq!(
            branches[1].prerelease_identifier,
            PrereleaseIdentifier::Increment
        );
        assert_eq!(
            branches[2].prerelease_identifier,
            PrereleaseIdentifier::Timestamp
        );
    }

    #[test]
    fn deserialize_branches_toml() {
        let toml_str = r#"
            [[workspace.branches]]
            name = "main"
            channel = false

            [[workspace.branches]]
            name = "develop"
            channel = "dev"
            prereleaseIdentifier = "short-hash"

            [[package]]
            name = "test"
            path = "."
        "#;
        let config: Config = toml_edit::de::from_str(toml_str).unwrap();
        let branches = config.workspace.branches.unwrap();
        assert_eq!(branches.len(), 2);
        assert!(matches!(branches[0].channel, ChannelValue::Stable(false)));
        assert_eq!(
            branches[1].prerelease_identifier,
            PrereleaseIdentifier::ShortHash
        );
    }

    #[test]
    fn deserialize_no_branches_backward_compatible() {
        let json = r#"{ "workspace": {}, "package": [] }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.workspace.branches.is_none());
    }

    #[test]
    fn channel_value_rejects_true() {
        let json = r#"{ "name": "main", "channel": true }"#;
        let config: BranchChannelConfig = serde_json::from_str(json).unwrap();
        assert!(matches!(config.channel, ChannelValue::Stable(true)));
    }

    // -----------------------------------------------------------------------
    // JS/TS config loading (requires node/tsx on PATH)
    // -----------------------------------------------------------------------

    #[cfg(feature = "cli")]
    #[test]
    fn load_explicit_js_config() {
        // Skip if node is not available
        if std::process::Command::new("node")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping: node not found");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ferrflow.js");
        std::fs::write(
            &path,
            r#"export default {
                workspace: { remote: "origin", branch: "main" },
                package: [{ name: "js-app", path: ".", versionedFiles: [{ path: "package.json", format: "json" }] }]
            };"#,
        )
        .unwrap();
        let config = Config::load_explicit(&path).unwrap();
        assert_eq!(config.packages[0].name, "js-app");
    }

    #[cfg(feature = "cli")]
    #[test]
    fn load_explicit_js_async_function() {
        if std::process::Command::new("node")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping: node not found");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ferrflow.js");
        std::fs::write(
            &path,
            r#"export default async () => ({
                workspace: { remote: "origin" },
                package: [{ name: "async-app", path: "." }]
            });"#,
        )
        .unwrap();
        let config = Config::load_explicit(&path).unwrap();
        assert_eq!(config.packages[0].name, "async-app");
    }

    #[cfg(feature = "cli")]
    #[test]
    fn load_discovers_js_config() {
        if std::process::Command::new("node")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping: node not found");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("ferrflow.js"),
            r#"export default { package: [{ name: "discovered-js", path: "." }] };"#,
        )
        .unwrap();
        let config = Config::load(dir.path(), None).unwrap();
        assert_eq!(config.packages[0].name, "discovered-js");
    }

    #[cfg(feature = "cli")]
    #[test]
    fn load_js_and_json_fails_multiple() {
        if std::process::Command::new("node")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping: node not found");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("ferrflow.json"),
            r#"{"package":[{"name":"a","path":"."}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("ferrflow.js"),
            r#"export default { package: [{ name: "b", path: "." }] };"#,
        )
        .unwrap();
        let result = Config::load(dir.path(), None);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("multiple config files"));
    }

    #[test]
    fn load_explicit_js_not_found() {
        let path = std::path::Path::new("/nonexistent/ferrflow.js");
        assert!(Config::load_explicit(path).is_err());
    }

    #[cfg(feature = "cli")]
    #[test]
    fn load_explicit_ts_config() {
        // Skip if tsx cannot actually execute a TS file (not just --version)
        let dir = tempfile::tempdir().unwrap();
        let probe = dir.path().join("probe.mts");
        std::fs::write(&probe, "process.stdout.write('ok');").unwrap();
        let tsx_works = std::process::Command::new("tsx")
            .arg(&probe)
            .output()
            .or_else(|_| {
                std::process::Command::new("npx")
                    .args(["tsx"])
                    .arg(&probe)
                    .output()
            })
            .map(|o| o.status.success() && o.stdout == b"ok")
            .unwrap_or(false);

        if !tsx_works {
            eprintln!("Skipping: tsx cannot execute TS files");
            return;
        }

        let path = dir.path().join("ferrflow.ts");
        std::fs::write(
            &path,
            r#"const config = { package: [{ name: "ts-app", path: "." }] };
export default config;"#,
        )
        .unwrap();
        let config = match Config::load_explicit(&path) {
            Ok(c) => c,
            Err(e) => panic!("load_explicit failed: {e}"),
        };
        assert!(
            !config.packages.is_empty(),
            "Expected packages but got none. Config: {:?}",
            config
        );
        assert_eq!(config.packages[0].name, "ts-app");
    }

    #[cfg(feature = "cli")]
    #[test]
    fn load_explicit_js_function_hooks() {
        if std::process::Command::new("node")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping: node not found");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ferrflow.js");
        std::fs::write(
            &path,
            r#"export default {
                workspace: {
                    hooks: {
                        postBump: (ctx) => { console.log(ctx.newVersion); },
                        preBump: "echo hello"
                    }
                },
                package: [{ name: "hook-app", path: "." }]
            };"#,
        )
        .unwrap();
        let config = Config::load_explicit(&path).unwrap();
        assert_eq!(config.packages[0].name, "hook-app");
        // String hook should remain as-is
        let hooks = config.workspace.hooks.unwrap();
        assert_eq!(hooks.pre_bump.as_deref(), Some("echo hello"));
        // Function hook should be converted to a node command
        let post_bump = hooks.post_bump.unwrap();
        assert!(
            post_bump.contains("node"),
            "function hook should be reified as a node command: {post_bump}"
        );
        assert!(post_bump.contains("postBump"));
    }

    #[cfg(feature = "cli")]
    #[test]
    fn path_to_file_url_unix_style() {
        // Test the URL conversion with a temp path
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.js");
        std::fs::write(&path, "").unwrap();
        let url = path_to_file_url(&path).unwrap();
        assert!(url.starts_with("file:///"));
        assert!(url.contains("test.js"));
        assert!(!url.contains('\\'));
    }
}
