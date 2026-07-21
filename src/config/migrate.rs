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

/// A `.releaserc` shape we can parse: JSON (or JSON5) content, whatever the
/// filename. YAML `.releaserc` and JS `release.config.js` are surfaced as
/// unsupported rather than mis-parsed.
const SEMANTIC_RELEASE_JSON_FILES: &[&str] = &[".releaserc", ".releaserc.json"];
const SEMANTIC_RELEASE_UNSUPPORTED_FILES: &[&str] = &[
    ".releaserc.yaml",
    ".releaserc.yml",
    ".releaserc.js",
    "release.config.js",
    "release.config.mjs",
    ".releaserc.cjs",
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
    if find_semantic_release_json().is_some() {
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
    if let Some(f) = SEMANTIC_RELEASE_UNSUPPORTED_FILES
        .iter()
        .find(|f| Path::new(f).exists())
    {
        return Err(anyhow::anyhow!(
            "found {f}, but ferrflow can only migrate JSON `.releaserc` for now. \
             Convert it to `.releaserc.json` (or inline the config as JSON) and rerun."
        ))
        .error_code(error_code::CONFIG_INVALID_JSON);
    }
    Err(anyhow::anyhow!(
        "no supported release-tool config found. Looked for {}, {}, {}, {}. \
         Pass --from to force a source.",
        SEMANTIC_RELEASE_JSON_FILES.join(", "),
        changesets::CONFIG_FILE,
        release_please::CONFIG_FILE,
        standard_version::CONFIG_FILES.join(", "),
    ))
    .error_code(error_code::CONFIG_NOT_FOUND)
}

fn find_semantic_release_json() -> Option<PathBuf> {
    SEMANTIC_RELEASE_JSON_FILES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
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
    let path = find_semantic_release_json().ok_or_else(|| {
        anyhow::anyhow!(
            "no JSON .releaserc found ({})",
            SEMANTIC_RELEASE_JSON_FILES.join(", ")
        )
    })?;
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("could not read {}: {e}", path.display()))?;

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
