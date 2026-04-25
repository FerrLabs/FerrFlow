use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

    /// Auto mode. When `true`, FerrFlow re-runs project detection on every
    /// `release`/`check` and appends newly-discovered packages or
    /// versioned files to this config — never overwriting hand edits. New
    /// users start with `auto: true` (FerrFlow scaffolded the file), keep
    /// it for hands-off behaviour as the repo grows, or remove the flag
    /// to freeze the config in place.
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
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
        let mut packages: Vec<PackageConfig> = Vec::new();
        let mut covered: HashSet<PathBuf> = HashSet::new();

        // ── Workspace layouts: detect monorepos and emit one
        // PackageConfig per member. Each detector returns a list of
        // member paths *relative to root* — empty when this layout
        // doesn't apply. Members are deduplicated by absolute path so
        // overlapping layouts (e.g. a Cargo workspace that also has a
        // pnpm-workspace.yaml) don't double-count the same directory.
        let mut members: Vec<String> = Vec::new();
        members.extend(detect_cargo_workspace_members(root));
        members.extend(detect_maven_modules(root));
        members.extend(detect_pnpm_workspace_members(root));

        for member_rel in &members {
            let member_dir = root.join(member_rel);
            let canonical = std::fs::canonicalize(&member_dir).unwrap_or(member_dir.clone());
            if !covered.insert(canonical) {
                continue;
            }
            if let Some(pkg) = detect_single_package(&member_dir, member_rel) {
                packages.push(pkg);
            }
        }

        // ── Root-level package: emitted only when it carries its own
        // version (e.g. a single-repo Cargo project, or a Maven parent
        // pom that's also a real package). Skipped when the root is
        // purely a workspace umbrella (`[workspace]`-only Cargo.toml,
        // pom with `<packaging>pom</packaging>`).
        if !root_is_workspace_only(root) {
            let canonical = std::fs::canonicalize(root).unwrap_or(root.to_path_buf());
            if covered.insert(canonical)
                && let Some(pkg) = detect_single_package(root, ".")
            {
                packages.push(pkg);
            }
        }

        Config {
            workspace: WorkspaceConfig::default(),
            packages,
            auto: false,
        }
    }

    pub fn is_monorepo(&self) -> bool {
        self.packages.len() > 1
    }

    /// Public for tests in the same module — keeps the auto-detect
    /// helper functions free-floating (not on `Config`) so they don't
    /// pollute the public API.
    #[cfg(test)]
    pub fn auto_detect_for_test(root: &Path) -> Self {
        Self::auto_detect(root)
    }

    /// Like [`Self::load`] but with the **auto mode** lifecycle layered on
    /// top. Used by `release` and `check` so first-time runs scaffold a
    /// real config file instead of returning an in-memory detection
    /// result that silently disappears.
    ///
    /// Three branches, mirroring the table in
    /// [`FerrLabs/FerrFlow#388`](https://github.com/FerrLabs/FerrFlow/issues/388):
    ///
    /// 1. **Explicit path or any existing config**: load it. If the loaded
    ///    config has `auto: true`, run reconcile (append-only) and write
    ///    back when the detection finds new things.
    /// 2. **No config anywhere AND detection finds at least one version
    ///    file**: scaffold `.ferrflow/config.json` with `auto: true`.
    ///    Tell the user.
    /// 3. **No config and detection is empty**: behave like the old
    ///    [`Self::load`] no-op fallback (empty Config, nothing on disk).
    ///    Surfaces later as a clear "no packages configured" message.
    ///
    /// Side effects (writing files) only happen when there's something
    /// useful to write. Tests that want pure load-without-side-effects
    /// can keep using [`Self::load`].
    pub fn load_or_scaffold(repo_root: &Path, explicit_path: Option<&Path>) -> Result<Self> {
        if let Some(path) = explicit_path {
            // Explicit path bypasses auto mode entirely.
            return Self::load(repo_root, Some(path));
        }

        // 1. Legacy discovery first. If there's a hand-written
        //    `ferrflow.json` (or any other root-level config), load it
        //    as-is — auto mode never kicks in for users who already have
        //    a config on disk. `.ferrflow/config.json` is intentionally
        //    NOT searched here: it's the auto-mode slot, not a regular
        //    discovery path.
        if let Some(legacy) = Self::find_legacy_config(repo_root)? {
            return Self::load_from_path(&legacy);
        }

        // 2. No legacy config — check the auto-mode slot.
        let scaffold_path = repo_root.join(".ferrflow").join("config.json");
        if scaffold_path.is_file() {
            let mut config = Self::load_from_path(&scaffold_path)?;
            if config.auto {
                let detected = Self::auto_detect(repo_root);
                let added = config.reconcile_with(&detected);
                if added > 0 {
                    config.write_to_path(&scaffold_path)?;
                    eprintln!(
                        "auto mode: added {added} new entr{} to {} after re-detection",
                        if added == 1 { "y" } else { "ies" },
                        scaffold_path
                            .strip_prefix(repo_root)
                            .unwrap_or(&scaffold_path)
                            .display(),
                    );
                }
            }
            return Ok(config);
        }

        // 3. Nothing on disk — try detection.
        let mut config = Self::auto_detect(repo_root);
        if config.packages.is_empty() {
            // Nothing to write. Caller will surface the "no packages
            // configured" message.
            return Ok(config);
        }

        config.auto = true;

        config.write_to_path(&scaffold_path)?;
        eprintln!(
            "no config found — running in auto mode (scaffolded {})",
            scaffold_path
                .strip_prefix(repo_root)
                .unwrap_or(&scaffold_path)
                .display(),
        );
        Ok(config)
    }

    /// Look for a hand-written config at any of the legacy root-level
    /// locations (`ferrflow.json`, `ferrflow.toml`, …, `.ferrflow`). Does
    /// **not** look inside `.ferrflow/config.json` — that path is reserved
    /// for the auto-mode scaffold and only consulted by
    /// [`Self::load_or_scaffold`].
    fn find_legacy_config(repo_root: &Path) -> Result<Option<PathBuf>> {
        let mut search: Vec<String> = CONFIG_FORMATS
            .iter()
            .map(|h| h.filename().to_string())
            .collect();
        #[cfg(feature = "cli")]
        {
            let dotfile_pos = search.len() - 1;
            search.insert(dotfile_pos, TS_CONFIG_FILENAME.to_string());
            search.insert(dotfile_pos + 1, JS_CONFIG_FILENAME.to_string());
        }

        let mut found: Vec<PathBuf> = Vec::new();
        for filename in &search {
            let path = repo_root.join(filename);
            if path.exists() && path.is_file() {
                found.push(path);
            }
        }

        match found.len() {
            0 => Ok(None),
            1 => Ok(Some(found.remove(0))),
            _ => {
                let names: Vec<String> = found
                    .iter()
                    .filter_map(|p| {
                        p.strip_prefix(repo_root)
                            .ok()
                            .map(|p| p.to_string_lossy().to_string())
                    })
                    .collect();
                Err(anyhow::anyhow!(
                    "multiple config files found: {}\nUse --config <path> to specify which one to use.",
                    names.join(", ")
                ))
                .error_code(error_code::CONFIG_MULTIPLE_FILES)
            }
        }
    }

    /// Append-only merge of `detected` into `self`. Returns the number of
    /// new entries added. Used in auto-mode reconcile so that adding a
    /// new module to a monorepo gets picked up on the next run without
    /// touching anything the user has hand-edited.
    ///
    /// Rules:
    /// - Packages keyed by `path` first, then by `name` (so renaming a
    ///   package to "frontend" while keeping `path: "."` doesn't make
    ///   detection see it as a brand-new package). New packages are
    ///   appended.
    /// - For existing packages, `versioned_files` keyed by `path`: new
    ///   entries appended, existing entries left intact (so a user-set
    ///   `selector` or a renamed `format` stays).
    /// - `workspace`, `auto`, and any other top-level fields are never
    ///   touched.
    pub fn reconcile_with(&mut self, detected: &Self) -> usize {
        let mut added = 0usize;
        for det_pkg in &detected.packages {
            // Match first by path (strong signal of "same logical
            // package"), then fall back to name.
            let existing_idx = self
                .packages
                .iter()
                .position(|p| p.path == det_pkg.path)
                .or_else(|| self.packages.iter().position(|p| p.name == det_pkg.name));

            match existing_idx {
                Some(idx) => {
                    let existing = &mut self.packages[idx];
                    let known_paths: HashSet<String> = existing
                        .versioned_files
                        .iter()
                        .map(|f| f.path.clone())
                        .collect();
                    for vf in &det_pkg.versioned_files {
                        if !known_paths.contains(&vf.path) {
                            existing.versioned_files.push(vf.clone());
                            added += 1;
                        }
                    }
                }
                None => {
                    self.packages.push(det_pkg.clone());
                    added += 1;
                }
            }
        }
        added
    }

    /// Serialize this config as pretty JSON and write it to `path`,
    /// creating any missing parent directories. Used by the auto-mode
    /// scaffold and reconcile paths.
    fn write_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        let serialized =
            serde_json::to_string_pretty(self).context("failed to serialize generated config")?;
        std::fs::write(path, format!("{serialized}\n"))
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
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
        auto: false,
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

