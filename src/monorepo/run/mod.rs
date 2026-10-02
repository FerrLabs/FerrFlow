use anyhow::Result;
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::changelog::{ChangelogRender, GitLog, compute_changelog_update};
use crate::config::{Config, PackageConfig};
use crate::conventional_commits::BumpType;
use crate::diff::unified_diff;
use crate::formats::{get_handler, render_new_version};
use crate::hooks::HookPackage;
use crate::timing::Timing;

use super::types::RunOutput;

mod authored_commit;
mod build_metadata;
mod cascade;
pub(crate) mod checkpoint;
mod commit_body;
mod context;
mod drafts;
mod execute;
#[cfg(test)]
mod execute_tests;
mod finalize;
mod forced;
pub(crate) mod graph;
mod groups;
mod lock;
mod output;
mod package;
mod package_write;
mod plan;
mod publish;
mod release_json;
mod state;
mod summary;
mod upgrade;
mod why;
use context::{ReleaseInputs, RepoAccess, RunFlags};
use forced::parse_forced_versions;
use package::PackageRun;
use plan::PackagePlan;
use publish::Publisher;
use state::ReleaseState;
pub use why::why;

#[allow(clippy::too_many_arguments)]
pub(super) fn run_release_logic(
    root: &Path,
    config: &Config,
    dry_run: bool,
    verbose: bool,
    json: bool,
    release_json: bool,
    force: bool,
    force_versions: &[String],
    excluded: &[String],
    channel: Option<&str>,
    draft: bool,
    force_unlock: bool,
    timing: &mut Timing,
) -> Result<Option<RunOutput>> {
    let flags = RunFlags {
        dry_run,
        verbose,
        json,
        release_json,
        force,
        draft,
        shadow: false,
    };
    let access = RepoAccess {
        channel,
        force_unlock,
    };
    run_with_flags(
        root,
        config,
        flags,
        access,
        force_versions,
        excluded,
        timing,
    )
}

pub(crate) fn run_shadow_release(root: &Path, config: &Config, verbose: bool) -> Result<()> {
    let flags = RunFlags {
        dry_run: false,
        verbose,
        json: false,
        release_json: false,
        force: false,
        draft: false,
        shadow: true,
    };
    let access = RepoAccess {
        channel: None,
        force_unlock: false,
    };
    let mut timing = Timing::new(false);
    run_with_flags(root, config, flags, access, &[], &[], &mut timing).map(|_| ())
}

fn run_with_flags(
    root: &Path,
    config: &Config,
    flags: RunFlags,
    access: RepoAccess<'_>,
    force_versions: &[String],
    excluded: &[String],
    timing: &mut Timing,
) -> Result<Option<RunOutput>> {
    if config.packages.is_empty() {
        return finish(flags.dry_run, output::empty_config(flags)?);
    }

    if let Err(errors) = config.validate_groups() {
        return Err(anyhow::anyhow!(
            "invalid linked/fixed groups:\n  - {}",
            errors.join("\n  - ")
        ));
    }

    let release_order = graph::release_order(&config.packages).map_err(graph::Cycle::into_error)?;

    let inputs = ReleaseInputs::resolve(root, config, flags, access, timing)?;

    let finalize_tags = inputs.finalize_tags(root, config, flags.dry_run);
    let finalizing = !finalize_tags.is_empty();

    let forced = parse_forced_versions(force_versions, config.is_monorepo())?;

    let compute_start = std::time::Instant::now();
    let mut plans = inputs.compute_plans(root, config, &forced, excluded)?;
    let all_packages = batch_package_snapshot(&release_order, &plans, &config.packages);

    let mut state = ReleaseState::default();
    if finalizing {
        state.record_finalize_tags(finalize_tags);
    }
    let bump_order: &[usize] = if finalizing { &[] } else { &release_order };

    let mut captured_metadata =
        build_metadata::capture(config, bump_order, &plans, root, flags.dry_run)?;

    let package_run = PackageRun {
        config,
        root,
        flags,
        inputs: &inputs,
        all_packages: &all_packages,
        captured_metadata: &captured_metadata,
    };
    for &pkg_idx in bump_order {
        let plan = plans[pkg_idx]
            .take()
            .expect("release_order visits each package exactly once");
        package_run.process(&mut state, pkg_idx, plan)?;
    }

    propagate_bumps(
        config,
        root,
        &inputs,
        flags,
        &mut state,
        &mut captured_metadata,
    )?;

    timing.record("per-package compute", compute_start.elapsed());

    if flags.json {
        return finish(flags.dry_run, output::check(state)?);
    }

    let publisher = Publisher {
        repo: &inputs.repo,
        config,
        root,
        target_branch: &inputs.target_branch,
        flags,
        finalizing,
        all_packages: &all_packages,
    };
    publisher.publish(&mut state, timing)?;

    if flags.release_json {
        let out = output::release_json(&inputs.repo, state, &inputs.target_branch, flags.dry_run)?;
        return finish(flags.dry_run, out);
    }

    finish(flags.dry_run, output::text(state, config, flags))
}

fn propagate_bumps(
    config: &Config,
    root: &Path,
    inputs: &ReleaseInputs,
    flags: RunFlags,
    state: &mut ReleaseState,
    captured_metadata: &mut HashMap<String, String>,
) -> Result<()> {
    if config.is_monorepo() {
        cascade::run_dependency_cascade(
            config,
            root,
            &inputs.all_tags,
            inputs.prerelease_ctx.channel.as_deref(),
            flags.json,
            flags.release_json,
            flags.dry_run,
            &mut state.cascade_sink(),
            captured_metadata,
        )?;
    }

    if config.workspace.update_dependents {
        let rewritten = cascade::update_dependent_manifests(
            config,
            root,
            &state.bumped_versions,
            flags.dry_run,
            &mut state.files_to_commit,
            &mut state.files_per_package,
        )?;
        state.shared_outputs.extend(rewritten);
    }
    Ok(())
}

