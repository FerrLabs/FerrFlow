use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;

use crate::changelog::ChangelogRender;
use crate::config::{Config, PackageConfig};
use crate::formats::get_handler;
use crate::git::tag_exists;
use crate::hooks::{
    HookCommit, HookContext, HookFile, HookPackage, HookPoint, resolve_hook, resolve_policy,
    run_hook,
};
use crate::versioning::truncate_version;

use super::super::types::{CheckCommit, CheckPackage};
use super::context::{ReleaseInputs, RunFlags};
use super::emit_dry_run_diffs;
use super::plan::{PackageBump, PackagePlan};
use super::release_json::ReleasedPackage;
use super::state::ReleaseState;

pub(super) struct PackageRun<'a> {
    pub config: &'a Config,
    pub root: &'a Path,
    pub flags: RunFlags,
    pub inputs: &'a ReleaseInputs,
    pub all_packages: &'a [HookPackage],
    pub captured_metadata: &'a HashMap<String, String>,
}

impl PackageRun<'_> {
    pub fn process(
        &self,
        state: &mut ReleaseState,
        pkg_idx: usize,
        plan: PackagePlan,
    ) -> Result<()> {
        let pkg = &self.config.packages[pkg_idx];
        self.log_recovered(pkg, &plan);

        let bump = match plan {
            PackagePlan::Skipped { reason, .. } => {
                state.record_skip(&pkg.name, reason, self.flags);
                return Ok(());
            }
            PackagePlan::Bump(bump_plan) => *bump_plan,
        };

        if self.flags.release_json {
            state.released.push(released_package(pkg, &bump));
        }

        if self.flags.dry_run && self.flags.verbose && !self.flags.quiet() {
            emit_dry_run_diffs(
                pkg,
                self.root,
                &bump.new_version,
                &bump.commits,
                bump.bump,
                &self.changelog_render(&bump),
            );
        }

        if self.flags.json {
            state.json_packages.push(self.check_package(pkg, &bump));
        } else {
            state
                .pkg_outputs
                .push((pkg.name.clone(), self.summary_lines(pkg, &bump)));
        }

        let mut hook_ctx = self.hook_context(pkg, &bump);

        if self.flags.dry_run {
            self.run_dry_hooks(pkg, &hook_ctx)?;
        } else if tag_exists(&self.inputs.repo, &bump.tag) {
            state.push_package_line(
                &pkg.name,
                format!(
                    "  {} {} — tag {} already exists, skipping",
                    "○".dimmed(),
                    pkg.name.dimmed(),
                    bump.tag.cyan()
                ),
            );
            return Ok(());
        } else {
            self.write_release(pkg, &bump, &mut hook_ctx, state)?;
        }

        state.hook_contexts.push((hook_ctx, pkg_idx));
        state.bumped.insert(pkg.name.clone(), bump.bump);
        state
            .bumped_versions
            .insert(pkg.name.clone(), bump.new_version.clone());
        state.any_bumped = true;
        Ok(())
    }

    fn log_recovered(&self, pkg: &PackageConfig, plan: &PackagePlan) {
        let recovered = match plan {
            PackagePlan::Skipped { recovered, .. } => *recovered,
            PackagePlan::Bump(bump_plan) => bump_plan.recovered,
        };
        if recovered && !self.flags.quiet() {
            tracing::debug!(
                "{} {} — recovering missed release",
                "↻".cyan(),
                pkg.name.cyan()
            );
        }
    }

    pub(super) fn changelog_render(&self, bump: &PackageBump) -> ChangelogRender<'_> {
        ChangelogRender {
            formats: Some(&self.config.workspace.commit_formats),
            config: self.config.workspace.changelog.as_ref(),
            forge_base: self.inputs.forge_base.clone(),
            last_tag: bump.last_tag.clone(),
            new_tag: Some(bump.tag.clone()),
        }
    }

    fn check_package(&self, pkg: &PackageConfig, bump: &PackageBump) -> CheckPackage {
        let check_commits: Vec<CheckCommit> = bump
            .commits
            .iter()
            .filter_map(|c| {
                c.message.lines().next().map(|first_line| CheckCommit {
                    hash: c.hash.clone(),
                    message: first_line.to_string(),
                })
            })
            .collect();
        CheckPackage {
            name: pkg.name.clone(),
            current_version: bump.current_version.clone(),
            next_version: bump.new_version.clone(),
            bump_type: bump.strategy_label.clone(),
            tag: bump.tag.clone(),
            channel: self.inputs.prerelease_ctx.channel.clone(),
            prerelease: bump.is_prerelease,
            version_source: bump.version_source.clone(),
            commits: check_commits,
        }
    }

    fn summary_lines(&self, pkg: &PackageConfig, bump: &PackageBump) -> Vec<String> {
        let channel_label = if bump.is_prerelease {
            format!(
                " [{}]",
                self.inputs
                    .prerelease_ctx
                    .channel
                    .as_deref()
                    .unwrap_or("pre")
            )
        } else {
            String::new()
        };
        let source_label = bump
            .version_source
            .as_ref()
            .map(|source| format!(", {source}"))
            .unwrap_or_default();
        let mut lines = vec![format!(
            "{} {}  {} → {}  ({}{}{})",
            "●".green().bold(),
            pkg.name.bold(),
            bump.current_version.dimmed(),
            bump.new_version.green().bold(),
            bump.strategy_label.cyan(),
            source_label.dimmed(),
            channel_label.yellow()
        )];

        if self.flags.verbose {
            lines.extend(bump.commits.iter().filter_map(|c| {
                c.message
                    .lines()
                    .next()
                    .map(|line| format!("    {} {}", c.hash.dimmed(), line.dimmed()))
            }));
        }

        if !bump.is_prerelease {
            lines.extend(self.floating_tag_lines(pkg, &bump.new_version));
        }
        lines
    }

    fn floating_tag_lines(&self, pkg: &PackageConfig, new_version: &str) -> Vec<String> {
        let workspace = &self.config.workspace;
        pkg.effective_floating_tags(workspace)
            .iter()
            .filter_map(|level| truncate_version(new_version, *level))
            .map(|truncated| {
                let float_tag =
                    pkg.tag_for_version(workspace, self.config.is_monorepo(), &truncated);
                let verb = if tag_exists(&self.inputs.repo, &float_tag) {
                    "move"
                } else {
                    "create"
                };
                format!(
                    "    {} floating tag {}",
                    format!("→ {verb}").dimmed(),
                    float_tag.cyan()
                )
            })
            .collect()
    }

    fn hook_context(&self, pkg: &PackageConfig, bump: &PackageBump) -> HookContext {
        let commit_formats = &self.config.workspace.commit_formats;
        let hook_commits: Vec<HookCommit> = bump
            .commits
            .iter()
            .map(|c| HookCommit::from_commit(&c.hash, &c.message, commit_formats))
            .collect();
        let hook_bumped_files: Vec<HookFile> = pkg
            .versioned_files
            .iter()
            .filter(|vf| get_handler(&vf.format).modifies_file())
            .map(|vf| HookFile {
                path: vf.path.clone(),
                format: serde_json::to_value(&vf.format)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default(),
            })
            .collect();

        HookContext {
            package: pkg.name.clone(),
            old_version: bump.current_version.clone(),
            new_version: bump.new_version.clone(),
            bump_type: bump.bump.to_string(),
            tag: bump.tag.clone(),
            dry_run: self.flags.dry_run,
            package_path: self
                .root
                .join(pkg.path.trim_start_matches("./"))
                .to_string_lossy()
                .into_owned(),
            channel: self.inputs.prerelease_ctx.channel.clone(),
            error_code: None,
            monorepo: self.config.is_monorepo(),
            is_prerelease: bump.is_prerelease,
            changelog: String::new(),
            commits: hook_commits,
            bumped_files: hook_bumped_files,
            all_packages: self.all_packages.to_vec(),
            release_url: None,
        }
    }

    fn run_dry_hooks(&self, pkg: &PackageConfig, hook_ctx: &HookContext) -> Result<()> {
        if self.flags.json {
            return Ok(());
        }
        for point in [HookPoint::PreBump, HookPoint::PostBump] {
            if let Some(cmd) = self.package_hook(pkg, point) {
                self.run_package_hook(pkg, point, &cmd, hook_ctx, true)?;
            }
        }
        Ok(())
    }

    pub(super) fn package_hook(&self, pkg: &PackageConfig, point: HookPoint) -> Option<String> {
        resolve_hook(
            pkg.hooks.as_ref(),
            self.config.workspace.hooks.as_ref(),
            point,
        )
    }

    pub(super) fn run_package_hook(
        &self,
        pkg: &PackageConfig,
        point: HookPoint,
        cmd: &str,
        hook_ctx: &HookContext,
        dry_run: bool,
    ) -> Result<()> {
        let policy = resolve_policy(pkg.hooks.as_ref(), self.config.workspace.hooks.as_ref());
        run_hook(
            point,
            cmd,
            hook_ctx,
            policy,
            dry_run,
            self.flags.verbose,
            self.root,
        )
    }
}

fn released_package(pkg: &PackageConfig, bump: &PackageBump) -> ReleasedPackage {
    ReleasedPackage {
        package: pkg.name.clone(),
        previous_version: bump.current_version.clone(),
        new_version: bump.new_version.clone(),
        bump_type: bump.strategy_label.clone(),
        tag: bump.tag.clone(),
        commit_count: bump.commits.len(),
        prerelease: bump.is_prerelease,
        version_source: bump.version_source.clone(),
        forge_release_url: None,
        forge_release_id: None,
    }
}