// ---------------------------------------------------------------------------
// Auto-detect helpers (used by Config::auto_detect)
// ---------------------------------------------------------------------------

/// Detect a single package at `dir`, given its relative path from the
/// repo root. Returns `None` when no version-bearing files are found.
///
/// Naming: the package gets the directory's basename, with `.` falling
/// back to the repo root's name. The changelog defaults to a sibling
/// `CHANGELOG.md` so single-repo projects keep their existing layout.
fn detect_single_package(dir: &Path, relative: &str) -> Option<PackageConfig> {
    let mut versioned_files: Vec<VersionedFile> = Vec::new();

    let mut push = |path: &str, format: FileFormat| {
        let full = if relative == "." {
            path.to_string()
        } else {
            format!("{relative}/{path}")
        };
        versioned_files.push(VersionedFile {
            path: full,
            format,
            selector: None,
        });
    };

    if dir.join("Cargo.toml").is_file() {
        // Workspace-only Cargo.toml ([workspace] without [package]) is
        // handled by the caller via `root_is_workspace_only`. At this
        // level we still emit it: detect_single_package is only invoked
        // for package directories, never for the workspace umbrella.
        push("Cargo.toml", FileFormat::Toml);
    }
    if dir.join("build.gradle.kts").is_file() {
        push("build.gradle.kts", FileFormat::Gradle);
    } else if dir.join("build.gradle").is_file() {
        push("build.gradle", FileFormat::Gradle);
    }
    if dir.join("Chart.yaml").is_file() {
        push("Chart.yaml", FileFormat::Helm);
    }
    if dir.join("go.mod").is_file() {
        push("go.mod", FileFormat::GoMod);
    }
    if dir.join("package.json").is_file() {
        push("package.json", FileFormat::Json);
    }
    if dir.join("pom.xml").is_file() {
        push("pom.xml", FileFormat::Xml);
    }
    if dir.join("pyproject.toml").is_file() {
        push("pyproject.toml", FileFormat::Toml);
    }
    for name in &["VERSION", "VERSION.txt"] {
        if dir.join(name).is_file() {
            push(name, FileFormat::Txt);
            break;
        }
    }

    if versioned_files.is_empty() {
        return None;
    }

    // Package name: basename of the directory (or repo root for ".").
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();

    // Changelog: per-package CHANGELOG.md so each member tracks its own
    // history. Single-repo (`.`) keeps the conventional repo-root path.
    let changelog = if relative == "." {
        Some("CHANGELOG.md".to_string())
    } else {
        Some(format!("{relative}/CHANGELOG.md"))
    };

    Some(PackageConfig {
        name,
        path: relative.to_string(),
        versioned_files,
        changelog,
        shared_paths: Vec::new(),
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        hooks: None,
        floating_tags: None,
    })
}

