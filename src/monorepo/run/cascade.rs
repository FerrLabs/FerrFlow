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
use super::release_json::ReleasedPackage;
use super::summary::PlannedTag;
use crate::changelog::update_changelog;

/// Mutable accumulators the cascade writes into, shared with the main
/// release pipeline. Bundled so the cascade keeps one parameter for its
/// output state instead of seven `&mut` arguments.
pub(super) struct CascadeSink<'a> {
    pub any_bumped: &'a mut bool,
    pub json_packages: &'a mut Vec<CheckPackage>,
    pub released: &'a mut Vec<ReleasedPackage>,
    pub files_to_commit: &'a mut Vec<String>,
    pub files_per_package: &'a mut HashMap<String, Vec<String>>,
    pub tags_to_create: &'a mut Vec<PlannedTag>,
    pub pkg_outputs: &'a mut Vec<(String, Vec<String>)>,
    /// Name -> the bump each already-released package received this run. The
    /// cascade reads it to know what to propagate, and extends it as it goes.
    pub bumped: &'a mut HashMap<String, BumpType>,
    /// Name -> the version each package landed on, consumed by the
    /// dependent-manifest rewrite once every bump is known.
    pub bumped_versions: &'a mut HashMap<String, String>,
}

/// Bump every package that depends (transitively) on a package
/// already bumped this run, propagating the upstream's bump through each
/// dependency's `PropagatePolicy`. Iterates to a fixed point, capped at
/// `packages.len()` rounds to break circular dependencies.
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
    let mut cascade_round = 0;
    loop {
        cascade_round += 1;
        if cascade_round > config.packages.len() {
            break; // safety: avoid infinite loops from circular deps
        }
        let mut new_bumps = Vec::new();
        for (pkg_idx, pkg) in config.packages.iter().enumerate() {
            if sink.bumped.contains_key(&pkg.name) {
                continue;
            }
            // Several dependencies may have moved at once under different
            // policies; the strongest resulting bump wins, the same way a
            // package's own commits resolve to their highest bump.
            let bump = pkg
                .depends_on
                .iter()
                .filter_map(|dep| {
                    let upstream = sink.bumped.get(dep.name())?;
                    Some(dep.propagate().resolve(*upstream))
                })
                .max()
                .unwrap_or(BumpType::None);
            if bump != BumpType::None {
                new_bumps.push((pkg_idx, bump));
            }
        }
        if new_bumps.is_empty() {
            break;
        }
        for (pkg_idx, bump) in new_bumps {
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
            let Ok(new_version) = compute_next_version(&current_version, bump, strategy) else {
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

/// Rewrites the constraints dependents declare for packages bumped this run.
///
/// Gated on `workspace.update_dependents` by the caller. Only manifests the
/// rewriter understands are touched; anything else is silently left alone, so
/// enabling this can never fail a release over a manifest shape we do not
/// model. A dry run plans and reports the same rewrites without writing them.
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
            // A `none` dependent is deliberately held back from the cascade, so
            // its manifest must stay on the constraint it declares today.
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

    // core is bumped; cli propagates it, docs opts out with `none`.
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

    // `propagate: "none"` holds the package back from the cascade, so its
    // manifest must keep declaring the version it was built against.
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
