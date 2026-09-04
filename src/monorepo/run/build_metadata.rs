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
    for command in commands {
        if dry_run {
            tracing::info!("  {} {}", "[buildMetadata]".dimmed(), command.dimmed());
            continue;
        }
        captured.insert(
            command.to_string(),
            crate::hooks::capture_build_metadata(command, root)?,
        );
    }
    Ok(captured)
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
        Some(metadata) => format!("{version}+{metadata}"),
        None => version.to_string(),
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
    fn a_dry_run_captures_nothing_and_leaves_versions_plain() {
        let config = config(Some("exit 1"), [r#"{"name":"a","path":"a"}"#].as_slice());

        let captured = capture(&config, &[0], &[bumped()], Path::new("."), true).unwrap();

        assert!(captured.is_empty());
        assert_eq!(
            stamp(&config, &config.packages[0], &captured, "1.4.0"),
            "1.4.0"
        );
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