/// True when the repo root is a workspace umbrella that doesn't itself
/// declare a package version. Currently recognises:
/// - Cargo `[workspace]` block in `Cargo.toml` with no `[package]`.
/// - Maven pom with `<packaging>pom</packaging>` and at least one
///   `<modules><module>`.
fn root_is_workspace_only(root: &Path) -> bool {
    if let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml")) {
        let has_workspace = text.contains("[workspace]");
        let has_package = text.contains("[package]");
        if has_workspace && !has_package {
            return true;
        }
    }
    if let Ok(text) = std::fs::read_to_string(root.join("pom.xml"))
        && text.contains("<packaging>pom</packaging>")
        && text.contains("<modules>")
    {
        return true;
    }
    false
}

/// Parse `Cargo.toml`'s `[workspace] members = [...]` and return the
/// list of member paths *relative to root*. Glob entries like
/// `"packages/*"` are expanded by listing the parent directory and
/// keeping each subdirectory that contains a `Cargo.toml`.
///
/// Empty Vec when the repo isn't a Cargo workspace.
fn detect_cargo_workspace_members(root: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml")) else {
        return Vec::new();
    };
    let Ok(doc) = text.parse::<toml_edit::DocumentMut>() else {
        return Vec::new();
    };
    let Some(members) = doc
        .get("workspace")
        .and_then(|w| w.as_table())
        .and_then(|t| t.get("members"))
        .and_then(|m| m.as_array())
    else {
        return Vec::new();
    };

    let mut out: Vec<String> = Vec::new();
    for entry in members.iter() {
        let Some(s) = entry.as_str() else { continue };
        out.extend(expand_glob_one_level(root, s, |dir| {
            dir.join("Cargo.toml").is_file()
        }));
    }
    out
}

