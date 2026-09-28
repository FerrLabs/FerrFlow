use std::collections::HashMap;
use std::path::Path;

use colored::Colorize;

use crate::config::{Config, PackageConfig, PropagatePolicy, VersionedFile};
use crate::conventional_commits::BumpType;
use crate::formats::dependents::{plan_dependency_update, supports_dependency_updates};
use crate::formats::{get_handler, read_version, write_version};
use crate::versioning::compute_next_version;

use super::super::types::CheckPackage;
use super::super::util::tags_for_package;
use super::super::version_source::VersionSource;
use super::release_json::ReleasedPackage;
use super::summary::PlannedTag;
use crate::changelog::update_changelog;

pub(super) struct CascadeSink<'a> {
    pub any_bumped: &'a mut bool,
    pub json_packages: &'a mut Vec<CheckPackage>,
    pub released: &'a mut Vec<ReleasedPackage>,
    pub files_to_commit: &'a mut Vec<String>,
    pub files_per_package: &'a mut HashMap<String, Vec<String>>,
    pub tags_to_create: &'a mut Vec<PlannedTag>,
    pub pkg_outputs: &'a mut Vec<(String, Vec<String>)>,
    pub bumped: &'a mut HashMap<String, BumpType>,
    pub bumped_versions: &'a mut HashMap<String, String>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_dependency_cascade(
    config: &Config,
    root: &Path,
    all_tags: &[String],
    channel: Option<&str>,
    json: bool,
    release_json: bool,
    dry_run: bool,
    sink: &mut CascadeSink<'_>,
    captured_metadata: &mut std::collections::HashMap<String, String>,
) -> anyhow::Result<()> {
    let settled = settle(config, sink.bumped);
    // These packages were not in `bump_order`, so `capture` never saw them and
    // their commands are missing from the map. Without this they released with
    // a plain version while a directly bumped sibling carried the suffix.
    // Restricted to packages the loop below can actually write. It still drops
    // a package on three further paths (unreadable current version, no next
    // version, version unchanged), which are only knowable by resolving the
    // whole settled set first; those keep the old behaviour of running the
    // command for a package that then writes nothing.
    let cascaded: Vec<usize> = settled
        .order
        .iter()
        .map(|(idx, _)| *idx)
        .filter(|idx| !config.packages[*idx].versioned_files.is_empty())
        .collect();
    super::build_metadata::capture_more(config, &cascaded, root, dry_run, captured_metadata)?;
    for (pkg_idx, bump) in settled.order {
        let Some(release) = resolve_cascaded(config, root, all_tags, &settled.state, pkg_idx, bump)
        else {
            continue;
        };
        if release_json {
            sink.released.push(release.released_package());
        }
        if json {
            sink.json_packages.push(release.check_package(channel));
        } else {
            let lines = cascaded_output(config, root, dry_run, sink, captured_metadata, &release)?;
            sink.pkg_outputs.push((release.pkg.name.clone(), lines));
        }
        release.record(config, sink);
    }
    Ok(())
}

struct CascadedRelease<'a> {
    pkg: &'a PackageConfig,
    vf: &'a VersionedFile,
    bump: BumpType,
    current_version: String,
    new_version: String,
    tag: String,
    dep_trigger: Vec<&'a str>,
}

fn resolve_cascaded<'a>(
    config: &'a Config,
    root: &Path,
    all_tags: &[String],
    state: &HashMap<String, BumpType>,
    pkg_idx: usize,
    bump: BumpType,
) -> Option<CascadedRelease<'a>> {
    let pkg = &config.packages[pkg_idx];
    let vf = pkg.versioned_files.first()?;
    let current_version = read_version(vf, root).ok()?;
    let pkg_tag_prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
    let strategy = pkg.effective_versioning(&config.workspace, || {
        tags_for_package(all_tags, &pkg_tag_prefix)
    });
    let version_template = pkg.effective_version_template(&config.workspace);
    let new_version =
        compute_next_version(&current_version, bump, strategy, version_template).ok()?;
    if current_version == new_version {
        return None;
    }
    let tag = pkg.tag_for_version(&config.workspace, config.is_monorepo(), &new_version);
    let dep_trigger = pkg
        .depends_on
        .iter()
        .filter(|dep| {
            state
                .get(dep.name())
                .is_some_and(|up| dep.propagate().resolve(*up) != BumpType::None)
        })
        .map(|dep| dep.name())
        .collect();
    Some(CascadedRelease {
        pkg,
        vf,
        bump,
        current_version,
        new_version,
        tag,
        dep_trigger,
    })
}

