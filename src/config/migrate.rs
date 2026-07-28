use anyhow::Result;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::error_code::{self, ErrorCodeExt};

use super::Config;
use super::format::{CONFIG_FORMATS, ConfigFileFormat, format_handler};
use super::loader_js::{JS_CONFIG_FILENAME, TS_CONFIG_FILENAME};
use super::package::{FileFormat, PackageConfig, VersionedFile};
use super::types::{
    BranchChannelConfig, ChannelValue, ForgeKind, HooksConfig, PrereleaseIdentifier,
};
use super::workspace::WorkspaceConfig;

mod changesets;
mod release_please;
mod standard_version;
mod workspace_packages;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    SemanticRelease,
    Changesets,
    ReleasePlease,
    StandardVersion,
}

impl Source {
    fn label(self) -> &'static str {
        match self {
            Source::SemanticRelease => "semantic-release",
            Source::Changesets => "changesets",
            Source::ReleasePlease => "release-please",
            Source::StandardVersion => "standard-version",
        }
    }
}

/// semantic-release config filenames (cosmiconfig for the `release` key), in
/// preference order. JSON/JSON5 is read directly; `.js/.cjs/.mjs` is evaluated
/// with node; `.yaml/.yml` is parsed as YAML — see [`read_source_as_json`].
const SEMANTIC_RELEASE_FILES: &[&str] = &[
    ".releaserc",
    ".releaserc.json",
    ".releaserc.yaml",
    ".releaserc.yml",
    ".releaserc.js",
    ".releaserc.cjs",
    ".releaserc.mjs",
    "release.config.js",
    "release.config.cjs",
    "release.config.mjs",
];

pub fn migrate(from: Option<Source>) -> Result<()> {
    ensure_no_existing_config()?;

    let source = match from {
        Some(s) => s,
        None => detect_source()?,
    };

    match source {
        Source::SemanticRelease => migrate_semantic_release(),
        Source::Changesets => changesets::run(),
        Source::ReleasePlease => release_please::run(),
        Source::StandardVersion => standard_version::run(),
    }
}

fn ensure_no_existing_config() -> Result<()> {
    for handler in CONFIG_FORMATS {
        if Path::new(handler.filename()).exists() {
            return Err(anyhow::anyhow!("{} already exists", handler.filename()))
                .error_code(error_code::CONFIG_ALREADY_EXISTS);
        }
    }
    for filename in [TS_CONFIG_FILENAME, JS_CONFIG_FILENAME] {
        if Path::new(filename).exists() {
            return Err(anyhow::anyhow!("{filename} already exists"))
                .error_code(error_code::CONFIG_ALREADY_EXISTS);
        }
    }
    Ok(())
}

fn detect_source() -> Result<Source> {
    if find_semantic_release_config().is_some() {
        return Ok(Source::SemanticRelease);
    }
    if changesets::detect().is_some() {
        return Ok(Source::Changesets);
    }
    if release_please::detect().is_some() {
        return Ok(Source::ReleasePlease);
    }
    if standard_version::detect().is_some() {
        return Ok(Source::StandardVersion);
    }
    Err(anyhow::anyhow!(
        "no supported release-tool config found. Looked for {}, {}, {}, {}. \
         Pass --from to force a source.",
        SEMANTIC_RELEASE_FILES.join(", "),
        changesets::CONFIG_FILE,
        release_please::CONFIG_FILE,
        standard_version::CONFIG_FILES.join(", "),
    ))
    .error_code(error_code::CONFIG_NOT_FOUND)
}

fn find_semantic_release_config() -> Option<PathBuf> {
    SEMANTIC_RELEASE_FILES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

/// Read a release-tool config file and return a JSON string the JSON
/// converters can parse. `.js/.cjs/.mjs` is evaluated with node, `.yaml/.yml`
/// is parsed as YAML, and everything else (`.json`, `.json5`, extensionless)
/// is handed straight to the json5-tolerant parsers.
pub(super) fn read_source_as_json(path: &Path) -> Result<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "js" | "cjs" | "mjs" => eval_js_to_json(path),
        "yaml" | "yml" => yaml_to_json(&read_file(path)?),
        _ => read_file(path),
    }
}

fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("could not read {}: {e}", path.display()))
        .error_code(error_code::CONFIG_NOT_FOUND)
}

