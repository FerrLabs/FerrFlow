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
        println!(
            "  {} {} {}",
            "⊙".dimmed(),
            format!("[{}]", point.label()).dimmed(),
            command.dimmed()
        );
        return Ok(());
    }

    println!(
        "  {} {} {}",
        "▸".cyan(),
        format!("[{}]", point.label()).cyan(),
        command
    );

    let mut cmd = build_command(command);
    cmd.current_dir(working_dir)
        .env("FERRFLOW_PACKAGE", &ctx.package)
        .env("FERRFLOW_OLD_VERSION", &ctx.old_version)
        .env("FERRFLOW_NEW_VERSION", &ctx.new_version)
        .env("FERRFLOW_BUMP_TYPE", &ctx.bump_type)
        .env("FERRFLOW_TAG", &ctx.tag)
        .env("FERRFLOW_DRY_RUN", ctx.dry_run.to_string())
        .env("FERRFLOW_PACKAGE_PATH", &ctx.package_path)
        .env("FERRFLOW_CHANNEL", ctx.channel.as_deref().unwrap_or(""))
        .env("FERRFLOW_IS_PRERELEASE", ctx.channel.is_some().to_string());

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
            eprintln!(
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