impl CascadedRelease<'_> {
    fn version_source(&self) -> Option<VersionSource> {
        Some(VersionSource::File {
            file: self.vf.path.clone(),
        })
    }

    fn released_package(&self) -> ReleasedPackage {
        ReleasedPackage {
            package: self.pkg.name.clone(),
            previous_version: self.current_version.clone(),
            new_version: self.new_version.clone(),
            bump_type: self.bump.to_string(),
            tag: self.tag.clone(),
            commit_count: 0,
            prerelease: false,
            version_source: self.version_source(),
            forge_release_url: None,
            forge_release_id: None,
        }
    }

    fn check_package(&self, channel: Option<&str>) -> CheckPackage {
        CheckPackage {
            name: self.pkg.name.clone(),
            current_version: self.current_version.clone(),
            next_version: self.new_version.clone(),
            bump_type: self.bump.to_string(),
            tag: self.tag.clone(),
            channel: channel.map(str::to_string),
            prerelease: false,
            version_source: self.version_source(),
            commits: vec![],
        }
    }

    fn record(self, config: &Config, sink: &mut CascadeSink<'_>) {
        let pkg = self.pkg;
        let body = format!("Dependency update: {}", self.dep_trigger.join(", "));
        sink.tags_to_create.push(PlannedTag {
            tag: self.tag,
            message: format!(
                "Release {}",
                pkg.tag_for_version(&config.workspace, config.is_monorepo(), &self.new_version)
            ),
            body,
            package: pkg.name.clone(),
            version: self.new_version.clone(),
            commit_count: 0,
            is_prerelease: false,
        });
        sink.bumped.insert(pkg.name.clone(), self.bump);
        sink.bumped_versions
            .insert(pkg.name.clone(), self.new_version);
        *sink.any_bumped = true;
    }
}

fn cascaded_output(
    config: &Config,
    root: &Path,
    dry_run: bool,
    sink: &mut CascadeSink<'_>,
    captured_metadata: &HashMap<String, String>,
    release: &CascadedRelease<'_>,
) -> anyhow::Result<Vec<String>> {
    let mut lines = vec![format!(
        "{} {}  {} → {}  ({}, dependency: {})",
        "●".green().bold(),
        release.pkg.name.bold(),
        release.current_version.dimmed(),
        release.new_version.green().bold(),
        release.bump.to_string().cyan(),
        release.dep_trigger.join(", ").cyan()
    )];
    if !dry_run {
        write_cascaded_files(config, root, sink, captured_metadata, release, &mut lines)?;
    }
    Ok(lines)
}

fn write_cascaded_files(
    config: &Config,
    root: &Path,
    sink: &mut CascadeSink<'_>,
    captured_metadata: &HashMap<String, String>,
    release: &CascadedRelease<'_>,
    lines: &mut Vec<String>,
) -> anyhow::Result<()> {
    let pkg = release.pkg;
    let stamped =
        super::build_metadata::stamp(config, pkg, captured_metadata, &release.new_version);
    for vf in &pkg.versioned_files {
        write_version(vf, root, &stamped)?;
        if get_handler(&vf.format).modifies_file() {
            lines.push(format!("  ✓ Updated {}", vf.path));
            stage_for(sink, &pkg.name, &vf.path);
        }
    }
    if let Some(changelog_rel) = &pkg.changelog {
        let changelog_path = root.join(changelog_rel);
        update_changelog(
            &changelog_path,
            &pkg.name,
            &release.new_version,
            &[],
            release.bump,
            false,
        )?;
        stage_for(sink, &pkg.name, changelog_rel);
    }
    if pkg.effective_update_lockfiles(&config.workspace) {
        super::refresh_lockfiles(
            pkg,
            root,
            sink.files_to_commit,
            sink.files_per_package.entry(pkg.name.clone()).or_default(),
        );
    }
    Ok(())
}