/// Evaluate a JS/TS release config to JSON by importing it with node and
/// printing its resolved default export. Same trust model as evaluating a
/// `ferrflow.js` config — it's the user's own repo, run locally.
fn eval_js_to_json(path: &Path) -> Result<String> {
    let file_url = super::loader_js::path_to_file_url(path)?;
    let script = format!(
        "const m = await import('{file_url}'); \
         const cfg = m.default ?? m; \
         const resolved = typeof cfg === 'function' ? await cfg() : cfg; \
         process.stdout.write(JSON.stringify(resolved));"
    );
    let mut cmd = std::process::Command::new("node");
    cmd.args(["--input-type=module", "-e", &script]);
    if let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "migrating a JavaScript config requires Node.js, but 'node' was not found in \
                     PATH. Install Node.js from https://nodejs.org/, or convert the config to JSON."
                )
            } else {
                anyhow::anyhow!("failed to execute node: {e}")
            }
        })
        .error_code(error_code::CONFIG_EVAL_NODE)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "could not evaluate {}:\n{}",
            path.display(),
            stderr.trim()
        ))
        .error_code(error_code::CONFIG_EVAL_FAILED);
    }
    String::from_utf8(output.stdout)
        .map_err(|_| anyhow::anyhow!("{} produced invalid UTF-8", path.display()))
        .error_code(error_code::CONFIG_INVALID_OUTPUT)
}

/// Convert a YAML config to a JSON string so the JSON converters can consume it.
pub(super) fn yaml_to_json(raw: &str) -> Result<String> {
    let value: serde_json::Value = serde_norway::from_str(raw)
        .map_err(|e| anyhow::anyhow!("could not parse YAML config: {e}"))
        .error_code(error_code::CONFIG_INVALID_JSON)?;
    serde_json::to_string(&value)
        .map_err(|e| anyhow::anyhow!("could not re-serialize YAML to JSON: {e}"))
        .error_code(error_code::CONFIG_INVALID_JSON)
}

