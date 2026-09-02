use std::sync::OnceLock;

use crate::config::{ChangelogConfig, CommitFormats};
use crate::conventional_commits::determine_bump;
use crate::conventional_commits::{
    BumpType, CommitCategory, breaking_footer_body, classify_commit, is_breaking, parse_header,
    parse_subject,
};
use anyhow::Result;
use chrono::Local;
use std::path::Path;

#[derive(Clone)]
pub struct GitLog {
    pub id: String,
    pub hash: String,
    pub message: String,
}

#[derive(Default)]
pub struct ChangelogRender<'a> {
    pub config: Option<&'a ChangelogConfig>,
    pub formats: Option<&'a CommitFormats>,
    pub forge_base: Option<String>,
    pub last_tag: Option<String>,
    pub new_tag: Option<String>,
}

impl<'a> ChangelogRender<'a> {
    fn formats(&self) -> &CommitFormats {
        static FALLBACK: OnceLock<CommitFormats> = OnceLock::new();
        self.formats
            .unwrap_or_else(|| FALLBACK.get_or_init(CommitFormats::default))
    }
}

#[cfg(feature = "cli")]
pub fn generate_only(config_path: Option<&Path>, dry_run: bool) -> Result<()> {
    use crate::config::Config;
    use crate::formats::read_version;
    use crate::git::{
        find_highest_semver_tag_with_cache, get_commits_since_last_tag, get_repo_root, open_repo,
    };
    use crate::versioning::bump_version;
    use colored::Colorize;
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if config.packages.is_empty() {
        tracing::warn!(
            "{}",
            "No packages configured. Run `ferrflow init` to create a ferrflow config.".yellow()
        );
        return Ok(());
    }

    let forge_base = config.workspace.changelog.as_ref().and_then(|cl| {
        if cl.include_commit_links || cl.include_compare_link {
            crate::git::get_remote_url(&repo, &config.workspace.remote)
                .as_deref()
                .and_then(crate::forge::web_base_url)
        } else {
            None
        }
    });

    for pkg in &config.packages {
        let tag_prefix = format!("{}@v", pkg.name);
        let skip_markers = config.workspace.effective_commit_skip_markers();
        let commits = get_commits_since_last_tag(
            &repo,
            &tag_prefix,
            config.workspace.orphaned_tag_strategy,
            &skip_markers,
            None,
        )?;

        if commits.is_empty() {
            continue;
        }

        let bump = commits
            .iter()
            .map(|c| determine_bump(&c.message, &config.workspace.commit_formats))
            .max()
            .unwrap_or(BumpType::None);

        if bump == BumpType::None {
            continue;
        }

        let highest_tag = find_highest_semver_tag_with_cache(
            &repo,
            &tag_prefix,
            config.workspace.orphaned_tag_strategy,
            None,
        )?;
        let last_tag = highest_tag.as_ref().map(|(tag, _version)| tag.clone());

        let current_version = match pkg.versioned_files.first() {
            Some(vf) => read_version(vf, &root)?,
            None => highest_tag
                .map(|(_tag, version)| version)
                .unwrap_or_else(|| "0.0.0".to_string()),
        };
        let new_version = bump_version(&current_version, bump)?;
        let new_tag = pkg.tag_for_version(&config.workspace, config.is_monorepo(), &new_version);

        let changelog_path = match &pkg.changelog {
            Some(rel) => crate::formats::join_within_repo(&root, rel)?,
            None => {
                tracing::warn!(
                    "{}",
                    format!(
                        "  No changelog configured for '{}', defaulting to CHANGELOG.md.",
                        pkg.name
                    )
                    .yellow()
                );
                root.join("CHANGELOG.md")
            }
        };

        let render = ChangelogRender {
            formats: None,
            config: config.workspace.changelog.as_ref(),
            forge_base: forge_base.clone(),
            last_tag,
            new_tag: Some(new_tag),
        };

        update_changelog_with(
            &changelog_path,
            &pkg.name,
            &new_version,
            &commits,
            bump,
            dry_run,
            &render,
        )?;
    }

    Ok(())
}

pub fn build_section_with(
    new_version: &str,
    commits: &[GitLog],
    render: &ChangelogRender,
) -> String {
    match render.config {
        Some(config) => build_rich_section(new_version, commits, config, render),
        None => build_classic_section(new_version, commits, render.formats()),
    }
}

fn build_classic_section(new_version: &str, commits: &[GitLog], formats: &CommitFormats) -> String {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let mut breaking = Vec::new();
    let mut features = Vec::new();
    let mut fixes = Vec::new();
    let mut refactors = Vec::new();

    for commit in commits {
        let subject = parse_subject(&commit.message);
        match classify_commit(&commit.message, formats) {
            CommitCategory::Breaking => breaking.push(format!("- {subject}")),
            CommitCategory::Feature => features.push(format!("- {subject}")),
            CommitCategory::Fix => fixes.push(format!("- {subject}")),
            CommitCategory::Refactor => refactors.push(format!("- {subject}")),
            CommitCategory::Other => {}
        }
    }

    let mut section = format!("\n## [{new_version}] - {date}\n");

    if !breaking.is_empty() {
        section.push_str("\n### Breaking Changes\n\n");
        section.push_str(&breaking.join("\n"));
        section.push('\n');
    }
    if !features.is_empty() {
        section.push_str("\n### Features\n\n");
        section.push_str(&features.join("\n"));
        section.push('\n');
    }
    if !fixes.is_empty() {
        section.push_str("\n### Bug Fixes\n\n");
        section.push_str(&fixes.join("\n"));
        section.push('\n');
    }
    if !refactors.is_empty() {
        section.push_str("\n### Refactoring\n\n");
        section.push_str(&refactors.join("\n"));
        section.push('\n');
    }

    section
}

