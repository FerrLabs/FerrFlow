//! Upload a sidecar file to the GitHub Release that
//! `ferrflow release` just created.
//!
//! Uses `gh release upload --clobber` so a re-run replaces the asset
//! rather than failing — the upload is the load-bearing step and we'd
//! rather end up with the *new* file than refuse to write because the
//! *old* file is there. For users who want strict no-clobber, they
//! can do that with a `webhook` publisher and a CI step instead.

use anyhow::{Context, Result, anyhow};
use std::process::Command;

use super::{PublishContext, PublishOutcome};
use crate::error_code::{self, ErrorCodeExt};

pub fn run(
    path: &str,
    display_name: Option<&str>,
    ctx: &PublishContext<'_>,
) -> Result<PublishOutcome> {
    let asset_path = ctx.package_path.join(path);
    if !asset_path.exists() {
        return Err(anyhow!(
            "publisher github-release-asset: file {} does not exist",
            asset_path.display()
        ))
        .error_code(error_code::CONFIG_INVALID_PATH);
    }

    if ctx.dry_run {
        return Ok(PublishOutcome::DryRun);
    }

    let asset_arg = match display_name {
        Some(name) => format!("{}#{name}", asset_path.display()),
        None => asset_path.display().to_string(),
    };

    let output = Command::new("gh")
        .arg("release")
        .arg("upload")
        .arg(ctx.tag)
        .arg(&asset_arg)
        .arg("--clobber")
        .output()
        .with_context(|| "spawn `gh release upload` failed (is gh in PATH?)")?;
    if !output.status.success() {
        return Err(anyhow!(
            "gh release upload failed for {}: {}",
            asset_path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
        .error_code(error_code::CONFIG_INVALID_PATH);
    }

    Ok(PublishOutcome::Published {
        url: Some(format!("attached to release {}", ctx.tag)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn missing_file_errors_clearly() {
        let dir = tempfile::tempdir().unwrap();
        let registries = BTreeMap::new();
        let ctx = PublishContext {
            package_name: "ferrflow",
            package_path: dir.path(),
            new_version: "5.3.0",
            tag: "v5.3.0",
            registries: &registries,
            dry_run: true,
            verbose: false,
        };
        let err = run("sbom.cdx.json", None, &ctx).expect_err("must error");
        assert!(format!("{err:?}").contains("does not exist"));
    }

    #[test]
    fn dry_run_short_circuits_when_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sbom.cdx.json"), b"{}").unwrap();
        let registries = BTreeMap::new();
        let ctx = PublishContext {
            package_name: "ferrflow",
            package_path: dir.path(),
            new_version: "5.3.0",
            tag: "v5.3.0",
            registries: &registries,
            dry_run: true,
            verbose: false,
        };
        let outcome = run("sbom.cdx.json", Some("sbom.json"), &ctx).expect("dry-run");
        assert!(matches!(outcome, PublishOutcome::DryRun));
    }
}
