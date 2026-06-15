//! `helm package` + `helm push` executor for OCI registries.
//!
//! Behaviour:
//! - `helm package <chart-dir> -d <tmp>` to build the `<name>-<v>.tgz`.
//! - `helm push <tarball> <oci-registry>` to publish.
//! - Idempotent: when `helm show chart <registry>/<name>:<version>`
//!   succeeds, the chart is already pushed → skip.
//! - Auth: requires the caller to have done `helm registry login`
//!   beforehand. Same model as docker — FerrFlow doesn't touch the
//!   user's helm credentials store.

use anyhow::{Context, Result, anyhow};
use std::process::Command;

use super::{PublishContext, PublishOutcome};
use crate::error_code::{self, ErrorCodeExt};

pub fn run(chart_path: &str, registry: &str, ctx: &PublishContext<'_>) -> Result<PublishOutcome> {
    let chart_dir = ctx.package_path.join(chart_path);
    if !chart_dir.exists() {
        return Err(anyhow!(
            "publisher helm: chart directory {} does not exist",
            chart_dir.display()
        ))
        .error_code(error_code::CONFIG_INVALID_PATH);
    }
    let chart_name = read_chart_name(&chart_dir)?;

    let oci_ref = format!("{registry}/{chart_name}:{}", ctx.new_version);
    if ctx.dry_run {
        return Ok(PublishOutcome::DryRun);
    }

    if helm_chart_exists(&oci_ref) {
        return Ok(PublishOutcome::Skipped {
            reason: format!("{oci_ref} already on registry"),
        });
    }

    let tmp = tempfile::tempdir().context("create temp dir for helm package")?;
    let pkg = Command::new("helm")
        .arg("package")
        .arg(&chart_dir)
        .arg("--destination")
        .arg(tmp.path())
        .output()
        .with_context(|| "spawn `helm package` failed (is helm in PATH?)")?;
    if !pkg.status.success() {
        return Err(anyhow!(
            "helm package failed for {chart_name}: {}",
            String::from_utf8_lossy(&pkg.stderr).trim()
        ))
        .error_code(error_code::CONFIG_INVALID_PATH);
    }

    let tgz_name = format!("{chart_name}-{}.tgz", ctx.new_version);
    let tgz = tmp.path().join(&tgz_name);
    if !tgz.exists() {
        return Err(anyhow!(
            "expected {tgz_name} after helm package but it's not there"
        ))
        .error_code(error_code::CONFIG_INVALID_PATH);
    }

    let push = Command::new("helm")
        .arg("push")
        .arg(&tgz)
        .arg(registry)
        .output()
        .with_context(|| "spawn `helm push` failed")?;
    if !push.status.success() {
        let stderr = String::from_utf8_lossy(&push.stderr);
        if stderr.to_ascii_lowercase().contains("already exists") {
            return Ok(PublishOutcome::Skipped {
                reason: format!("{oci_ref} already exists (race with another publisher)"),
            });
        }
        return Err(anyhow!("helm push failed for {oci_ref}: {}", stderr.trim()))
            .error_code(error_code::CONFIG_INVALID_PATH);
    }

    Ok(PublishOutcome::Published { url: Some(oci_ref) })
}

fn read_chart_name(chart_dir: &std::path::Path) -> Result<String> {
    let chart_yaml = chart_dir.join("Chart.yaml");
    let raw = std::fs::read_to_string(&chart_yaml)
        .with_context(|| format!("read {}", chart_yaml.display()))?;
    for line in raw.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name:") {
            let value = rest.trim().trim_matches(|c| c == '"' || c == '\'');
            if !value.is_empty() {
                return Ok(value.to_string());
            }
        }
    }
    Err(anyhow!(
        "could not parse `name:` from {}",
        chart_yaml.display()
    ))
    .error_code(error_code::CONFIG_INVALID_PATH)
}

fn helm_chart_exists(oci_ref: &str) -> bool {
    Command::new("helm")
        .arg("show")
        .arg("chart")
        .arg(oci_ref)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RegistryConfig;
    use std::collections::BTreeMap;

    fn ctx<'a>(
        registries: &'a BTreeMap<String, RegistryConfig>,
        package_path: &'a std::path::Path,
        dry_run: bool,
    ) -> PublishContext<'a> {
        PublishContext {
            package_name: "ferrvault-operator",
            package_path,
            new_version: "1.2.3",
            tag: "ferrvault-operator@v1.2.3",
            registries,
            dry_run,
            verbose: false,
        }
    }

    #[test]
    fn dry_run_skips_after_chart_resolution() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("chart")).unwrap();
        std::fs::write(
            dir.path().join("chart").join("Chart.yaml"),
            "apiVersion: v2\nname: ferrvault-operator\nversion: 0.1.0\n",
        )
        .unwrap();
        let registries = BTreeMap::new();
        let c = ctx(&registries, dir.path(), true);
        let outcome =
            run("chart", "oci://ghcr.io/x/charts", &c).expect("dry-run after Chart.yaml load");
        assert!(matches!(outcome, PublishOutcome::DryRun));
    }

    #[test]
    fn missing_chart_dir_errors_clearly() {
        let dir = tempfile::tempdir().unwrap();
        let registries = BTreeMap::new();
        let c = ctx(&registries, dir.path(), true);
        let err = run("does-not-exist", "oci://ghcr.io/x", &c).expect_err("must error");
        assert!(format!("{err:?}").contains("does not exist"));
    }

    #[test]
    fn read_chart_name_strips_quotes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Chart.yaml"),
            "apiVersion: v2\nname: \"my-chart\"\nversion: 0.1.0\n",
        )
        .unwrap();
        assert_eq!(read_chart_name(dir.path()).unwrap(), "my-chart");
    }

    #[test]
    fn read_chart_name_errors_when_name_missing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Chart.yaml"),
            "apiVersion: v2\nversion: 0.1.0\n",
        )
        .unwrap();
        assert!(read_chart_name(dir.path()).is_err());
    }
}
