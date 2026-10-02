use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::{ReleaseCommitMode, ReleaseCommitScope};
use crate::forge::{Forge, MR_TITLE_MAX_CHARS};
use crate::git::{
    Remote, create_branch_and_commit, create_branch_and_commits, create_commit, force_push_branch,
    release_branch_foreign_commit,
};
use crate::hooks::HookPoint;
use crate::monorepo::preview::{build_forge_instance, try_build_forge_instance};

use super::{
    ReleasePlan, forge_unavailable, release_branch_name, release_pr_title, run_release_summary_hook,
};

pub(super) struct ReleaseCommit<'m> {
    pub mode: ReleaseCommitMode,
    pub scope: ReleaseCommitScope,
    pub file_refs: &'m [&'m str],
    pub message: &'m str,
    pub release_parts: &'m [String],
    pub skip_ci: &'m str,
}

pub(super) fn run_commit_or_pr(
    plan: &mut ReleasePlan<'_>,
    commit: &ReleaseCommit<'_>,
) -> Result<()> {
    match commit.mode {
        ReleaseCommitMode::Commit => commit_release(plan, commit),
        ReleaseCommitMode::Pr => propose_release(plan, commit),
        ReleaseCommitMode::None => Ok(()),
    }
}

fn authoring_forge(plan: &ReleasePlan<'_>) -> Option<Box<dyn crate::forge::Forge>> {
    if plan.shadow || !crate::bot_token::bot_mode_enabled() {
        return None;
    }
    let forge = build_forge_instance(plan.repo, plan.config)?;
    forge.authors_verified_commits().then_some(forge)
}

fn commits_per_package(plan: &ReleasePlan<'_>, scope: ReleaseCommitScope) -> bool {
    scope == ReleaseCommitScope::PerPackage && plan.tags_to_create.len() > 1
}

fn per_package_commits<'p>(
    plan: &'p ReleasePlan<'_>,
    skip_ci: &str,
) -> Vec<(Vec<&'p str>, String)> {
    plan.tags_to_create
        .iter()
        .filter_map(|tag| {
            plan.files_per_package.get(&tag.package).map(|pf| {
                let refs: Vec<&str> = pf.iter().map(String::as_str).collect();
                let msg = super::super::commit_body::build_commit_message(
                    &format!("chore(release): {} v{}{skip_ci}", tag.package, tag.version),
                    std::slice::from_ref(tag),
                    plan.config.workspace.release_commit_body,
                );
                (refs, msg)
            })
        })
        .collect()
}

fn commit_release(plan: &mut ReleasePlan<'_>, commit: &ReleaseCommit<'_>) -> Result<()> {
    if commits_per_package(plan, commit.scope) {
        for (refs, msg) in per_package_commits(plan, commit.skip_ci) {
            create_commit(plan.repo, &refs, &msg)?;
        }
        plan.shared_outputs
            .push("✓ Committed release changes (per-package)".to_string());
    } else if let Some(forge) = authoring_forge(plan) {
        let head = plan.repo.head_id()?.to_string();
        super::super::authored_commit::author_on_branch(
            forge.as_ref(),
            plan.repo,
            plan.root,
            Remote::of(&plan.config.workspace),
            plan.target_branch,
            &head,
            commit.file_refs,
            commit.message,
        )?;
        plan.shared_outputs
            .push("✓ Committed release changes as ferrflow[bot] (verified)".to_string());
    } else {
        create_commit(plan.repo, commit.file_refs, commit.message)?;
        plan.shared_outputs
            .push("✓ Committed release changes".to_string());
    }
    Ok(())
}

fn propose_release(plan: &mut ReleasePlan<'_>, commit: &ReleaseCommit<'_>) -> Result<()> {
    let branch_name = release_branch_name(plan.target_branch);
    let remote = Remote::of(&plan.config.workspace);

    if release_branch_is_foreign(plan, remote, &branch_name) {
        return Ok(());
    }

    let authored = commit_release_branch(plan, commit, remote, &branch_name)?;
    if !authored {
        force_push_branch(plan.repo, remote, &branch_name)?;
    }
    plan.shared_outputs
        .push(format!("✓ Pushed branch {}", branch_name.cyan()));

    match plan.forge {
        Some(forge) => open_or_update_release_mr(plan, forge, &branch_name, commit.release_parts),
        None => {
            let instance = try_build_forge_instance(plan.repo, plan.config)
                .map_err(|reason| forge_unavailable(&branch_name, reason))?;
            open_or_update_release_mr(plan, instance.as_ref(), &branch_name, commit.release_parts)
        }
    }
}