fn stage_for(sink: &mut CascadeSink<'_>, package: &str, path: &str) {
    sink.files_to_commit.push(path.to_string());
    sink.files_per_package
        .entry(package.to_string())
        .or_default()
        .push(path.to_string());
}

/// What the cascade adds: which packages, at which bump, in the order they
/// were resolved, plus the state the walk settled on.
struct Settled {
    order: Vec<(usize, BumpType)>,
    state: HashMap<String, BumpType>,
}

/// Which packages the cascade adds, and the bump each ends up with, decided
/// before anything is written.
///
/// A package fed by two edges of different strength must settle on the
/// strongest, which means revisiting it when a stronger bump arrives in a
/// later round. Doing that while writing files would give it two changelog
/// entries and two planned tags, so the fixpoint is reached first and each
/// package is acted on once.
///
/// The order is the order packages were reached, so a dependency is always
/// emitted before what depends on it. Sorting by array index instead would
/// let a config that declares a dependent first produce its release commit
/// and tag before the dependency it was bumped for.
///
/// Packages already bumped from their own commits are left alone: their
/// version files, changelog and tag were produced before the cascade ran.
fn settle(config: &Config, seeded: &HashMap<String, BumpType>) -> Settled {
    let mut state = seeded.clone();
    let mut order: Vec<usize> = Vec::new();
    let mut reached: HashMap<usize, BumpType> = HashMap::new();

    for _ in 0..config.packages.len().saturating_mul(4) {
        let mut moved: Vec<(usize, BumpType)> =
            super::graph::cascade_round(&config.packages, &state)
                .into_iter()
                .filter(|(idx, _)| !seeded.contains_key(&config.packages[*idx].name))
                .collect();
        if moved.is_empty() {
            break;
        }
        moved.sort_by_key(|(idx, _)| *idx);
        for (idx, bump) in moved {
            state.insert(config.packages[idx].name.clone(), bump);
            if reached.insert(idx, bump).is_none() {
                order.push(idx);
            }
        }
    }

    Settled {
        order: order.into_iter().map(|idx| (idx, reached[&idx])).collect(),
        state,
    }
}

pub(super) fn update_dependent_manifests(
    config: &Config,
    root: &Path,
    bumped_versions: &HashMap<String, String>,
    dry_run: bool,
    files_to_commit: &mut Vec<String>,
    files_per_package: &mut HashMap<String, Vec<String>>,
) -> anyhow::Result<Vec<String>> {
    let mut lines = Vec::new();

    for (pkg, dep_name, new_version) in propagating_edges(config, bumped_versions) {
        let rewritable = pkg
            .versioned_files
            .iter()
            .filter(|vf| supports_dependency_updates(&vf.format));
        for vf in rewritable {
            let Some(planned) = plan_dependency_update(vf, root, dep_name, new_version)? else {
                continue;
            };
            lines.push(format!(
                "  {} {} → {} in {}",
                "↳".dimmed(),
                dep_name.cyan(),
                new_version.green(),
                vf.path.dimmed()
            ));
            if dry_run {
                continue;
            }
            planned.apply()?;
            push_once(files_to_commit, &vf.path);
            push_once(
                files_per_package.entry(pkg.name.clone()).or_default(),
                &vf.path,
            );
        }
    }

    Ok(lines)
}

fn propagating_edges<'a>(
    config: &'a Config,
    bumped_versions: &'a HashMap<String, String>,
) -> impl Iterator<Item = (&'a PackageConfig, &'a str, &'a String)> {
    config.packages.iter().flat_map(move |pkg| {
        pkg.depends_on
            .iter()
            .filter(|dep| dep.propagate() != PropagatePolicy::None)
            .filter_map(move |dep| {
                bumped_versions
                    .get(dep.name())
                    .map(|new_version| (pkg, dep.name(), new_version))
            })
    })
}

