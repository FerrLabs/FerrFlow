use std::collections::HashMap;
use std::path::Path;

use colored::Colorize;

use crate::config::Config;
use crate::conventional_commits::BumpType;
use crate::formats::{get_handler, read_version, write_version};
use crate::versioning::compute_next_version;

use super::super::types::CheckPackage;
use super::super::util::tags_for_package;
use super::release_json::ReleasedPackage;
use super::summary::TagToCreate;
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
    pub tags_to_create: &'a mut Vec<TagToCreate>,
    pub pkg_outputs: &'a mut Vec<(String, Vec<String>)>,
    /// Name -> the bump each already-released package received this run. The
    /// cascade reads it to know what to propagate, and extends it as it goes.
    pub bumped: &'a mut HashMap<String, BumpType>,
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
            sink.tags_to_create.push((
                tag,
                format!(
                    "Release {}",
                    pkg.tag_for_version(&config.workspace, config.is_monorepo(), &new_version)
                ),
                body,
                pkg.name.clone(),
                new_version,
                0,
                false,
            ));
            sink.bumped.insert(pkg.name.clone(), bump);
            *sink.any_bumped = true;
        }
    }
    Ok(())
}