fn release_branch_is_foreign(
    plan: &ReleasePlan<'_>,
    remote: Remote<'_>,
    branch_name: &str,
) -> bool {
    match release_branch_foreign_commit(plan.repo, remote, branch_name, plan.target_branch) {
        Ok(Some(subject)) => {
            tracing::warn!(
                "{}",
                format!(
                    "  Warning: release branch {branch_name} has a commit FerrFlow didn't \
                     author (\"{subject}\"); leaving it and the existing release PR untouched."
                )
                .yellow()
            );
            true
        }
        Ok(None) => false,
        Err(err) => {
            tracing::warn!(
                "{}",
                format!("  Warning: could not inspect release branch {branch_name}: {err}")
                    .yellow()
            );
            false
        }
    }
}

fn commit_release_branch(
    plan: &mut ReleasePlan<'_>,
    commit: &ReleaseCommit<'_>,
    remote: Remote<'_>,
    branch_name: &str,
) -> Result<bool> {
    if commits_per_package(plan, commit.scope) {
        let commit_list = per_package_commits(plan, commit.skip_ci);
        let commit_refs: Vec<(&[&str], &str)> = commit_list
            .iter()
            .map(|(f, m)| (f.as_slice(), m.as_str()))
            .collect();
        create_branch_and_commits(plan.repo, branch_name, &commit_refs)?;
        return Ok(false);
    }
    let Some(forge) = authoring_forge(plan) else {
        create_branch_and_commit(plan.repo, branch_name, commit.file_refs, commit.message)?;
        return Ok(false);
    };
    // The branch has to exist before the mutation can commit onto
    // it, and it is recreated from the target branch on every run.
    let head = plan.repo.head_id()?.to_string();
    forge.set_branch(branch_name, &head)?;
    super::super::authored_commit::author_on_branch(
        forge.as_ref(),
        plan.repo,
        plan.root,
        remote,
        branch_name,
        &head,
        commit.file_refs,
        commit.message,
    )?;
    // author_on_branch leaves the checkout on the release branch;
    // the rest of the run expects the target branch.
    crate::git::reset_branch_to_remote(plan.repo, remote, plan.target_branch)?;
    Ok(true)
}

fn open_or_update_release_mr(
    plan: &mut ReleasePlan<'_>,
    forge: &dyn Forge,
    branch_name: &str,
    release_parts: &[String],
) -> Result<()> {
    let pr_title = release_pr_title(release_parts, MR_TITLE_MAX_CHARS);
    let pr_body = format!(
        "Automated release commit.\n\n{}",
        plan.tags_to_create
            .iter()
            .map(|t| format!("- `{}`", t.tag))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let existing = forge
        .find_open_pr(branch_name, plan.target_branch)
        .with_context(|| {
            format!(
                "could not look up the release {} for branch {branch_name}",
                forge.mr_noun()
            )
        })?;

    let (result, verb, action) = match existing {
        Some(id) => (
            forge.update_merge_request(id, &pr_title, &pr_body),
            "Updated",
            "update",
        ),
        None => (
            forge.create_merge_request(branch_name, plan.target_branch, &pr_title, &pr_body),
            "Created",
            "create",
        ),
    };

    match result {
        Ok(mr) => {
            plan.shared_outputs.push(format!(
                "✓ {verb} {} #{}",
                forge.mr_noun(),
                mr.id.to_string().cyan()
            ));
            run_release_summary_hook(plan, HookPoint::PreRelease)?;
            if plan.config.workspace.auto_merge_releases {
                match forge.enable_auto_merge(&mr) {
                    Ok(()) => plan.shared_outputs.push("✓ Auto-merge enabled".to_string()),
                    Err(err) => tracing::warn!(
                        "{}",
                        format!("  Warning: failed to enable auto-merge: {err}").yellow()
                    ),
                }
            }
        }
        Err(err) if !forge.supports_merge_requests() => tracing::warn!(
            "{}",
            format!(
                "  Warning: could not {action} the release {}: {err}",
                forge.mr_noun()
            )
            .yellow()
        ),
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "failed to {action} the release {} for branch {branch_name}",
                    forge.mr_noun()
                )
            });
        }
    }
    Ok(())
}