/// Parse the root `pom.xml` for `<modules><module>NAME</module></modules>`
/// and return each module name as a relative path. Reuses the depth-aware
/// XML walker shipped for the version-file selector feature.
fn detect_maven_modules(root: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(root.join("pom.xml")) else {
        return Vec::new();
    };

    // Quick literal scan: we only care about <module>NAME</module> entries
    // inside a top-level <modules> block. A regex would be enough but we
    // already have the depth-aware walker on the format side; keep this
    // file dep-light by doing a small string parse.
    let mut out: Vec<String> = Vec::new();
    if let Some(modules_start) = text.find("<modules>") {
        let after = &text[modules_start + "<modules>".len()..];
        if let Some(modules_end) = after.find("</modules>") {
            let block = &after[..modules_end];
            for line in block.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("<module>")
                    && let Some(name) = rest.strip_suffix("</module>")
                {
                    let name = name.trim();
                    if !name.is_empty() {
                        out.push(name.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Parse `pnpm-workspace.yaml` for the `packages:` list and expand any
/// `path/*` patterns to actual subdirectories that contain a
/// `package.json`. Hand-rolled (no YAML dep) — accepts the common
/// formatting only:
///
/// ```yaml
/// packages:
///   - 'apps/*'
///   - 'packages/foo'
/// ```
///
/// Lines starting with `!` (negative patterns) are honoured: matching
/// directories get filtered out from the final list.
fn detect_pnpm_workspace_members(root: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(root.join("pnpm-workspace.yaml")) else {
        return Vec::new();
    };

    let mut in_packages = false;
    let mut patterns: Vec<String> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if !in_packages {
            if line.starts_with("packages:") {
                in_packages = true;
            }
            continue;
        }
        // List item — must start with whitespace + `-`.
        if let Some(rest) = line.trim_start().strip_prefix("- ") {
            let value = rest.trim().trim_matches(|c: char| c == '"' || c == '\'');
            patterns.push(value.to_string());
        } else if !line.starts_with(' ') && !line.starts_with('\t') {
            // Top-level key reached — end of the packages list.
            break;
        }
    }

    let mut positives: Vec<String> = Vec::new();
    let mut negatives: Vec<String> = Vec::new();
    for p in patterns {
        if let Some(neg) = p.strip_prefix('!') {
            negatives.push(neg.to_string());
        } else {
            positives.push(p);
        }
    }

    let mut expanded: Vec<String> = Vec::new();
    for pat in &positives {
        expanded.extend(expand_glob_one_level(root, pat, |dir| {
            dir.join("package.json").is_file()
        }));
    }

    // Filter negatives: drop any expanded entry that matches a negative
    // pattern as a literal path.
    expanded.retain(|m| {
        !negatives.iter().any(|neg| {
            // Negative is literal or `name/*` style; expand with a
            // permissive predicate (anything is a match) and check
            // membership.
            let candidates = expand_glob_one_level(root, neg, |_| true);
            candidates.iter().any(|c| c == m)
        })
    });
    expanded
}

/// Expand a one-level glob pattern (no `**`) like `packages/*` to
/// concrete subdirectory paths under `root`. The optional `keep`
/// predicate filters the matches — typically by checking for a
/// signature file (Cargo.toml, package.json, …) in the candidate dir.
///
/// Patterns without `*` are returned as-is when the directory exists.
fn expand_glob_one_level(root: &Path, pattern: &str, keep: impl Fn(&Path) -> bool) -> Vec<String> {
    if !pattern.contains('*') {
        let candidate = root.join(pattern);
        if candidate.is_dir() && keep(&candidate) {
            return vec![pattern.to_string()];
        }
        return Vec::new();
    }

    // Only `path/*` form supported. Anything else (e.g. `**`, multi-level
    // globs) is silently skipped — users with exotic layouts can pin
    // members manually.
    let Some((parent_rel, last)) = pattern.rsplit_once('/') else {
        return Vec::new();
    };
    if last != "*" {
        return Vec::new();
    }
    let parent_dir = root.join(parent_rel);
    let Ok(entries) = std::fs::read_dir(&parent_dir) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !keep(&path) {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            out.push(format!("{parent_rel}/{name}"));
        }
    }
    // Stable order across filesystems — deterministic config diffs.
    out.sort();
    out
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
            auto: false,
        };
        assert!(!config.is_monorepo());
    }

    #[test]
    fn is_monorepo_multi() {
        let config = Config {
            workspace: WorkspaceConfig::default(),
            packages: vec![make_pkg("a", None), make_pkg("b", None)],
            auto: false,
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
            auto: false,
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
            auto: false,
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
            auto: false,
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
            auto: false,
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
            auto: false,
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
            auto: false,
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
            auto: false,
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

    // ── Auto mode (#388) ────────────────────────────────────────────────

    #[test]
    fn load_or_scaffold_writes_config_when_absent_and_detection_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"foo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let config = Config::load_or_scaffold(dir.path(), None).unwrap();
        assert!(config.auto, "scaffolded config must have auto: true");
        assert_eq!(config.packages.len(), 1);

        let scaffold = dir.path().join(".ferrflow").join("config.json");
        assert!(scaffold.exists(), "config.json should have been written");

        // The persisted JSON must round-trip with auto: true so the next
        // run re-runs detection (instead of treating the scaffold as a
        // frozen hand-written config).
        let body = std::fs::read_to_string(&scaffold).unwrap();
        assert!(body.contains("\"auto\": true"));
    }

    #[test]
    fn load_or_scaffold_no_op_when_detection_finds_nothing() {
        let dir = tempfile::tempdir().unwrap();
        // Empty dir — nothing detectable.
        let config = Config::load_or_scaffold(dir.path(), None).unwrap();
        assert!(config.packages.is_empty());
        assert!(!dir.path().join(".ferrflow").exists());
    }

    #[test]
    fn load_or_scaffold_picks_up_existing_scaffold_on_second_run() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"foo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        // First run scaffolds.
        let _ = Config::load_or_scaffold(dir.path(), None).unwrap();
        let scaffold = dir.path().join(".ferrflow").join("config.json");
        let mtime1 = std::fs::metadata(&scaffold).unwrap().modified().unwrap();

        // Second run loads the same file — no rewrite expected because
        // detection finds the same single Cargo.toml that's already in
        // the config, so nothing to add.
        std::thread::sleep(std::time::Duration::from_millis(50));
        let config = Config::load_or_scaffold(dir.path(), None).unwrap();
        assert!(config.auto);
        let mtime2 = std::fs::metadata(&scaffold).unwrap().modified().unwrap();
        assert_eq!(mtime1, mtime2, "no-op reconcile must not touch the file");
    }

    #[test]
    fn load_or_scaffold_appends_new_package_in_auto_mode() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"foo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        // First run scaffolds with one package.
        let first = Config::load_or_scaffold(dir.path(), None).unwrap();
        assert_eq!(first.packages.len(), 1);

        // User adds a package.json — second run reconciles in append-only
        // mode and persists.
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"bar","version":"0.1.0"}"#,
        )
        .unwrap();
        let second = Config::load_or_scaffold(dir.path(), None).unwrap();
        // Same single package (named after the dir) but two versioned
        // files now.
        assert_eq!(second.packages.len(), 1);
        let pkg = &second.packages[0];
        assert!(pkg.versioned_files.iter().any(|f| f.path == "Cargo.toml"));
        assert!(pkg.versioned_files.iter().any(|f| f.path == "package.json"));
    }

    #[test]
    fn load_or_scaffold_does_not_overwrite_user_edits() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".ferrflow")).unwrap();
        // Hand-written-ish config: auto: true, but custom package name
        // and a custom selector on Cargo.toml.
        std::fs::write(
            dir.path().join(".ferrflow").join("config.json"),
            r#"{
  "auto": true,
  "package": [
    {
      "name": "user-named",
      "path": ".",
      "versioned_files": [
        { "path": "Cargo.toml", "format": "toml", "selector": "/foo/bar" }
      ],
      "changelog": "CHANGELOG.md"
    }
  ]
}
"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"foo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"bar","version":"0.1.0"}"#,
        )
        .unwrap();

        let config = Config::load_or_scaffold(dir.path(), None).unwrap();
        assert_eq!(config.packages.len(), 1);
        let pkg = &config.packages[0];
        assert_eq!(pkg.name, "user-named", "user-set name must survive");
        // Cargo.toml entry: existing one kept (with selector), NOT
        // re-detected as a fresh entry.
        let cargo = pkg
            .versioned_files
            .iter()
            .find(|f| f.path == "Cargo.toml")
            .unwrap();
        assert_eq!(cargo.selector.as_deref(), Some("/foo/bar"));
        // New file (package.json) was appended.
        assert!(pkg.versioned_files.iter().any(|f| f.path == "package.json"));
    }

    #[test]
    fn reconcile_with_appends_new_packages_only() {
        let mut existing = Config::default();
        existing.packages.push(PackageConfig {
            name: "alpha".into(),
            path: ".".into(),
            versioned_files: vec![VersionedFile {
                path: "a.json".into(),
                format: FileFormat::Json,
                selector: None,
            }],
            changelog: None,
            shared_paths: vec![],
            depends_on: vec![],
            versioning: None,
            tag_template: None,
            hooks: None,
            floating_tags: None,
        });

        let mut detected = Config::default();
        detected.packages.push(PackageConfig {
            name: "alpha".into(),
            path: ".".into(),
            versioned_files: vec![
                // Same path — should be skipped.
                VersionedFile {
                    path: "a.json".into(),
                    format: FileFormat::Json,
                    selector: None,
                },
                // New path — should be appended.
                VersionedFile {
                    path: "b.toml".into(),
                    format: FileFormat::Toml,
                    selector: None,
                },
            ],
            changelog: None,
            shared_paths: vec![],
            depends_on: vec![],
            versioning: None,
            tag_template: None,
            hooks: None,
            floating_tags: None,
        });
        detected.packages.push(PackageConfig {
            // Distinct path so it doesn't collide with "alpha" — that
            // would be the case for a real monorepo gaining a module.
            name: "beta".into(),
            path: "packages/beta".into(),
            versioned_files: vec![],
            changelog: None,
            shared_paths: vec![],
            depends_on: vec![],
            versioning: None,
            tag_template: None,
            hooks: None,
            floating_tags: None,
        });

        let added = existing.reconcile_with(&detected);
        assert_eq!(added, 2, "1 new vf + 1 new pkg");
        assert_eq!(existing.packages.len(), 2);
        assert_eq!(existing.packages[0].versioned_files.len(), 2);
    }

    /// `.ferrflow/config.json` is the auto-mode slot, not a regular
    /// discovery path. If a legacy `ferrflow.json` exists at the root,
    /// it must win — and `.ferrflow/config.json` is *not* even consulted.
    /// This guards against the path being treated as a generic
    /// "alternative config location".
    #[test]
    fn load_or_scaffold_legacy_config_wins_over_scaffold_path() {
        let dir = tempfile::tempdir().unwrap();
        // A "legacy" hand-written ferrflow.json with one named package.
        std::fs::write(
            dir.path().join("ferrflow.json"),
            r#"{
  "package": [
    { "name": "legacy-app", "path": ".", "versioned_files": [] }
  ]
}
"#,
        )
        .unwrap();
        // And a stale scaffold with completely different content.
        std::fs::create_dir_all(dir.path().join(".ferrflow")).unwrap();
        std::fs::write(
            dir.path().join(".ferrflow").join("config.json"),
            r#"{
  "auto": true,
  "package": [
    { "name": "scaffold-app", "path": ".", "versioned_files": [] }
  ]
}
"#,
        )
        .unwrap();

        let config = Config::load_or_scaffold(dir.path(), None).unwrap();
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.packages[0].name, "legacy-app");
        // Auto stays false — we loaded the legacy config, not the scaffold.
        assert!(!config.auto);
    }

    // ── Monorepo detection ──────────────────────────────────────────────

    #[test]
    fn auto_detect_cargo_workspace_with_glob_members() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*", "tools/cli"]
