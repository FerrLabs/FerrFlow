use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;

use crate::config::{BuildMetadata, Config, PackageConfig};

use super::plan::PackagePlan;

/// The command that stamps `pkg`, taking the package override when there is one
/// and falling back to the workspace command otherwise. `None` means this
/// package's versions stay plain.
pub fn resolve<'a>(config: &'a Config, pkg: &'a PackageConfig) -> Option<&'a str> {
    match &pkg.build_metadata {
        Some(BuildMetadata::Command(command)) => Some(command),
        Some(BuildMetadata::Enabled(false)) => None,
        Some(BuildMetadata::Enabled(true)) | None => config.workspace.build_metadata.as_deref(),
    }
}

/// Runs each distinct command once, before any version file is written, and
/// returns the output keyed by command. Only packages that actually bump are
/// consulted, so a skipped package's command neither runs nor can fail the run. A dry run prints the commands instead,
/// which is how hooks behave, so a rehearsal stays free of side effects.
pub fn capture(
    config: &Config,
    bump_order: &[usize],
    plans: &[Option<PackagePlan>],
    root: &Path,
    dry_run: bool,
) -> Result<HashMap<String, String>> {
    let mut commands: Vec<&str> = Vec::new();
    for &idx in bump_order {
        if !matches!(plans[idx], Some(PackagePlan::Bump(_))) {
            continue;
        }
        if let Some(command) = resolve(config, &config.packages[idx])
            && !commands.contains(&command)
        {
            commands.push(command);
        }
    }

    let mut captured = HashMap::new();
    capture_commands(&commands, root, dry_run, &mut captured)?;
    Ok(captured)
}

/// Fills `captured` with the output of any of `indices`' commands it does not
/// already hold. The dependency cascade settles after [`capture`] has run, so
/// a package bumped only because a dependency moved is unknown at that point
/// and its command has to be captured here instead.
pub fn capture_more(
    config: &Config,
    indices: &[usize],
    root: &Path,
    dry_run: bool,
    captured: &mut HashMap<String, String>,
) -> Result<()> {
    let mut commands: Vec<&str> = Vec::new();
    for &idx in indices {
        if let Some(command) = resolve(config, &config.packages[idx])
            && !captured.contains_key(command)
            && !commands.contains(&command)
        {
            commands.push(command);
        }
    }
    capture_commands(&commands, root, dry_run, captured)
}

fn capture_commands(
    commands: &[&str],
    root: &Path,
    dry_run: bool,
    captured: &mut HashMap<String, String>,
) -> Result<()> {
    for command in commands {
        if dry_run {
            // Nothing is captured in a rehearsal, so record the command with an
            // empty value: it dedupes the print when the cascade pass asks for
            // the same command, and `stamp` reads an empty value as no
            // metadata.
            if captured
                .insert((*command).to_string(), String::new())
                .is_none()
            {
                tracing::info!("  {} {}", "[buildMetadata]".dimmed(), command.dimmed());
            }
            continue;
        }
        captured.insert(
            (*command).to_string(),
            crate::hooks::capture_build_metadata(command, root)?,
        );
    }
    Ok(())
}

