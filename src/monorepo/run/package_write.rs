use anyhow::Result;

use crate::changelog::{ChangelogRender, build_section_with, update_changelog_with};
use crate::config::PackageConfig;
use crate::formats::{get_handler, write_version};
use crate::hooks::{HookContext, HookPoint};

use super::super::util::{auto_stage_new_files, collect_dirty_files};
use super::build_metadata;
use super::package::PackageRun;
use super::plan::PackageBump;
use super::refresh_lockfiles;
use super::state::ReleaseState;
use super::summary::PlannedTag;

impl PackageRun<'_> {
    pub(super) fn write_release(
        &self,
        pkg: &PackageConfig,
        bump: &PackageBump,
        hook_ctx: &mut HookContext,
        state: &mut ReleaseState,
    ) -> Result<()> {
        if let Some(cmd) = self.package_hook(pkg, HookPoint::PreBump) {
            self.run_package_hook(pkg, HookPoint::PreBump, &cmd, hook_ctx, false)?;
        }

        self.write_versioned_files(pkg, &bump.new_version, state)?;

        if pkg.effective_update_lockfiles(&self.config.workspace) {
            refresh_lockfiles(
                pkg,
                self.root,
                &mut state.files_to_commit,
                state.files_per_package.entry(pkg.name.clone()).or_default(),
            );
        }

        let changelog_render = self.changelog_render(bump);
        let mut body = build_section_with(&bump.new_version, &bump.commits, &changelog_render);
        hook_ctx.changelog = body.clone();

        self.write_changelog(pkg, bump, &changelog_render, state)?;

        if let Some(cmd) = self.package_hook(pkg, HookPoint::PostBump) {
            self.run_post_bump_hook(pkg, &cmd, hook_ctx, state)?;
            body = crate::changelog::published_section(
                self.root,
                pkg.changelog.as_deref(),
                &bump.new_version,
                &body,
            );
        }

        state.tags_to_create.push(PlannedTag {
            tag: bump.tag.clone(),
            message: format!("Release {}", bump.tag),
            body,
            package: pkg.name.clone(),
            version: bump.new_version.clone(),
            commit_count: bump.commits.len() as i32,
            is_prerelease: bump.is_prerelease,
        });
        Ok(())
    }

    fn write_versioned_files(
        &self,
        pkg: &PackageConfig,
        new_version: &str,
        state: &mut ReleaseState,
    ) -> Result<()> {
        let stamped = build_metadata::stamp(self.config, pkg, self.captured_metadata, new_version);

        for vf in &pkg.versioned_files {
            write_version(vf, self.root, &stamped)?;
            if get_handler(&vf.format).modifies_file() {
                state.push_package_line(&pkg.name, format!("  ✓ Updated {}", vf.path));
                state.track_file(&pkg.name, &vf.path);
            }
        }
        Ok(())
    }

    fn write_changelog(
        &self,
        pkg: &PackageConfig,
        bump: &PackageBump,
        changelog_render: &ChangelogRender,
        state: &mut ReleaseState,
    ) -> Result<()> {
        let Some(changelog_rel) = &pkg.changelog else {
            return Ok(());
        };
        let changelog_path = self.root.join(changelog_rel);
        update_changelog_with(
            &changelog_path,
            &pkg.name,
            &bump.new_version,
            &bump.commits,
            bump.bump,
            false,
            changelog_render,
        )?;
        state.track_file(&pkg.name, changelog_rel);
        Ok(())
    }

    fn run_post_bump_hook(
        &self,
        pkg: &PackageConfig,
        cmd: &str,
        hook_ctx: &HookContext,
        state: &mut ReleaseState,
    ) -> Result<()> {
        let repo = &self.inputs.repo;
        let before = collect_dirty_files(repo);
        self.run_package_hook(pkg, HookPoint::PostBump, cmd, hook_ctx, false)?;
        let len_before = state.files_to_commit.len();
        auto_stage_new_files(repo, &before, &mut state.files_to_commit);
        let pkg_files = state.files_per_package.entry(pkg.name.clone()).or_default();
        for f in &state.files_to_commit[len_before..] {
            pkg_files.push(f.clone());
        }
        Ok(())
    }
}