"#,
        )
        .unwrap();
        for member in ["crates/api", "crates/web", "tools/cli"] {
            let path = dir.path().join(member);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(
                path.join("Cargo.toml"),
                "[package]\nname=\"x\"\nversion = \"0.1.0\"\n",
            )
            .unwrap();
        }
        // A directory under crates/ without Cargo.toml — must be skipped
        // by the glob expansion.
        std::fs::create_dir_all(dir.path().join("crates").join("notes")).unwrap();

        let config = Config::auto_detect_for_test(dir.path());
        let names: Vec<&str> = config.packages.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"api"));
        assert!(names.contains(&"web"));
        assert!(names.contains(&"cli"));
        // Workspace umbrella isn't itself a package.
        assert!(
            !config.packages.iter().any(|p| p.path == "."),
            "[workspace]-only Cargo.toml must not become a package"
        );
        assert_eq!(config.packages.len(), 3);
        // Per-package versioned_files use the correct relative path.
        let api = config.packages.iter().find(|p| p.name == "api").unwrap();
        assert_eq!(api.path, "crates/api");
        assert_eq!(
            api.versioned_files[0].path, "crates/api/Cargo.toml",
            "versioned file path must be relative to repo root"
        );
        assert_eq!(api.changelog.as_deref(), Some("crates/api/CHANGELOG.md"));
    }

    #[test]
    fn auto_detect_maven_multi_module() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>parent</artifactId>
    <version>1.0.0</version>
    <packaging>pom</packaging>
    <modules>
        <module>common</module>
        <module>rest-api</module>
    </modules>