/// `version`, with the package's build metadata appended after a `+` when it
/// has any.
pub fn stamp(
    config: &Config,
    pkg: &PackageConfig,
    captured: &HashMap<String, String>,
    version: &str,
) -> String {
    match resolve(config, pkg).and_then(|command| captured.get(command)) {
        Some(metadata) if !metadata.is_empty() => format!("{version}+{metadata}"),
        _ => version.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::plan::{PackageBump, SkipReason};
    use super::*;
    use crate::conventional_commits::BumpType;

    fn bumped() -> Option<PackagePlan> {
        Some(PackagePlan::Bump(Box::new(PackageBump {
            recovered: false,
            current_version: "1.0.0".to_string(),
            new_version: "1.1.0".to_string(),
            is_prerelease: false,
            last_tag: None,
            commits: Vec::new(),
            bump: BumpType::Minor,
            strategy_label: BumpType::Minor.to_string(),
            tag: "v1.1.0".to_string(),
            version_source: None,
        })))
    }

    fn skipped() -> Option<PackagePlan> {
        Some(PackagePlan::Skipped {
            reason: SkipReason::NotTouched,
            recovered: false,
        })
    }

    fn config(workspace_command: Option<&str>, packages: &[&str]) -> Config {
        let mut config = Config::default();
        config.workspace.build_metadata = workspace_command.map(str::to_string);
        config.packages = packages
            .iter()
            .map(|json| serde_json::from_str(json).unwrap())
            .collect();
        config
    }

    #[test]
    fn package_false_opts_out_while_the_workspace_command_still_applies() {
        let config = config(
            Some("sh meta.sh"),
            [
                r#"{"name":"a","path":"a"}"#,
                r#"{"name":"b","path":"b","buildMetadata":false}"#,
            ]
            .as_slice(),
        );

        assert_eq!(resolve(&config, &config.packages[0]), Some("sh meta.sh"));
        assert_eq!(resolve(&config, &config.packages[1]), None);
    }

    #[test]
    fn package_command_overrides_the_workspace_command() {
        let config = config(
            Some("sh meta.sh"),
            [r#"{"name":"a","path":"a","buildMetadata":"sh other.sh"}"#].as_slice(),
        );

        assert_eq!(resolve(&config, &config.packages[0]), Some("sh other.sh"));
    }

    #[test]
    fn a_package_command_applies_without_a_workspace_one() {
        let config = config(
            None,
            [
                r#"{"name":"a","path":"a","buildMetadata":"sh only.sh"}"#,
                r#"{"name":"b","path":"b"}"#,
            ]
            .as_slice(),
        );

        assert_eq!(resolve(&config, &config.packages[0]), Some("sh only.sh"));
        assert_eq!(resolve(&config, &config.packages[1]), None);
    }

    #[test]
    fn stamp_appends_the_captured_output_of_the_resolved_command() {
        let config = config(
            Some("sh meta.sh"),
            [
                r#"{"name":"a","path":"a"}"#,
                r#"{"name":"b","path":"b","buildMetadata":false}"#,
                r#"{"name":"c","path":"c","buildMetadata":"sh other.sh"}"#,
            ]
            .as_slice(),
        );
        let captured = HashMap::from([
            ("sh meta.sh".to_string(), "26.2-26.45".to_string()),
            ("sh other.sh".to_string(), "c-only".to_string()),
        ]);

        assert_eq!(
            stamp(&config, &config.packages[0], &captured, "1.4.0"),
            "1.4.0+26.2-26.45"
        );
        assert_eq!(
            stamp(&config, &config.packages[1], &captured, "1.4.0"),
            "1.4.0"
        );
        assert_eq!(
            stamp(&config, &config.packages[2], &captured, "1.4.0"),
            "1.4.0+c-only"
        );
    }

    #[test]
    fn a_dry_run_records_the_command_without_stamping_anything() {
        let config = config(Some("exit 1"), [r#"{"name":"a","path":"a"}"#].as_slice());

        let mut captured = capture(&config, &[0], &[bumped()], Path::new("."), true).unwrap();

        // The command is recorded so the cascade pass does not print it a
        // second time, but with no value, so nothing is stamped.
        assert_eq!(captured.get("exit 1").map(String::as_str), Some(""));
        assert_eq!(
            stamp(&config, &config.packages[0], &captured, "1.4.0"),
            "1.4.0"
        );

        // And asking again prints nothing new.
        capture_more(&config, &[0], Path::new("."), true, &mut captured).unwrap();
        assert_eq!(captured.len(), 1);
    }

    #[test]
    fn nothing_runs_when_no_package_bumps() {
        let config = config(Some("exit 1"), [r#"{"name":"a","path":"a"}"#].as_slice());

        assert!(
            capture(&config, &[], &[bumped()], Path::new("."), false)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn capture_more_reuses_an_output_already_in_the_map() {
        let config = config(
            Some("echo fresh"),
            [r#"{"name":"a","path":"a"}"#].as_slice(),
        );
        let mut captured =
            HashMap::from([("echo fresh".to_string(), "from-the-main-loop".to_string())]);

        capture_more(&config, &[0], Path::new("."), false, &mut captured).unwrap();

        assert_eq!(captured["echo fresh"], "from-the-main-loop");
    }

    #[test]
    fn capture_more_runs_a_command_the_main_loop_never_saw() {
        let config = config(
            None,
            [r#"{"name":"a","path":"a","buildMetadata":"echo cascaded"}"#].as_slice(),
        );
        let mut captured = HashMap::new();

        capture_more(&config, &[0], Path::new("."), false, &mut captured).unwrap();

        assert_eq!(
            captured.get("echo cascaded").map(String::as_str),
            Some("cascaded")
        );
    }

    #[test]
    fn capture_more_skips_a_package_that_opted_out() {
        let config = config(
            Some("echo workspace"),
            [r#"{"name":"a","path":"a","buildMetadata":false}"#].as_slice(),
        );
        let mut captured = HashMap::new();

        capture_more(&config, &[0], Path::new("."), false, &mut captured).unwrap();

        assert!(captured.is_empty());
        assert_eq!(
            stamp(&config, &config.packages[0], &captured, "1.4.0"),
            "1.4.0"
        );
    }

    #[test]
    fn a_skipped_package_never_runs_its_command() {
        let config = config(
            None,
            [
                r#"{"name":"a","path":"a","buildMetadata":"exit 1"}"#,
                r#"{"name":"b","path":"b"}"#,
            ]
            .as_slice(),
        );

        let captured = capture(
            &config,
            &[0, 1],
            &[skipped(), bumped()],
            Path::new("."),
            false,
        )
        .unwrap();

        assert!(captured.is_empty());
    }

    #[test]
    fn package_true_inherits_the_workspace_command() {
        let config = config(
            Some("sh meta.sh"),
            [r#"{"name":"a","path":"a","buildMetadata":true}"#].as_slice(),
        );

        assert_eq!(resolve(&config, &config.packages[0]), Some("sh meta.sh"));
    }
}
