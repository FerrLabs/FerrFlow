use crate::config::{HooksConfig, OnFailure};
use anyhow::{Result, bail};
use colored::Colorize;
use std::path::Path;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Hook points
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum HookPoint {
    PreBump,
    PostBump,
    PreCommit,
    PrePublish,
    PostPublish,
}

impl HookPoint {
    pub fn label(self) -> &'static str {
        match self {
            Self::PreBump => "pre_bump",
            Self::PostBump => "post_bump",
            Self::PreCommit => "pre_commit",
            Self::PrePublish => "pre_publish",
            Self::PostPublish => "post_publish",
        }
    }
}

// ---------------------------------------------------------------------------
// Hook context (environment variables)
// ---------------------------------------------------------------------------

pub struct HookContext {
    pub package: String,
    pub old_version: String,
    pub new_version: String,
    pub bump_type: String,
    pub tag: String,
    pub dry_run: bool,
    pub package_path: String,
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

pub fn resolve_hook(
    pkg_hooks: Option<&HooksConfig>,
    ws_hooks: Option<&HooksConfig>,
    point: HookPoint,
) -> Option<String> {
    fn get(h: &HooksConfig, point: HookPoint) -> Option<&String> {
        match point {
            HookPoint::PreBump => h.pre_bump.as_ref(),
            HookPoint::PostBump => h.post_bump.as_ref(),
            HookPoint::PreCommit => h.pre_commit.as_ref(),
            HookPoint::PrePublish => h.pre_publish.as_ref(),
            HookPoint::PostPublish => h.post_publish.as_ref(),
        }
    }

    if let Some(pkg) = pkg_hooks
        && let Some(cmd) = get(pkg, point)
    {
        return Some(cmd.clone());
    }

    if let Some(ws) = ws_hooks
        && let Some(cmd) = get(ws, point)
    {
        return Some(cmd.clone());
    }

    None
}

pub fn resolve_on_failure(
    pkg_hooks: Option<&HooksConfig>,
    ws_hooks: Option<&HooksConfig>,
) -> OnFailure {
    if let Some(pkg) = pkg_hooks
        && let Some(v) = pkg.on_failure
    {
        return v;
    }
    if let Some(ws) = ws_hooks
        && let Some(v) = ws.on_failure
    {
        return v;
    }
    OnFailure::Abort
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

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
        .env("FERRFLOW_PACKAGE_PATH", &ctx.package_path);

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
        OnFailure::Abort => {
            bail!(
                "hook [{}] failed (exit {}): {}",
                point.label(),
                code_str,
                command
            );
        }
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ws_hooks(pre_bump: Option<&str>, post_publish: Option<&str>) -> HooksConfig {
        HooksConfig {
            pre_bump: pre_bump.map(String::from),
            post_publish: post_publish.map(String::from),
            ..Default::default()
        }
    }

    #[test]
    fn resolve_falls_back_to_workspace() {
        let ws = ws_hooks(Some("echo ws"), None);
        let result = resolve_hook(None, Some(&ws), HookPoint::PreBump);
        assert_eq!(result.as_deref(), Some("echo ws"));
    }

    #[test]
    fn resolve_package_overrides_workspace() {
        let ws = ws_hooks(Some("echo ws"), None);
        let pkg = HooksConfig {
            pre_bump: Some("echo pkg".into()),
            ..Default::default()
        };
        let result = resolve_hook(Some(&pkg), Some(&ws), HookPoint::PreBump);
        assert_eq!(result.as_deref(), Some("echo pkg"));
    }

    #[test]
    fn resolve_returns_none_when_unset() {
        let ws = ws_hooks(Some("echo ws"), None);
        let result = resolve_hook(None, Some(&ws), HookPoint::PostBump);
        assert!(result.is_none());
    }

    #[test]
    fn resolve_no_hooks_at_all() {
        let result = resolve_hook(None, None, HookPoint::PreBump);
        assert!(result.is_none());
    }

    #[test]
    fn on_failure_defaults_to_abort() {
        assert_eq!(resolve_on_failure(None, None), OnFailure::Abort);
    }

    #[test]
    fn on_failure_inherits_workspace() {
        let ws = HooksConfig {
            on_failure: Some(OnFailure::Continue),
            ..Default::default()
        };
        assert_eq!(resolve_on_failure(None, Some(&ws)), OnFailure::Continue);
    }

    #[test]
    fn on_failure_package_overrides_workspace() {
        let ws = HooksConfig {
            on_failure: Some(OnFailure::Continue),
            ..Default::default()
        };
        let pkg = HooksConfig {
            on_failure: Some(OnFailure::Abort),
            ..Default::default()
        };
        assert_eq!(resolve_on_failure(Some(&pkg), Some(&ws)), OnFailure::Abort);
    }

    #[test]
    fn handle_failure_abort_returns_error() {
        let result = handle_failure(HookPoint::PreBump, "echo fail", Some(1), OnFailure::Abort);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
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
        assert!(result.unwrap_err().to_string().contains("signal"));
    }

    #[test]
    fn hook_point_labels() {
        assert_eq!(HookPoint::PreBump.label(), "pre_bump");
        assert_eq!(HookPoint::PostBump.label(), "post_bump");
        assert_eq!(HookPoint::PreCommit.label(), "pre_commit");
        assert_eq!(HookPoint::PrePublish.label(), "pre_publish");
        assert_eq!(HookPoint::PostPublish.label(), "post_publish");
    }

    #[test]
    fn resolve_all_hook_points() {
        let hooks = HooksConfig {
            pre_bump: Some("a".into()),
            post_bump: Some("b".into()),
            pre_commit: Some("c".into()),
            pre_publish: Some("d".into()),
            post_publish: Some("e".into()),
            on_failure: None,
        };
        assert_eq!(
            resolve_hook(Some(&hooks), None, HookPoint::PreBump).as_deref(),
            Some("a")
        );
        assert_eq!(
            resolve_hook(Some(&hooks), None, HookPoint::PostBump).as_deref(),
            Some("b")
        );
        assert_eq!(
            resolve_hook(Some(&hooks), None, HookPoint::PreCommit).as_deref(),
            Some("c")
        );
        assert_eq!(
            resolve_hook(Some(&hooks), None, HookPoint::PrePublish).as_deref(),
            Some("d")
        );
        assert_eq!(
            resolve_hook(Some(&hooks), None, HookPoint::PostPublish).as_deref(),
            Some("e")
        );
    }
}