</project>
"#,
        )
        .unwrap();
        for m in ["common", "rest-api"] {
            let p = dir.path().join(m);
            std::fs::create_dir_all(&p).unwrap();
            std::fs::write(
                p.join("pom.xml"),
                r#"<project><artifactId>x</artifactId><version>1.0.0</version></project>"#,
            )
            .unwrap();
        }

        let config = Config::auto_detect_for_test(dir.path());
        assert_eq!(config.packages.len(), 2, "no umbrella entry expected");
        let names: Vec<&str> = config.packages.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"common"));
        assert!(names.contains(&"rest-api"));
    }

    #[test]
    fn auto_detect_pnpm_workspace_expands_globs_and_negatives() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'apps/*'\n  - 'packages/shared'\n  - '!apps/legacy'\n",
        )
        .unwrap();
        for member in ["apps/web", "apps/admin", "apps/legacy", "packages/shared"] {
            let p = dir.path().join(member);
            std::fs::create_dir_all(&p).unwrap();
            std::fs::write(p.join("package.json"), r#"{"name":"x","version":"0.1.0"}"#).unwrap();
        }

        let config = Config::auto_detect_for_test(dir.path());
        let paths: Vec<&str> = config.packages.iter().map(|p| p.path.as_str()).collect();
        assert!(paths.contains(&"apps/web"));
        assert!(paths.contains(&"apps/admin"));
        assert!(paths.contains(&"packages/shared"));
        assert!(
            !paths.contains(&"apps/legacy"),
            "negative pattern must drop apps/legacy"
        );
    }

    #[test]
    fn auto_detect_falls_back_to_single_package_for_non_workspace_repo() {
        // No Cargo workspace, no Maven modules, no pnpm-workspace —
        // behaves exactly like the pre-monorepo path: one package at
        // root.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"foo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let config = Config::auto_detect_for_test(dir.path());
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.packages[0].path, ".");
    }

    #[test]
    fn auto_detect_single_repo_keeps_root_changelog() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"foo","version":"0.1.0"}"#,
        )
        .unwrap();
        let config = Config::auto_detect_for_test(dir.path());
        assert_eq!(config.packages.len(), 1);
        assert_eq!(
            config.packages[0].changelog.as_deref(),
            Some("CHANGELOG.md")
        );
    }

    #[test]
    fn auto_detect_workspace_only_root_does_not_become_package() {
        // Pure umbrella: Cargo.toml has [workspace] but no [package].
        // Detection must not emit a root-level package entry even if
        // there are other root files (none here).
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\"]\n",
        )
        .unwrap();
        let path_a = dir.path().join("a");
        std::fs::create_dir_all(&path_a).unwrap();
        std::fs::write(
            path_a.join("Cargo.toml"),
            "[package]\nname=\"a\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let config = Config::auto_detect_for_test(dir.path());
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.packages[0].path, "a");
    }

    #[test]
    fn auto_field_does_not_serialise_when_false() {
        let cfg = Config::default();
        let s = serde_json::to_string(&cfg).unwrap();
        // Match the JSON key boundary precisely — naive substring matches
        // the unrelated `auto_merge_releases` key on the workspace.
        assert!(
            !s.contains("\"auto\":"),
            "default Config should hide top-level auto field, got: {s}"
        );
    }
}