fn batch_package_snapshot(
    release_order: &[usize],
    plans: &[Option<PackagePlan>],
    packages: &[PackageConfig],
) -> Vec<HookPackage> {
    release_order
        .iter()
        .filter_map(|&idx| match plans[idx].as_ref() {
            Some(PackagePlan::Bump(bump)) => Some(HookPackage {
                name: packages[idx].name.clone(),
                version: bump.new_version.clone(),
                bump: bump.bump.to_string(),
            }),
            _ => None,
        })
        .collect()
}

fn emit_dry_run_diffs(
    pkg: &PackageConfig,
    root: &Path,
    new_version: &str,
    commits: &[GitLog],
    bump: BumpType,
    changelog_render: &ChangelogRender,
) {
    for (path, diff) in
        collect_dry_run_diffs(pkg, root, new_version, commits, bump, changelog_render)
    {
        println!("{}", path.bold());
        print!("{diff}");
        println!();
    }
}

pub(super) fn collect_dry_run_diffs(
    pkg: &PackageConfig,
    root: &Path,
    new_version: &str,
    commits: &[GitLog],
    bump: BumpType,
    changelog_render: &ChangelogRender,
) -> Vec<(String, String)> {
    let mut diffs = Vec::new();
    for vf in &pkg.versioned_files {
        if !get_handler(&vf.format).modifies_file() {
            continue;
        }
        let (Ok(old), Ok(new)) = (
            std::fs::read_to_string(root.join(&vf.path)),
            render_new_version(vf, root, new_version),
        ) else {
            continue;
        };
        let diff = unified_diff(&old, &new);
        if !diff.is_empty() {
            diffs.push((vf.path.clone(), diff));
        }
    }

    if let Some(changelog_rel) = &pkg.changelog {
        let changelog_path = root.join(changelog_rel);
        if let Ok(Some((old, new))) = compute_changelog_update(
            &changelog_path,
            &pkg.name,
            new_version,
            commits,
            bump,
            changelog_render,
        ) {
            let diff = unified_diff(&old, &new);
            if !diff.is_empty() {
                diffs.push((changelog_rel.clone(), diff));
            }
        }
    }
    diffs
}

fn finish(dry_run: bool, out: RunOutput) -> Result<Option<RunOutput>> {
    if dry_run {
        Ok(Some(out))
    } else {
        out.print();
        Ok(None)
    }
}

pub(super) fn refresh_lockfiles(
    pkg: &PackageConfig,
    root: &Path,
    files_to_commit: &mut Vec<String>,
    pkg_files: &mut Vec<String>,
) {
    use crate::formats::lockfiles::{self, UpdateOutcome};

    let mut handled: HashSet<String> = HashSet::new();
    for vf in &pkg.versioned_files {
        let outcome = match lockfiles::update_for_manifest(root, &vf.path) {
            Ok(outcome) => outcome,
            Err(err) => {
                tracing::warn!(package = %pkg.name, manifest = %vf.path, error = %err, "lockfile update skipped");
                continue;
            }
        };
        match outcome {
            UpdateOutcome::Updated { lockfile_rel } => {
                if handled.insert(lockfile_rel.clone()) {
                    tracing::info!(package = %pkg.name, lockfile = %lockfile_rel, "refreshed lockfile");
                    files_to_commit.push(lockfile_rel.clone());
                    pkg_files.push(lockfile_rel);
                }
            }
            UpdateOutcome::NotOnPath { program } => {
                tracing::warn!(package = %pkg.name, program = %program, "package manager not on PATH; lockfile left stale");
            }
            UpdateOutcome::Failed { program, detail } => {
                tracing::warn!(package = %pkg.name, program = %program, detail = %detail, "lockfile update failed; lockfile left stale");
            }
            UpdateOutcome::NoLockfile | UpdateOutcome::UnsupportedManifest => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conventional_commits::BumpType;
    use plan::{PackageBump, SkipReason};

    fn pkg(name: &str) -> PackageConfig {
        serde_json::from_str(&format!(r#"{{"name":"{name}","path":"{name}"}}"#)).unwrap()
    }

    fn bump_plan(new_version: &str, bump: BumpType) -> PackagePlan {
        PackagePlan::Bump(Box::new(PackageBump {
            recovered: false,
            current_version: "1.0.0".to_string(),
            new_version: new_version.to_string(),
            is_prerelease: false,
            last_tag: None,
            commits: Vec::new(),
            bump,
            strategy_label: bump.to_string(),
            tag: format!("v{new_version}"),
            version_source: None,
        }))
    }

    #[test]
    fn batch_snapshot_lists_only_bumped_packages_in_release_order() {
        let packages = vec![pkg("api"), pkg("web"), pkg("cli")];
        let plans = vec![
            Some(bump_plan("2.0.0", BumpType::Major)),
            Some(PackagePlan::Skipped {
                reason: SkipReason::NotTouched,
                recovered: false,
            }),
            Some(bump_plan("1.4.0", BumpType::Minor)),
        ];

        let snapshot = batch_package_snapshot(&[2, 0, 1], &plans, &packages);

        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].name, "cli");
        assert_eq!(snapshot[0].version, "1.4.0");
        assert_eq!(snapshot[0].bump, "minor");
        assert_eq!(snapshot[1].name, "api");
        assert_eq!(snapshot[1].version, "2.0.0");
        assert_eq!(snapshot[1].bump, "major");
        assert!(!snapshot.iter().any(|p| p.name == "web"));
    }
}