fn push_once(files: &mut Vec<String>, path: &str) {
    if !files.iter().any(|f| f == path) {
        files.push(path.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for name in ["core", "cli", "docs"] {
            std::fs::create_dir(root.join(name)).unwrap();
        }
        std::fs::write(
            root.join("core/package.json"),
            "{\n  \"name\": \"core\",\n  \"version\": \"1.0.0\"\n}\n",
        )
        .unwrap();
        for name in ["cli", "docs"] {
            std::fs::write(
                root.join(name).join("package.json"),
                format!(
                    "{{\n  \"name\": \"{name}\",\n  \"version\": \"1.0.0\",\n  \"dependencies\": {{\n    \"core\": \"^1.0.0\"\n  }}\n}}\n"
                ),
            )
            .unwrap();
        }
        std::fs::write(
            root.join("ferrflow.json"),
            r#"{
  "package": [
    { "name": "core", "path": "core", "versionedFiles": [{ "path": "core/package.json", "format": "json" }] },
    { "name": "cli", "path": "cli", "dependsOn": ["core"],
      "versionedFiles": [{ "path": "cli/package.json", "format": "json" }] },
    { "name": "docs", "path": "docs", "dependsOn": [{ "name": "core", "propagate": "none" }],
      "versionedFiles": [{ "path": "docs/package.json", "format": "json" }] }
  ]
}
"#,
        )
        .unwrap();
        dir
    }

    fn rewrite(root: &Path, dry_run: bool) -> (Vec<String>, Vec<String>) {
        let config = Config::load(root, None).unwrap();
        let bumped_versions = HashMap::from([("core".to_string(), "2.0.0".to_string())]);
        let mut files_to_commit = Vec::new();
        let mut files_per_package = HashMap::new();
        let lines = update_dependent_manifests(
            &config,
            root,
            &bumped_versions,
            dry_run,
            &mut files_to_commit,
            &mut files_per_package,
        )
        .unwrap();
        (lines, files_to_commit)
    }

    fn templated_workspace() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for name in ["core", "cli"] {
            std::fs::create_dir(root.join(name)).unwrap();
        }
        std::fs::write(
            root.join("core/package.json"),
            "{
  \"name\": \"core\",
  \"version\": \"1.0.0\"
}
",
        )
        .unwrap();
        std::fs::write(
            root.join("cli/package.json"),
            "{
  \"name\": \"cli\",
  \"version\": \"1.0.0\",
  \"dependencies\": {
    \"core\": \"^1.0.0\"
  }
}
",
        )
        .unwrap();
        std::fs::write(
            root.join("ferrflow.json"),
            r#"{
  "package": [
    { "name": "core", "path": "core",
      "versionedFiles": [{ "path": "core/package.json", "format": "json" }] },
    { "name": "cli", "path": "cli", "dependsOn": [{ "name": "core" }],
      "versionTemplate": "{year}.{month}.{seq}",
      "versionedFiles": [{ "path": "cli/package.json", "format": "json" }] }
  ]
}
"#,
        )
        .unwrap();
        dir
    }

    fn metadata_workspace() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for name in ["core", "cli"] {
            std::fs::create_dir(root.join(name)).unwrap();
        }
        std::fs::write(
            root.join("core/package.json"),
            "{\n  \"name\": \"core\",\n  \"version\": \"1.0.0\"\n}\n",
        )
        .unwrap();
        std::fs::write(
            root.join("cli/package.json"),
            "{\n  \"name\": \"cli\",\n  \"version\": \"1.0.0\",\n  \"dependencies\": {\n    \"core\": \"^1.0.0\"\n  }\n}\n",
        )
        .unwrap();
        std::fs::write(
            root.join("ferrflow.json"),
            r#"{
  "workspace": { "buildMetadata": "echo 26.2-26.45" },
  "package": [
    { "name": "core", "path": "core",
      "versionedFiles": [{ "path": "core/package.json", "format": "json" }] },
    { "name": "cli", "path": "cli", "dependsOn": [{ "name": "core" }],
      "versionedFiles": [{ "path": "cli/package.json", "format": "json" }] }
  ]
}
"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn a_cascaded_package_is_stamped_but_its_tag_is_not() {
        let dir = metadata_workspace();
        let config = Config::load(dir.path(), None).unwrap();

        let mut any_bumped = false;
        let mut json_packages = Vec::new();
        let mut released = Vec::new();
        let mut files_to_commit = Vec::new();
        let mut files_per_package = HashMap::new();
        let mut tags_to_create = Vec::new();
        let mut pkg_outputs = Vec::new();
        let mut bumped = HashMap::from([("core".to_string(), BumpType::Minor)]);
        let mut bumped_versions = HashMap::from([("core".to_string(), "1.1.0".to_string())]);
        let mut sink = CascadeSink {
            any_bumped: &mut any_bumped,
            json_packages: &mut json_packages,
            released: &mut released,
            files_to_commit: &mut files_to_commit,
            files_per_package: &mut files_per_package,
            tags_to_create: &mut tags_to_create,
            pkg_outputs: &mut pkg_outputs,
            bumped: &mut bumped,
            bumped_versions: &mut bumped_versions,
        };

        run_dependency_cascade(
            &config,
            dir.path(),
            &[],
            None,
            false,
            false,
            false,
            &mut sink,
            &mut std::collections::HashMap::new(),
        )
        .unwrap();

        let written = std::fs::read_to_string(dir.path().join("cli/package.json")).unwrap();
        assert!(
            written.contains("+26.2-26.45"),
            "a cascaded package was written without its build metadata: {written}"
        );
        assert!(
            tags_to_create
                .iter()
                .all(|t| !t.tag.contains('+') && !t.version.contains('+')),
            "build metadata leaked into a tag: {:?}",
            tags_to_create.iter().map(|t| &t.tag).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_cascaded_package_uses_its_version_template() {
        let dir = templated_workspace();
        let config = Config::load(dir.path(), None).unwrap();

        let mut any_bumped = false;
        let mut json_packages = Vec::new();
        let mut released = Vec::new();
        let mut files_to_commit = Vec::new();
        let mut files_per_package = HashMap::new();
        let mut tags_to_create = Vec::new();
        let mut pkg_outputs = Vec::new();
        let mut bumped = HashMap::from([("core".to_string(), BumpType::Minor)]);
        let mut bumped_versions = HashMap::from([("core".to_string(), "1.1.0".to_string())]);
        let mut sink = CascadeSink {
            any_bumped: &mut any_bumped,
            json_packages: &mut json_packages,
            released: &mut released,
            files_to_commit: &mut files_to_commit,
            files_per_package: &mut files_per_package,
            tags_to_create: &mut tags_to_create,
            pkg_outputs: &mut pkg_outputs,
            bumped: &mut bumped,
            bumped_versions: &mut bumped_versions,
        };

        run_dependency_cascade(
            &config,
            dir.path(),
            &[],
            None,
            false,
            false,
            true,
            &mut sink,
            &mut std::collections::HashMap::new(),
        )
        .unwrap();

        let cli = bumped_versions.get("cli").expect("cli should be cascaded");
        let year = chrono::Utc::now().format("%Y").to_string();
        assert!(
            cli.starts_with(&format!("{year}.")),
            "cascaded package ignored its versionTemplate: got {cli}"
        );
    }

    #[test]
    fn a_dependent_that_opts_out_of_the_cascade_keeps_its_constraint() {
        let dir = workspace();
        let (lines, files) = rewrite(dir.path(), false);

        assert_eq!(files, vec!["cli/package.json".to_string()]);
        assert!(lines.iter().all(|l| !l.contains("docs")), "{lines:?}");
        assert!(
            std::fs::read_to_string(dir.path().join("docs/package.json"))
                .unwrap()
                .contains("\"core\": \"^1.0.0\"")
        );
        assert!(
            std::fs::read_to_string(dir.path().join("cli/package.json"))
                .unwrap()
                .contains("\"core\": \"^2.0.0\"")
        );
    }

    #[test]
    fn a_dry_run_reports_the_rewrite_without_performing_it() {
        let dir = workspace();
        let (lines, files) = rewrite(dir.path(), true);

        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("cli/package.json"), "{lines:?}");
        assert!(files.is_empty(), "a dry run stages nothing: {files:?}");
        assert!(
            std::fs::read_to_string(dir.path().join("cli/package.json"))
                .unwrap()
                .contains("\"core\": \"^1.0.0\""),
            "the manifest must be left alone"
        );
    }
}