#[derive(Debug, Deserialize, Default)]
struct ReleaseRc {
    #[serde(default, rename = "tagFormat")]
    tag_format: Option<String>,
    #[serde(default)]
    branches: Option<Branches>,
    #[serde(default, rename = "repositoryUrl")]
    repository_url: Option<String>,
    #[serde(default)]
    plugins: Vec<PluginEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Branches {
    One(BranchSpec),
    Many(Vec<BranchSpec>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BranchSpec {
    Name(String),
    Object {
        name: String,
        #[serde(default)]
        prerelease: Option<PrereleaseFlag>,
        #[serde(default)]
        channel: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PrereleaseFlag {
    Bool(bool),
    Name(String),
}

/// A plugin is either `"name"` or `["name", { options }]`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PluginEntry {
    Name(String),
    WithOptions(String, serde_json::Value),
}

impl PluginEntry {
    fn name(&self) -> &str {
        match self {
            PluginEntry::Name(n) => n,
            PluginEntry::WithOptions(n, _) => n,
        }
    }

    fn options(&self) -> Option<&serde_json::Value> {
        match self {
            PluginEntry::Name(_) => None,
            PluginEntry::WithOptions(_, o) => Some(o),
        }
    }
}

/// Everything the converter wants to say to the user: what it mapped, what it
/// dropped, and what needs a human. Separated from the write so it can be
/// asserted in tests without touching the filesystem.
#[derive(Debug, Default, PartialEq)]
pub struct MigrationReport {
    pub mapped: Vec<String>,
    pub warnings: Vec<String>,
    pub ignored: Vec<String>,
}

pub fn build_config_from_releaserc(raw: &str) -> Result<(Config, MigrationReport)> {
    let rc: ReleaseRc = json5::from_str(raw)
        .map_err(|e| anyhow::anyhow!("could not parse .releaserc as JSON: {e}"))
        .error_code(error_code::CONFIG_INVALID_JSON)?;

    let mut report = MigrationReport::default();
    let mut workspace = WorkspaceConfig::default();

    if let Some(tag_format) = &rc.tag_format {
        workspace.tag_template = Some(convert_tag_format(tag_format));
        report
            .mapped
            .push(format!("tagFormat → tagTemplate ({})", tag_format));
    }

    if let Some(branches) = &rc.branches {
        let converted = convert_branches(branches, &mut report);
        if !converted.is_empty() {
            workspace.branches = Some(converted);
        }
    }

    if let Some(url) = &rc.repository_url {
        report.ignored.push(format!(
            "repositoryUrl ({url}) — ferrflow derives the remote from git"
        ));
    }

    let mut changelog: Option<String> = None;
    let mut hooks = HooksConfig::default();
    let mut hooks_touched = false;

    for plugin in &rc.plugins {
        apply_plugin(
            plugin,
            &mut workspace,
            &mut changelog,
            &mut hooks,
            &mut hooks_touched,
            &mut report,
        );
    }

    if hooks_touched {
        workspace.hooks = Some(hooks);
    }

    let package = PackageConfig {
        name: default_package_name(),
        path: ".".to_string(),
        versioned_files: vec![VersionedFile {
            path: "package.json".to_string(),
            format: FileFormat::Json,
            selector: None,
        }],
        changelog: Some(changelog.unwrap_or_else(|| "CHANGELOG.md".to_string())),
        shared_paths: Vec::new(),
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        hooks: None,
        floating_tags: None,
        publishers: vec![],
        update_lockfiles: None,
    };

    report.warnings.push(
        "semantic-release is single-package; scaffolded one package writing to package.json. \
         Edit `package` if your layout differs, or add more for a monorepo."
            .to_string(),
    );

    Ok((
        Config {
            workspace,
            packages: vec![package],
        },
        report,
    ))
}

pub(super) fn default_package_name() -> String {
    std::fs::read_to_string("package.json")
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_string))
        .unwrap_or_else(|| "app".to_string())
}

/// `v${version}` → `v{{version}}`. semantic-release only defines the
/// `${version}` token; anything else is passed through and flagged.
fn convert_tag_format(tag_format: &str) -> String {
    tag_format.replace("${version}", "{{version}}")
}

fn convert_branches(branches: &Branches, report: &mut MigrationReport) -> Vec<BranchChannelConfig> {
    let specs: Vec<&BranchSpec> = match branches {
        Branches::One(b) => vec![b],
        Branches::Many(bs) => bs.iter().collect(),
    };

    let mut out = Vec::new();
    for spec in specs {
        match spec {
            BranchSpec::Name(name) => {
                // A bare non-release branch name in the list is a maintenance
                // or next branch; semantic-release treats `main`/`master` as
                // the stable line.
                let is_stable = name == "main" || name == "master";
                out.push(BranchChannelConfig {
                    name: name.clone(),
                    channel: ChannelValue::Stable(is_stable),
                    prerelease_identifier: PrereleaseIdentifier::default(),
                });
                report
                    .mapped
                    .push(format!("branch {name} → {}", channel_desc(is_stable, None)));
            }
            BranchSpec::Object {
                name,
                prerelease,
                channel,
            } => {
                let (channel_value, desc) = match prerelease {
                    Some(PrereleaseFlag::Bool(true)) => {
                        let id = channel.clone().unwrap_or_else(|| name.clone());
                        (
                            ChannelValue::Named(id.clone()),
                            format!("prerelease `{id}`"),
                        )
                    }
                    Some(PrereleaseFlag::Name(id)) => (
                        ChannelValue::Named(id.clone()),
                        format!("prerelease `{id}`"),
                    ),
                    Some(PrereleaseFlag::Bool(false)) | None => {
                        let is_stable = name == "main" || name == "master";
                        (
                            ChannelValue::Stable(is_stable),
                            channel_desc(is_stable, channel.as_deref()),
                        )
                    }
                };
                out.push(BranchChannelConfig {
                    name: name.clone(),
                    channel: channel_value,
                    prerelease_identifier: PrereleaseIdentifier::default(),
                });
                report.mapped.push(format!("branch {name} → {desc}"));
            }
        }
    }
    out
}

fn channel_desc(is_stable: bool, channel: Option<&str>) -> String {
    match (is_stable, channel) {
        (true, _) => "stable".to_string(),
        (false, Some(c)) => format!("channel `{c}`"),
        (false, None) => "stable (non-default branch)".to_string(),
    }
}

fn apply_plugin(
    plugin: &PluginEntry,
    workspace: &mut WorkspaceConfig,
    changelog: &mut Option<String>,
    hooks: &mut HooksConfig,
    hooks_touched: &mut bool,
    report: &mut MigrationReport,
) {
    match plugin.name() {
        "@semantic-release/commit-analyzer" => {
            if let Some(opts) = plugin.options()
                && (opts.get("releaseRules").is_some() || opts.get("preset").is_some())
            {
                report.warnings.push(
                    "commit-analyzer has custom releaseRules/preset — ferrflow's bump rules are \
                     fixed (feat→minor, fix/perf/refactor→patch, !/BREAKING→major) and can't honour \
                     them. Review whether the defaults match your intent."
                        .to_string(),
                );
            }
        }
        "@semantic-release/release-notes-generator" => {}
        "@semantic-release/changelog" => {
            let path = plugin
                .options()
                .and_then(|o| o.get("changelogFile"))
                .and_then(|v| v.as_str())
                .unwrap_or("CHANGELOG.md")
                .to_string();
            *changelog = Some(path.clone());
            report.mapped.push(format!("changelog plugin → {path}"));
        }
        "@semantic-release/github" | "@semantic-release/gitlab" => {
            workspace.forge = if plugin.name().contains("gitlab") {
                ForgeKind::Gitlab
            } else {
                ForgeKind::Github
            };
            report.mapped.push(format!("{} → forge", plugin.name()));
        }
        "@semantic-release/git" => {
            report.mapped.push(
                "git plugin → (implicit; ferrflow commits the release by default)".to_string(),
            );
        }
        "@semantic-release/exec" => {
            apply_exec_plugin(plugin, hooks, hooks_touched, report);
        }
        "@semantic-release/npm" => {
            report.warnings.push(
                "@semantic-release/npm → not mapped. ferrflow has publishers, but the semantics \
                 differ enough that an automatic map would mislead. Configure `publishers` by hand \
                 if you publish to npm."
                    .to_string(),
            );
        }
        other => {
            report.warnings.push(format!(
                "plugin {other} has no ferrflow equivalent — review manually"
            ));
        }
    }
}

fn apply_exec_plugin(
    plugin: &PluginEntry,
    hooks: &mut HooksConfig,
    hooks_touched: &mut bool,
    report: &mut MigrationReport,
) {
    let Some(opts) = plugin.options() else {
        return;
    };
    let get = |key: &str| opts.get(key).and_then(|v| v.as_str()).map(str::to_string);

    for (rc_key, ff_label, slot) in [
        ("verifyConditionsCmd", "preRelease", &mut hooks.pre_release),
        ("prepareCmd", "preBump", &mut hooks.pre_bump),
        ("publishCmd", "postPublish", &mut hooks.post_publish),
        ("successCmd", "onSuccess", &mut hooks.on_success),
        ("failCmd", "onError", &mut hooks.on_error),
    ] {
        if let Some(cmd) = get(rc_key) {
            *slot = Some(cmd);
            *hooks_touched = true;
            report
                .mapped
                .push(format!("exec {rc_key} → hooks.{ff_label}"));
        }
    }
}

fn migrate_semantic_release() -> Result<()> {
    let path = find_semantic_release_config().ok_or_else(|| {
        anyhow::anyhow!(
            "no semantic-release config found ({})",
            SEMANTIC_RELEASE_FILES.join(", ")
        )
    })?;
    let raw = read_source_as_json(&path)?;

    let (config, report) = build_config_from_releaserc(&raw)?;
    write_and_report(Source::SemanticRelease, &path, &config, &report)
}

/// Serialize the migrated config to `.ferrflow` (JSON) and print the report.
/// Shared by every source converter.
pub(super) fn write_and_report(
    source: Source,
    from: &Path,
    config: &Config,
    report: &MigrationReport,
) -> Result<()> {
    let handler = format_handler(ConfigFileFormat::Json);
    let content = handler.serialize(config)?;
    let filename = handler.filename();
    std::fs::write(filename, &content)?;
    print_report(source, from, filename, report);
    Ok(())
}

fn print_report(source: Source, from: &Path, wrote: &str, report: &MigrationReport) {
    println!(
        "Migrated {} config from {} → {wrote}\n",
        source.label(),
        from.display()
    );

    if !report.mapped.is_empty() {
        println!("Mapped:");
        for m in &report.mapped {
            println!("  • {m}");
        }
        println!();
    }
    if !report.ignored.is_empty() {
        println!("Ignored:");
        for i in &report.ignored {
            println!("  • {i}");
        }
        println!();
    }
    if !report.warnings.is_empty() {
        println!("Review manually:");
        for w in &report.warnings {
            println!("  ! {w}");
        }
        println!();
    }

    println!("Next: review {wrote}, then `ferrflow validate` and `ferrflow check`.");
}

#[cfg(test)]
mod tests;
