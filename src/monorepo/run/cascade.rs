use std::collections::HashMap;
use std::path::Path;

use colored::Colorize;

use crate::config::{Config, PropagatePolicy};
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
) -> anyhow::Result<()> {
    for (pkg_idx, bump) in settle(config, sink.bumped) {
        {
            let pkg = &config.packages[pkg_idx];
            let Some(vf) = pkg.versioned_files.first() else {
                continue;
            };
            let Ok(current_version) = read_version(vf, root) else {
                continue;
            };
            let pkg_tag_prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
            let strategy = pkg.effective_versioning(&config.workspace, || {
                tags_for_package(all_tags, &pkg_tag_prefix)
            });
            let version_template = pkg.effective_version_template(&config.workspace);
            let Ok(new_version) =
                compute_next_version(&current_version, bump, strategy, version_template)
            else {
                continue;
            };
            if current_version == new_version {
                continue;
            }
            let tag = pkg.tag_for_version(&config.workspace, config.is_monorepo(), &new_version);
            let dep_trigger: Vec<&str> = pkg
                .depends_on
                .iter()
                .map(|d| d.name())
                .filter(|name| sink.bumped.contains_key(*name))
                .collect();

            if release_json {
                sink.released.push(ReleasedPackage {
                    package: pkg.name.clone(),
                    previous_version: current_version.clone(),
                    new_version: new_version.clone(),
                    bump_type: bump.to_string(),
                    tag: tag.clone(),
                    commit_count: 0,
                    prerelease: false,
                    version_source: Some(VersionSource::File {
                        file: vf.path.clone(),
                    }),
                    forge_release_url: None,
                    forge_release_id: None,
                });
            }

            if json {
                sink.json_packages.push(CheckPackage {
                    name: pkg.name.clone(),
                    current_version: current_version.clone(),
                    next_version: new_version.clone(),
                    bump_type: bump.to_string(),
                    tag: tag.clone(),
                    channel: channel.map(str::to_string),
                    prerelease: false,
                    version_source: Some(VersionSource::File {
                        file: vf.path.clone(),
                    }),
                    commits: vec![],
                });
            } else {
                let mut lines = vec![format!(
                    "{} {}  {} → {}  ({}, dependency: {})",
                    "●".green().bold(),
                    pkg.name.bold(),
                    current_version.dimmed(),
                    new_version.green().bold(),
                    bump.to_string().cyan(),
                    dep_trigger.join(", ").cyan()
                )];
                if !dry_run {
                    for vf in &pkg.versioned_files {
                        write_version(vf, root, &new_version)?;
                        if get_handler(&vf.format).modifies_file() {
                            lines.push(format!("  ✓ Updated {}", vf.path));
                            sink.files_to_commit.push(vf.path.clone());
                            sink.files_per_package
                                .entry(pkg.name.clone())
                                .or_default()
                                .push(vf.path.clone());
                        }
                    }
                    if let Some(changelog_rel) = &pkg.changelog {
                        let changelog_path = root.join(changelog_rel);
                        update_changelog(
                            &changelog_path,
                            &pkg.name,
                            &new_version,
                            &[],
                            bump,
                            false,
                        )?;
                        sink.files_to_commit.push(changelog_rel.clone());
                        sink.files_per_package
                            .entry(pkg.name.clone())
                            .or_default()
                            .push(changelog_rel.clone());
                    }
                    if pkg.effective_update_lockfiles(&config.workspace) {
                        super::refresh_lockfiles(
                            pkg,
                            root,
                            sink.files_to_commit,
                            sink.files_per_package.entry(pkg.name.clone()).or_default(),
                        );
                    }
                }
                sink.pkg_outputs.push((pkg.name.clone(), lines));
            }
            let body = format!("Dependency update: {}", dep_trigger.join(", "));
            sink.tags_to_create.push(PlannedTag {
                tag,
                message: format!(
                    "Release {}",
                    pkg.tag_for_version(&config.workspace, config.is_monorepo(), &new_version)
                ),
                body,
                package: pkg.name.clone(),
                version: new_version.clone(),
                commit_count: 0,
                is_prerelease: false,
            });
            sink.bumped.insert(pkg.name.clone(), bump);
            sink.bumped_versions.insert(pkg.name.clone(), new_version);
            *sink.any_bumped = true;
        }
    }
    Ok(())
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
/// Packages already bumped from their own commits are left alone: their
/// version files, changelog and tag were produced before the cascade ran.
fn settle(config: &Config, seeded: &HashMap<String, BumpType>) -> Vec<(usize, BumpType)> {
    let mut state = seeded.clone();
    let mut added: HashMap<usize, BumpType> = HashMap::new();

    for _ in 0..config.packages.len().saturating_mul(4) {
        let moved: Vec<(usize, BumpType)> = super::graph::cascade_round(&config.packages, &state)
            .into_iter()
            .filter(|(idx, _)| !seeded.contains_key(&config.packages[*idx].name))
            .collect();
        if moved.is_empty() {
            break;
        }
        for (idx, bump) in moved {
            state.insert(config.packages[idx].name.clone(), bump);
            added.insert(idx, bump);
        }
    }

    let mut settled: Vec<(usize, BumpType)> = added.into_iter().collect();
    settled.sort_by_key(|(idx, _)| *idx);
    settled
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

    for pkg in &config.packages {
        for dep in &pkg.depends_on {
            if dep.propagate() == PropagatePolicy::None {
                continue;
            }
            let Some(new_version) = bumped_versions.get(dep.name()) else {
                continue;
            };
            for vf in &pkg.versioned_files {
                if !supports_dependency_updates(&vf.format) {
                    continue;
                }
                let Some(planned) = plan_dependency_update(vf, root, dep.name(), new_version)?
                else {
                    continue;
                };
                lines.push(format!(
                    "  {} {} → {} in {}",
                    "↳".dimmed(),
                    dep.name().cyan(),
                    new_version.green(),
                    vf.path.dimmed()
                ));
                if dry_run {
                    continue;
                }
                planned.apply()?;
                if !files_to_commit.contains(&vf.path) {
                    files_to_commit.push(vf.path.clone());
                }
                let owned = files_per_package.entry(pkg.name.clone()).or_default();
                if !owned.contains(&vf.path) {
                    owned.push(vf.path.clone());
                }
            }
        }
    }

    Ok(lines)
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
            .into_iter()
            .map(|(idx, bump)| (config.packages[idx].name.clone(), bump))
            .collect()
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