const BREAKING_KEY: &str = "breaking";

fn render_section_key(
    message: &str,
    formats: &CommitFormats,
    configured: &[String],
) -> Option<String> {
    let category = classify_commit(message, formats);
    if category == CommitCategory::Breaking {
        return Some(BREAKING_KEY.to_string());
    }

    let header = parse_header(message);

    if let Some(h) = header.as_ref() {
        match h.commit_type {
            "perf" => return Some("perf".to_string()),
            "security" => return Some("security".to_string()),
            "docs" => return Some("docs".to_string()),
            other if category == CommitCategory::Other && configured.iter().any(|k| k == other) => {
                return Some(other.to_string());
            }
            _ => {}
        }
    }

    let is_security_scope = header
        .as_ref()
        .and_then(|h| h.scope)
        .is_some_and(|s| s.eq_ignore_ascii_case("security"));

    match category {
        CommitCategory::Feature => Some("feat".to_string()),
        CommitCategory::Fix if is_security_scope => Some("security".to_string()),
        CommitCategory::Fix => Some("fix".to_string()),
        CommitCategory::Refactor => Some("refactor".to_string()),
        CommitCategory::Breaking | CommitCategory::Other => None,
    }
}

fn configured_section_keys(config: &ChangelogConfig) -> Vec<String> {
    config
        .sections
        .as_ref()
        .map(|map| {
            map.iter()
                .filter(|(_, setting)| !setting.is_hidden())
                .map(|(key, _)| key.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn resolve_sections(config: &ChangelogConfig) -> Vec<(String, String)> {
    let default_label = |key: &str| -> String {
        match key {
            "feat" => "Features",
            "fix" => "Bug Fixes",
            "perf" => "Performance",
            "security" => "Security",
            "docs" => "Documentation",
            "refactor" => "Code Refactoring",
            _ => "Changes",
        }
        .to_string()
    };

    let mut enabled: Vec<(String, String)> = Vec::new();
    let mut push = |key: &str, label: String| {
        if !enabled.iter().any(|(k, _)| k == key) {
            enabled.push((key.to_string(), label));
        }
    };

    match &config.sections {
        None => {
            push("feat", default_label("feat"));
            push("fix", default_label("fix"));
        }
        Some(map) => {
            let label_for = |key: &str| -> Option<String> {
                map.get(key).and_then(|setting| {
                    (!setting.is_hidden()).then(|| {
                        setting
                            .label()
                            .map(str::to_string)
                            .unwrap_or_else(|| default_label(key))
                    })
                })
            };
            for key in ["feat", "fix", "perf", "security", "docs", "refactor"] {
                if let Some(label) = label_for(key) {
                    push(key, label);
                }
            }
            for key in map.keys() {
                if let Some(label) = label_for(key) {
                    push(key, label);
                }
            }
        }
    }

    enabled
}

struct Entry {
    scope: Option<String>,
    text: String,
}

fn build_rich_section(
    new_version: &str,
    commits: &[GitLog],
    config: &ChangelogConfig,
    render: &ChangelogRender,
) -> String {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let sections = resolve_sections(config);
    let configured = configured_section_keys(config);

    let mut breaking: Vec<Entry> = Vec::new();
    let mut buckets: std::collections::BTreeMap<String, Vec<Entry>> =
        std::collections::BTreeMap::new();

    for commit in commits {
        let Some(key) = render_section_key(&commit.message, render.formats(), &configured) else {
            continue;
        };
        let entry = build_entry(commit, config, render);
        if key == BREAKING_KEY {
            breaking.push(entry);
        } else if sections.iter().any(|(k, _)| *k == key) {
            buckets.entry(key).or_default().push(entry);
        }
    }

    let mut section = format!("\n## [{new_version}] - {date}\n");

    if !breaking.is_empty() {
        section.push_str("\n### Breaking Changes\n\n");
        section.push_str(&render_entries(&breaking, config.group_by_scope));
        section.push('\n');
    }

    for (key, label) in &sections {
        if let Some(entries) = buckets.get(key)
            && !entries.is_empty()
        {
            section.push_str(&format!("\n### {label}\n\n"));
            section.push_str(&render_entries(entries, config.group_by_scope));
            section.push('\n');
        }
    }

    if config.include_compare_link
        && let (Some(base), Some(last), Some(new_tag)) =
            (&render.forge_base, &render.last_tag, &render.new_tag)
    {
        section.push_str(&format!(
            "\n[{new_version}]: {base}/compare/{last}...{new_tag}\n"
        ));
    }

    section
}

fn build_entry(commit: &GitLog, config: &ChangelogConfig, render: &ChangelogRender) -> Entry {
    let header = parse_header(&commit.message);
    let scope = header.as_ref().and_then(|h| h.scope.map(|s| s.to_string()));

    let breaking = is_breaking(&commit.message, render.formats());
    let body = if breaking {
        let desc = breaking_footer_body(&commit.message);
        match (desc, header.as_ref()) {
            (Some(d), _) => d,
            (None, Some(h)) => h.description.to_string(),
            (None, None) => parse_subject(&commit.message).to_string(),
        }
    } else {
        match header.as_ref() {
            Some(h) => h.description.to_string(),
            None => parse_subject(&commit.message).to_string(),
        }
    };

    let mut text = body;
    if config.include_commit_links
        && let Some(base) = &render.forge_base
    {
        let short = &commit.hash;
        text.push_str(&format!(" ([{short}]({base}/commit/{short}))"));
    }

    Entry { scope, text }
}

fn render_entries(entries: &[Entry], group_by_scope: bool) -> String {
    if !group_by_scope {
        return entries
            .iter()
            .map(|e| format!("- {}", e.text))
            .collect::<Vec<_>>()
            .join("\n");
    }

    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by(|a, b| match (&a.scope, &b.scope) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(x), Some(y)) => x.cmp(y),
    });

    sorted
        .iter()
        .map(|e| match &e.scope {
            Some(scope) => format!("- **{scope}:** {}", e.text),
            None => format!("- {}", e.text),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn update_changelog(
    changelog_path: &Path,
    package_name: &str,
    new_version: &str,
    commits: &[GitLog],
    bump: BumpType,
    dry_run: bool,
) -> Result<()> {
    update_changelog_with(
        changelog_path,
        package_name,
        new_version,
        commits,
        bump,
        dry_run,
        &ChangelogRender::default(),
    )
}

#[allow(clippy::too_many_arguments)]
fn changelog_existing(changelog_path: &Path, package_name: &str) -> Result<String> {
    if changelog_path.exists() {
        Ok(std::fs::read_to_string(changelog_path)?)
    } else {
        Ok(format!(
            "# Changelog\n\nAll notable changes to `{package_name}` will be documented here.\n\nThe format is based on [Keep a Changelog](https://keepachangelog.com/).\n"
        ))
    }
}

/// The section a changelog currently holds for `version`, from its heading to
/// the next one.
///
/// Read back after `postBump` runs so that a hook which rewrote the file is
/// what reaches the tag, the forge release body and the release commit,
/// instead of the text generated before the hook had a say.
pub fn section_for_version(changelog: &str, version: &str) -> Option<String> {
    let start = changelog.find(&format!("## [{version}]"))?;
    let rest = &changelog[start..];
    let end = rest[1..]
        .find(
            "
## ",
        )
        .map(|i| i + 1)
        .unwrap_or(rest.len());
    Some(rest[..end].trim_end().to_string())
}

/// The section to publish for `version`, preferring what the changelog file
/// holds on disk over `generated`.
///
/// Called after `postBump`, so a hook that rewrote the file is what reaches
/// the tag, the forge release body and the release commit. Falls back to
/// `generated` whenever the file cannot be read or no longer carries a
/// heading for this version, which keeps a hook that mangles the changelog
/// from emptying the release notes as well.
pub fn published_section(
    root: &Path,
    changelog_rel: Option<&str>,
    version: &str,
    generated: &str,
) -> String {
    changelog_rel
        .map(|rel| root.join(rel))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|content| section_for_version(&content, version))
        .unwrap_or_else(|| generated.to_string())
}

fn splice_section(existing: &str, section: &str) -> String {
    if let Some(pos) = existing.find("\n## ") {
        format!("{}{}{}", &existing[..pos], section, &existing[pos..])
    } else {
        format!("{}\n{}", existing.trim_end(), section)
    }
}

/// Compute the `(old, new)` changelog contents a release would produce,
/// without writing anything. Returns `None` when the bump is `None`
/// (nothing would change). Used by `release --dry-run --verbose` to
/// render a unified diff of the changelog alongside the versioned files.
#[cfg(feature = "cli")]
pub fn compute_changelog_update(
    changelog_path: &Path,
    package_name: &str,
    new_version: &str,
    commits: &[GitLog],
    bump: BumpType,
    render: &ChangelogRender,
) -> Result<Option<(String, String)>> {
    if bump == BumpType::None {
        return Ok(None);
    }
    let section = build_section_with(new_version, commits, render);
    let existing = changelog_existing(changelog_path, package_name)?;
    let new_content = splice_section(&existing, &section);
    Ok(Some((existing, new_content)))
}

pub fn update_changelog_with(
    changelog_path: &Path,
    package_name: &str,
    new_version: &str,
    commits: &[GitLog],
    bump: BumpType,
    dry_run: bool,
    render: &ChangelogRender,
) -> Result<()> {
    if bump == BumpType::None {
        return Ok(());
    }

    let section = build_section_with(new_version, commits, render);

    if dry_run {
        tracing::info!(
            "  [dry-run] Would update {}: {}",
            changelog_path.display(),
            section.trim()
        );
        return Ok(());
    }

    let existing = changelog_existing(changelog_path, package_name)?;
    let new_content = splice_section(&existing, &section);

    std::fs::write(changelog_path, new_content)?;
    tracing::info!("  ✓ Updated {}", changelog_path.display());
    Ok(())
}

#[cfg(test)]
mod published_section_tests {
    use super::{published_section, section_for_version};

    const FILE: &str = "# Changelog

## [1.1.0] - 2026-09-01

### Features

- Added passkey authentication support.

## [1.0.0] - 2026-08-01

### Features

- The first one.
";

    #[test]
    fn a_section_stops_at_the_next_version() {
        let section = section_for_version(FILE, "1.1.0").expect("1.1.0 present");

        assert!(section.contains("passkey"), "{section}");
        assert!(
            !section.contains("1.0.0") && !section.contains("The first one"),
            "the older release must not bleed into the newer section: {section}"
        );
    }

    #[test]
    fn the_last_section_runs_to_the_end_of_the_file() {
        let section = section_for_version(FILE, "1.0.0").expect("1.0.0 present");

        assert!(section.contains("The first one"), "{section}");
    }

    #[test]
    fn a_version_the_file_does_not_carry_is_absent_rather_than_guessed() {
        assert!(section_for_version(FILE, "2.0.0").is_none());
    }

    #[test]
    fn a_hook_rewrite_is_what_gets_published() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CHANGELOG.md"), FILE).unwrap();

        let published = published_section(
            dir.path(),
            Some("CHANGELOG.md"),
            "1.1.0",
            "## [1.1.0]

- feat(auth): support passkeys
",
        );

        assert!(
            published.contains("Added passkey authentication support"),
            "the file on disk wins over the text generated before the hook: {published}"
        );
        assert!(
            !published.contains("feat(auth)"),
            "the pre-hook text must not survive: {published}"
        );
    }

    #[test]
    fn a_package_with_no_changelog_keeps_the_generated_text() {
        let dir = tempfile::tempdir().unwrap();

        let published = published_section(dir.path(), None, "1.1.0", "generated");

        assert_eq!(published, "generated");
    }

    #[test]
    fn a_changelog_a_hook_mangled_falls_back_instead_of_publishing_nothing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("CHANGELOG.md"),
            "# Changelog

oops
",
        )
        .unwrap();

        let published = published_section(dir.path(), Some("CHANGELOG.md"), "1.1.0", "generated");

        assert_eq!(
            published, "generated",
            "losing the heading must not empty the release notes"
        );
    }

    #[test]
    fn a_missing_file_falls_back_rather_than_failing_the_release() {
        let dir = tempfile::tempdir().unwrap();

        let published = published_section(dir.path(), Some("absent.md"), "1.1.0", "generated");

        assert_eq!(published, "generated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_section(new_version: &str, commits: &[GitLog]) -> String {
        build_section_with(new_version, commits, &ChangelogRender::default())
    }

    fn make_commits(messages: &[&str]) -> Vec<GitLog> {
        messages
            .iter()
            .map(|m| GitLog {
                id: String::new(),
                hash: "abc1234".to_string(),
                message: m.to_string(),
            })
            .collect()
    }

    #[test]
    fn build_section_features_only() {
        let commits = make_commits(&["feat: add login", "feat(ui): new dashboard"]);
        let section = build_section("1.1.0", &commits);
        assert!(section.contains("## [1.1.0]"));
        assert!(section.contains("### Features"));
        assert!(section.contains("- feat: add login"));
        assert!(section.contains("- feat(ui): new dashboard"));
        assert!(!section.contains("### Bug Fixes"));
        assert!(!section.contains("### Breaking Changes"));
    }

    #[test]
    fn build_section_fixes_only() {
        let commits = make_commits(&["fix: null pointer", "perf: faster query"]);
        let section = build_section("1.0.1", &commits);
        assert!(section.contains("### Bug Fixes"));
        assert!(section.contains("- fix: null pointer"));
        assert!(section.contains("- perf: faster query"));
        assert!(!section.contains("### Features"));
    }

    #[test]
    fn build_section_breaking_changes() {
        let commits = make_commits(&["feat!: remove old API"]);
        let section = build_section("2.0.0", &commits);
        assert!(section.contains("### Breaking Changes"));
        assert!(section.contains("- feat!: remove old API"));
    }

    #[test]
    fn build_section_mixed_commits() {
        let commits = make_commits(&[
            "feat: new feature",
            "fix: bug fix",
            "feat!: breaking",
            "chore: update deps",
        ]);
        let section = build_section("2.0.0", &commits);
        assert!(section.contains("### Breaking Changes"));
        assert!(section.contains("### Features"));
        assert!(section.contains("### Bug Fixes"));
        assert!(!section.contains("chore: update deps"));
    }

    #[test]
    fn build_section_does_not_misclassify_prose() {
        let commits = make_commits(&[
            "fix: handle the !: token in the parser",
            "features added without a conventional prefix",
        ]);
        let section = build_section("1.0.1", &commits);
        assert!(section.contains("### Bug Fixes"));
        assert!(section.contains("- fix: handle the !: token in the parser"));
        assert!(!section.contains("### Breaking Changes"));
        assert!(!section.contains("### Features"));
    }

    #[test]
    fn build_section_empty_commits() {
        let section = build_section("1.0.0", &[]);
        assert!(section.contains("## [1.0.0]"));
        assert!(!section.contains("### "));
    }

    #[test]
    fn update_changelog_creates_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        let commits = make_commits(&["feat: initial"]);
        update_changelog(&path, "myapp", "0.1.0", &commits, BumpType::Minor, false).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Changelog"));
        assert!(content.contains("## [0.1.0]"));
        assert!(content.contains("- feat: initial"));
    }

    #[test]
    fn update_changelog_inserts_before_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        std::fs::write(
            &path,
            "# Changelog\n\n## [1.0.0] - 2025-01-01\n\n- old stuff\n",
        )
        .unwrap();
        let commits = make_commits(&["feat: new stuff"]);
        update_changelog(&path, "myapp", "1.1.0", &commits, BumpType::Minor, false).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let pos_new = content.find("## [1.1.0]").unwrap();
        let pos_old = content.find("## [1.0.0]").unwrap();
        assert!(pos_new < pos_old, "new version should come before old");
    }

    #[test]
    fn update_changelog_skips_none_bump() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        update_changelog(&path, "myapp", "1.0.0", &[], BumpType::None, false).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn update_changelog_dry_run_no_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        let commits = make_commits(&["feat: something"]);
        update_changelog(&path, "myapp", "1.0.0", &commits, BumpType::Minor, true).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn build_section_breaking_change_in_body() {
        let commits = vec![GitLog {
            id: String::new(),
            hash: "abc".to_string(),
            message: "feat: add endpoint\n\nBREAKING CHANGE: removed old endpoint".to_string(),
        }];
        let section = build_section("2.0.0", &commits);
        assert!(section.contains("### Breaking Changes"));
    }

    #[test]
    fn build_section_chore_docs_excluded_refactor_kept() {
        let commits = make_commits(&[
            "refactor: clean up",
            "chore: update deps",
            "docs: update readme",
            "ci: fix pipeline",
            "style: format code",
            "test: add tests",
            "build: update config",
        ]);
        let section = build_section("1.0.1", &commits);
        assert!(!section.contains("### Features"));
        assert!(!section.contains("### Bug Fixes"));
        assert!(!section.contains("### Breaking Changes"));
        assert!(
            section.contains("### Refactoring"),
            "refactor: must render its own section, not be silently dropped"
        );
        assert!(section.contains("- refactor: clean up"));
        assert!(!section.contains("chore: update deps"));
        assert!(!section.contains("docs: update readme"));
        assert!(!section.contains("ci: fix pipeline"));
        assert!(!section.contains("style: format code"));
        assert!(!section.contains("test: add tests"));
        assert!(!section.contains("build: update config"));
    }

    #[test]
    fn build_section_treats_feature_as_a_feature_but_not_a_bare_feat() {
        let commits = make_commits(&["feature: renamed type", "feat add no colon"]);
        let section = build_section("1.0.1", &commits);
        assert!(section.contains("### Features"));
        assert!(section.contains("renamed type"));
        assert!(!section.contains("no colon"));
    }

    #[test]
    fn build_section_scoped_commits() {
        let commits = make_commits(&["feat(api): add endpoint", "fix(db): connection leak"]);
        let section = build_section("1.1.0", &commits);
        assert!(section.contains("- feat(api): add endpoint"));
        assert!(section.contains("- fix(db): connection leak"));
    }

    #[test]
    fn build_section_perf_in_fixes() {
        let commits = make_commits(&["perf: optimize query"]);
        let section = build_section("1.0.1", &commits);
        assert!(section.contains("### Bug Fixes"));
        assert!(section.contains("- perf: optimize query"));
    }

    #[test]
    fn update_changelog_preserves_existing_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        std::fs::write(
            &path,
            "# Changelog\n\n## [1.0.0] - 2025-01-01\n\n- feat: initial release\n",
        )
        .unwrap();
        let commits = make_commits(&["feat: new feature"]);
        update_changelog(&path, "myapp", "1.1.0", &commits, BumpType::Minor, false).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("## [1.1.0]"));
        assert!(content.contains("## [1.0.0]"));
        assert!(content.contains("- feat: initial release"));
    }

    #[test]
    fn update_changelog_empty_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        std::fs::write(&path, "").unwrap();
        let commits = make_commits(&["feat: first"]);
        update_changelog(&path, "myapp", "0.1.0", &commits, BumpType::Minor, false).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("## [0.1.0]"));
    }

    #[test]
    fn build_section_contains_date() {
        let commits = make_commits(&["feat: something"]);
        let section = build_section("1.0.0", &commits);
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        assert!(section.contains(&today));
    }

    #[test]
    fn update_changelog_multiple_versions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");

        let commits1 = make_commits(&["feat: first"]);
        update_changelog(&path, "myapp", "0.1.0", &commits1, BumpType::Minor, false).unwrap();

        let commits2 = make_commits(&["feat: second"]);
        update_changelog(&path, "myapp", "0.2.0", &commits2, BumpType::Minor, false).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let pos1 = content.find("## [0.2.0]").unwrap();
        let pos2 = content.find("## [0.1.0]").unwrap();
        assert!(pos1 < pos2, "newer version should come first");
    }

    use crate::config::{ChangelogConfig, SectionSetting};
    use std::collections::BTreeMap;

    fn sections(pairs: &[(&str, SectionSetting)]) -> BTreeMap<String, SectionSetting> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn enabled() -> SectionSetting {
        SectionSetting::Enabled(true)
    }

    fn labelled(label: &str) -> SectionSetting {
        SectionSetting::Label(label.to_string())
    }

    fn render_with(map: BTreeMap<String, SectionSetting>, commits: &[GitLog]) -> String {
        let config = ChangelogConfig {
            sections: Some(map),
            ..Default::default()
        };
        let render = ChangelogRender {
            config: Some(&config),
            ..Default::default()
        };
        build_section_with("1.0.0", commits, &render)
    }

    #[test]
    fn a_configured_section_renders_a_type_the_built_ins_do_not_cover() {
        let commits = make_commits(&["chore: bump deps", "ci: cache uv", "build: pin wheel"]);
        let out = render_with(
            sections(&[
                ("chore", labelled("Chores")),
                ("ci", labelled("CI")),
                ("build", labelled("Build")),
            ]),
            &commits,
        );
        assert!(out.contains("### Chores"), "{out}");
        assert!(out.contains("- bump deps"), "{out}");
        assert!(out.contains("### CI"), "{out}");
        assert!(out.contains("### Build"), "{out}");
    }

    #[test]
    fn an_unconfigured_type_is_still_dropped() {
        let commits = make_commits(&["chore: bump deps", "test: add cases"]);
        let out = render_with(sections(&[("chore", labelled("Chores"))]), &commits);
        assert!(out.contains("- bump deps"), "{out}");
        assert!(
            !out.contains("add cases"),
            "test: must stay out when unconfigured: {out}"
        );
    }

    #[test]
    fn a_hidden_extra_section_stays_hidden() {
        let commits = make_commits(&["chore: bump deps"]);
        let out = render_with(
            sections(&[("chore", SectionSetting::Enabled(false))]),
            &commits,
        );
        assert!(!out.contains("bump deps"), "{out}");
    }

    #[test]
    fn configuring_an_extra_section_does_not_move_the_built_in_ones() {
        let commits = make_commits(&[
            "feat: add login",
            "fix(security): patch xss",
            "refactor: clean up",
            "chore: bump deps",
        ]);
        let out = render_with(
            sections(&[
                ("feat", enabled()),
                ("fix", enabled()),
                ("security", enabled()),
                ("refactor", enabled()),
                ("chore", labelled("Chores")),
            ]),
            &commits,
        );
        assert!(out.contains("### Features"), "{out}");
        assert!(out.contains("### Security"), "{out}");
        assert!(
            out.contains("patch xss"),
            "fix(security) must stay in Security: {out}"
        );
        assert!(out.contains("### Code Refactoring"), "{out}");
        assert!(out.contains("### Chores"), "{out}");
    }

    #[test]
    fn a_breaking_extra_typed_commit_still_lands_in_breaking_changes() {
        let commits = make_commits(&["chore!: drop python 3.8"]);
        let out = render_with(sections(&[("chore", labelled("Chores"))]), &commits);
        assert!(out.contains("### Breaking Changes"), "{out}");
        assert!(
            !out.contains("### Chores"),
            "breaking wins over the type section: {out}"
        );
    }

    #[test]
    fn default_render_is_byte_identical_to_classic() {
        let commits = make_commits(&[
            "feat: add login",
            "feat(ui): new dashboard",
            "fix: null pointer",
            "perf: faster query",
            "refactor: clean up",
            "feat!: remove old API",
            "chore: noise",
        ]);
        let with_default = build_section_with("1.2.3", &commits, &ChangelogRender::default());
        let classic = build_classic_section("1.2.3", &commits, &Default::default());
        assert_eq!(with_default, classic);
    }

    #[test]
    fn rich_render_with_no_config_field_matches_classic() {
        let commits = make_commits(&["feat: a", "fix: b"]);
        let render = ChangelogRender {
            formats: None,
            config: None,
            ..Default::default()
        };
        assert_eq!(
            build_section_with("1.0.0", &commits, &render),
            build_classic_section("1.0.0", &commits, &Default::default())
        );
    }

    #[test]
    fn rich_render_files_breaking_perf_and_docs_under_breaking_changes() {
        let formats = CommitFormats::default();
        for subject in ["perf!: drop legacy API", "docs!: remove the v1 guide"] {
            assert_eq!(
                render_section_key(subject, &formats, &[]).as_deref(),
                Some(BREAKING_KEY),
                "{subject:?} bumps major and must render as breaking"
            );
        }
        assert_eq!(
            render_section_key("perf: cache", &formats, &[]).as_deref(),
            Some("perf")
        );
        assert_eq!(
            render_section_key("docs: typo", &formats, &[]).as_deref(),
            Some("docs")
        );
        assert_eq!(
            render_section_key("security: patch", &formats, &[]).as_deref(),
            Some("security")
        );
    }

    #[test]
    fn rich_render_keeps_permissive_commits_the_bump_counted() {
        let cfg = ChangelogConfig {
            sections: Some(sections(&[
                ("feat", enabled()),
                ("fix", enabled()),
                ("refactor", enabled()),
            ])),
            ..Default::default()
        };
        let commits = make_commits(&[
            "Feat: title-case feature",
            "feature: aliased feature",
            "Fix/branch-style-fix",
            "Refactor/branch-style-refactor",
        ]);
        let formats = CommitFormats::default();
        let render = ChangelogRender {
            formats: Some(&formats),
            config: Some(&cfg),
            ..Default::default()
        };
        let s = build_section_with("1.1.0", &commits, &render);

        for expected in [
            "title-case feature",
            "aliased feature",
            "branch-style-fix",
            "branch-style-refactor",
        ] {
            assert!(
                s.contains(expected),
                "{expected:?} bumped the version but never reached the changelog:\n{s}"
            );
        }
        assert!(s.contains("### Features"));
        assert!(s.contains("### Bug Fixes"));
        assert!(
            s.contains("### Code Refactoring"),
            "branch-style refactor should keep its own section:\n{s}"
        );
    }

    #[test]
    fn rich_render_perf_and_security_sections() {
        let cfg = ChangelogConfig {
            sections: Some(sections(&[
                ("feat", enabled()),
                ("fix", enabled()),
                ("perf", enabled()),
                ("security", enabled()),
            ])),
            ..Default::default()
        };
        let commits = make_commits(&[
            "feat: new endpoint",
            "perf: cache results",
            "fix(security): patch xss",
            "fix: small bug",
        ]);
        let render = ChangelogRender {
            formats: None,
            config: Some(&cfg),
            ..Default::default()
        };
        let s = build_section_with("1.1.0", &commits, &render);
        assert!(s.contains("### Features"));
        assert!(s.contains("### Performance"));
        assert!(s.contains("### Security"));
        assert!(s.contains("### Bug Fixes"));
        assert!(s.contains("- cache results"));
        assert!(s.contains("- patch xss"));
        let pos_perf = s.find("### Performance").unwrap();
        let pos_sec = s.find("### Security").unwrap();
        let pos_fix = s.find("### Bug Fixes").unwrap();
        assert!(pos_fix < pos_perf, "Bug Fixes precedes Performance");
        assert!(pos_perf < pos_sec, "Performance precedes Security");
    }

    #[test]
    fn rich_render_security_takes_precedence_over_fix() {
        let cfg = ChangelogConfig {
            sections: Some(sections(&[("fix", enabled()), ("security", enabled())])),
            ..Default::default()
        };
        let commits = make_commits(&["fix(security): sanitize input"]);
        let render = ChangelogRender {
            formats: None,
            config: Some(&cfg),
            ..Default::default()
        };
        let s = build_section_with("1.0.1", &commits, &render);
        assert!(s.contains("### Security"));
        let sec = s.find("### Security").unwrap();
        assert!(s[sec..].contains("- sanitize input"));
    }

    #[test]
    fn rich_render_docs_opt_in_and_hidden() {
        let with_docs = ChangelogConfig {
            sections: Some(sections(&[("docs", enabled())])),
            ..Default::default()
        };
        let commits = make_commits(&["docs: explain config"]);
        let r = ChangelogRender {
            formats: None,
            config: Some(&with_docs),
            ..Default::default()
        };
        assert!(build_section_with("1.0.1", &commits, &r).contains("### Documentation"));

        let hidden = ChangelogConfig {
            sections: Some(sections(&[("docs", SectionSetting::Enabled(false))])),
            ..Default::default()
        };
        let r2 = ChangelogRender {
            formats: None,
            config: Some(&hidden),
            ..Default::default()
        };
        assert!(!build_section_with("1.0.1", &commits, &r2).contains("### Documentation"));
    }

    #[test]
    fn rich_render_custom_labels() {
        let cfg = ChangelogConfig {
            sections: Some(sections(&[(
                "feat",
                SectionSetting::Label("New Stuff".to_string()),
            )])),
            ..Default::default()
        };
        let commits = make_commits(&["feat: x"]);
        let r = ChangelogRender {
            formats: None,
            config: Some(&cfg),
            ..Default::default()
        };
        let s = build_section_with("1.1.0", &commits, &r);
        assert!(s.contains("### New Stuff"));
        assert!(!s.contains("### Features"));
    }

    #[test]
    fn rich_render_breaking_uses_footer_body() {
        let cfg = ChangelogConfig::default();
        let commits = vec![GitLog {
            id: String::new(),
            hash: "abc1234".to_string(),
            message: "feat!: change api\n\nBREAKING CHANGE: the v1 endpoint was removed"
                .to_string(),
        }];
        let r = ChangelogRender {
            formats: None,
            config: Some(&cfg),
            ..Default::default()
        };
        let s = build_section_with("2.0.0", &commits, &r);
        assert!(s.contains("### Breaking Changes"));
        assert!(s.contains("- the v1 endpoint was removed"));
    }

    #[test]
    fn rich_render_breaking_bang_without_footer_uses_description() {
        let cfg = ChangelogConfig::default();
        let commits = make_commits(&["feat!: drop legacy flag"]);
        let r = ChangelogRender {
            formats: None,
            config: Some(&cfg),
            ..Default::default()
        };
        let s = build_section_with("2.0.0", &commits, &r);
        assert!(s.contains("### Breaking Changes"));
        assert!(s.contains("- drop legacy flag"));
    }

    #[test]
    fn rich_render_scope_grouping_inline_bold() {
        let cfg = ChangelogConfig {
            sections: Some(sections(&[("feat", enabled())])),
            group_by_scope: true,
            ..Default::default()
        };
        let commits = make_commits(&[
            "feat(api): add events endpoint",
            "feat(db): add index",
            "feat: top-level change",
        ]);
        let r = ChangelogRender {
            formats: None,
            config: Some(&cfg),
            ..Default::default()
        };
        let s = build_section_with("1.1.0", &commits, &r);
        assert!(s.contains("- **api:** add events endpoint"));
        assert!(s.contains("- **db:** add index"));
        assert!(s.contains("- top-level change"));
        assert!(!s.contains("### api"), "scopes must not become subheadings");
        let pos_scopeless = s.find("- top-level change").unwrap();
        let pos_api = s.find("- **api:**").unwrap();
        assert!(pos_scopeless < pos_api, "scopeless entries render first");
    }

    #[test]
    fn rich_render_commit_links_when_forge_known() {
        let cfg = ChangelogConfig {
            sections: Some(sections(&[("feat", enabled())])),
            include_commit_links: true,
            ..Default::default()
        };
        let commits = vec![GitLog {
            id: String::new(),
            hash: "abc1234".to_string(),
            message: "feat: add endpoint".to_string(),
        }];
        let r = ChangelogRender {
            formats: None,
            config: Some(&cfg),
            forge_base: Some("https://github.com/owner/repo".to_string()),
            ..Default::default()
        };
        let s = build_section_with("1.1.0", &commits, &r);
        assert!(
            s.contains("- add endpoint ([abc1234](https://github.com/owner/repo/commit/abc1234))")
        );
    }

    #[test]
    fn rich_render_commit_links_omitted_when_forge_unknown() {
        let cfg = ChangelogConfig {
            sections: Some(sections(&[("feat", enabled())])),
            include_commit_links: true,
            ..Default::default()
        };
        let commits = make_commits(&["feat: add endpoint"]);
        let r = ChangelogRender {
            formats: None,
            config: Some(&cfg),
            forge_base: None,
            ..Default::default()
        };
        let s = build_section_with("1.1.0", &commits, &r);
        assert!(s.contains("- add endpoint"));
        assert!(!s.contains("(["), "no commit link without a known forge");
    }

    #[test]
    fn rich_render_compare_link_when_last_tag_exists() {
        let cfg = ChangelogConfig {
            sections: Some(sections(&[("feat", enabled())])),
            include_compare_link: true,
            ..Default::default()
        };
        let commits = make_commits(&["feat: x"]);
        let r = ChangelogRender {
            formats: None,
            config: Some(&cfg),
            forge_base: Some("https://github.com/owner/repo".to_string()),
            last_tag: Some("v1.2.2".to_string()),
            new_tag: Some("v1.2.3".to_string()),
        };
        let s = build_section_with("1.2.3", &commits, &r);
        assert!(s.contains("[1.2.3]: https://github.com/owner/repo/compare/v1.2.2...v1.2.3"));
    }

    #[test]
    fn rich_render_compare_link_omitted_without_last_tag() {
        let cfg = ChangelogConfig {
            sections: Some(sections(&[("feat", enabled())])),
            include_compare_link: true,
            ..Default::default()
        };
        let commits = make_commits(&["feat: x"]);
        let r = ChangelogRender {
            formats: None,
            config: Some(&cfg),
            forge_base: Some("https://github.com/owner/repo".to_string()),
            last_tag: None,
            new_tag: Some("v1.0.0".to_string()),
        };
        let s = build_section_with("1.0.0", &commits, &r);
        assert!(!s.contains("/compare/"));
    }

    #[test]
    fn rich_render_omitted_sections_drop_commits() {
        let cfg = ChangelogConfig {
            sections: Some(sections(&[("feat", enabled())])),
            ..Default::default()
        };
        let commits = make_commits(&["feat: kept", "perf: dropped", "fix: dropped"]);
        let r = ChangelogRender {
            formats: None,
            config: Some(&cfg),
            ..Default::default()
        };
        let s = build_section_with("1.1.0", &commits, &r);
        assert!(s.contains("- kept"));
        assert!(!s.contains("### Performance"));
        assert!(!s.contains("### Bug Fixes"));
        assert!(!s.contains("dropped"));
    }

    #[test]
    fn rich_render_default_sections_match_feat_fix_only() {
        let cfg = ChangelogConfig::default();
        let commits = make_commits(&["feat: f", "fix: b", "perf: p", "docs: d"]);
        let r = ChangelogRender {
            formats: None,
            config: Some(&cfg),
            ..Default::default()
        };
        let s = build_section_with("1.1.0", &commits, &r);
        assert!(s.contains("### Features"));
        assert!(s.contains("### Bug Fixes"));
        assert!(!s.contains("### Performance"));
        assert!(!s.contains("### Documentation"));
    }
}