#[cfg(test)]
mod settle_tests {
    use super::settle;
    use crate::config::Config;
    use crate::conventional_commits::BumpType;
    use std::collections::HashMap;

    fn settled(json: &str, seeded: &[(&str, BumpType)]) -> Vec<(String, BumpType)> {
        let config: Config = serde_json::from_str(json).expect("valid config");
        let seed: HashMap<String, BumpType> = seeded
            .iter()
            .map(|(name, bump)| ((*name).to_string(), *bump))
            .collect();
        settle(&config, &seed)
            .order
            .into_iter()
            .map(|(idx, bump)| (config.packages[idx].name.clone(), bump))
            .collect()
    }

    fn settled_state(json: &str, seeded: &[(&str, BumpType)]) -> HashMap<String, BumpType> {
        let config: Config = serde_json::from_str(json).expect("valid config");
        let seed: HashMap<String, BumpType> = seeded
            .iter()
            .map(|(name, bump)| ((*name).to_string(), *bump))
            .collect();
        settle(&config, &seed).state
    }

    const DEPENDENT_FIRST: &str = r#"{
        "package": [
            { "name": "leaf", "path": "leaf", "dependsOn": ["mid"] },
            { "name": "mid", "path": "mid", "dependsOn": ["shared"] },
            { "name": "shared", "path": "shared" }
        ]
    }"#;

    #[test]
    fn a_dependency_is_emitted_before_what_depends_on_it() {
        let out = settled(DEPENDENT_FIRST, &[("shared", BumpType::Minor)]);
        let names: Vec<&str> = out.iter().map(|(n, _)| n.as_str()).collect();

        assert_eq!(
            names,
            vec!["mid", "leaf"],
            "the config declares leaf first, but emitting in array order would tag and commit it before the dependency it was bumped for"
        );
    }

    #[test]
    fn the_settled_state_names_every_package_reached() {
        let state = settled_state(DEPENDENT_FIRST, &[("shared", BumpType::Minor)]);

        assert_eq!(state.get("mid"), Some(&BumpType::Minor));
        assert_eq!(
            state.get("leaf"),
            Some(&BumpType::Minor),
            "callers resolve which dependency triggered a bump from this, so a package missing here reports an empty trigger"
        );
    }

    const DIAMOND: &str = r#"{
        "package": [
            { "name": "shared", "path": "shared" },
            { "name": "api", "path": "api", "dependsOn": ["shared"] },
            { "name": "web", "path": "web",
              "dependsOn": [{ "name": "shared", "propagate": "patch" }, "api"] }
        ]
    }"#;

    #[test]
    fn a_package_fed_by_two_edges_settles_on_the_strongest() {
        let out = settled(DIAMOND, &[("shared", BumpType::Minor)]);

        let web = out
            .iter()
            .find(|(name, _)| name == "web")
            .expect("web is reached");
        assert_eq!(
            web.1,
            BumpType::Minor,
            "the patch edge reaches web first, the minor through api has to win: {out:?}"
        );
    }

    #[test]
    fn a_package_upgraded_across_rounds_is_still_acted_on_once() {
        let out = settled(DIAMOND, &[("shared", BumpType::Minor)]);

        assert_eq!(
            out.iter().filter(|(name, _)| name == "web").count(),
            1,
            "two entries would write two changelog sections and plan two tags: {out:?}"
        );
    }

    #[test]
    fn a_package_bumped_from_its_own_commits_is_left_to_the_main_loop() {
        let out = settled(
            DIAMOND,
            &[("shared", BumpType::Minor), ("web", BumpType::Patch)],
        );

        assert!(
            !out.iter().any(|(name, _)| name == "web"),
            "its files, changelog and tag were produced before the cascade ran: {out:?}"
        );
    }

    #[test]
    fn an_edge_that_declines_to_propagate_adds_nothing() {
        let out = settled(
            r#"{
                "package": [
                    { "name": "shared", "path": "shared" },
                    { "name": "docs", "path": "docs",
                      "dependsOn": [{ "name": "shared", "propagate": "none" }] }
                ]
            }"#,
            &[("shared", BumpType::Major)],
        );

        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn a_cycle_terminates_rather_than_spinning() {
        let out = settled(
            r#"{
                "package": [
                    { "name": "a", "path": "a", "dependsOn": ["b"] },
                    { "name": "b", "path": "b", "dependsOn": ["a"] }
                ]
            }"#,
            &[("a", BumpType::Minor)],
        );

        assert_eq!(out.len(), 1, "only b joins, and the walk stops: {out:?}");
    }
}
