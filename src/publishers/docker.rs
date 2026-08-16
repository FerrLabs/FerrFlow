use anyhow::{Context, Result, anyhow};
use std::path::Path;
use std::process::Command;

use super::{PublishContext, PublishOutcome};
use crate::config::DockerSign;
use crate::error_code::{self, ErrorCodeExt};

#[allow(clippy::too_many_arguments)]
pub fn run(
    image: &str,
    tags: &[String],
    platforms: &[String],
    context_subdir: &str,
    dockerfile: &str,
    sign: DockerSign,
    extra_args: &[String],
    ctx: &PublishContext<'_>,
) -> Result<PublishOutcome> {
    let resolved_tags = expand_tags(tags, ctx.new_version);
    let image_refs: Vec<String> = resolved_tags
        .iter()
        .map(|t| format!("{image}:{t}"))
        .collect();

    if ctx.dry_run {
        return Ok(PublishOutcome::DryRun);
    }

    let probe = image_refs
        .iter()
        .all(|r| manifest_exists(r).unwrap_or(false));
    if probe {
        return Ok(PublishOutcome::Skipped {
            reason: format!(
                "all {} tag(s) for {image} already on the registry",
                image_refs.len()
            ),
        });
    }

    let context_path = ctx.package_path.join(context_subdir);
    if !context_path.exists() {
        return Err(anyhow!(
            "publisher docker: build context {} does not exist",
            context_path.display()
        ))
        .error_code(error_code::CONFIG_INVALID_PATH);
    }

    let mut cmd = Command::new("docker");
    cmd.arg("buildx").arg("build").arg("--push");
    if !platforms.is_empty() {
        cmd.arg(format!("--platform={}", platforms.join(",")));
    }
    cmd.arg(format!("--file={dockerfile}"));
    for r in &image_refs {
        cmd.arg("--tag").arg(r);
    }
    cmd.arg("--metadata-file")
        .arg(metadata_path(ctx.package_path));
    cmd.args(extra_args);
    cmd.arg(&context_path);

    let output = cmd.output().with_context(|| {
        format!(
            "spawn `docker buildx build` failed for {} (is docker buildx in PATH?)",
            ctx.package_name
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "docker buildx build failed for {}: {}",
            ctx.package_name,
            stderr.trim()
        ))
        .error_code(error_code::CONFIG_INVALID_PATH);
    }

    if matches!(sign, DockerSign::Sigstore) {
        let digest = read_manifest_digest(&metadata_path(ctx.package_path)).with_context(
            || "could not read manifest digest from buildx metadata for cosign signing",
        )?;
        let target = format!("{image}@{digest}");
        let cosign = Command::new("cosign")
            .arg("sign")
            .arg("--yes")
            .arg(&target)
            .output()
            .with_context(|| "spawn cosign failed (is cosign installed?)")?;
        if !cosign.status.success() {
            let stderr = String::from_utf8_lossy(&cosign.stderr);
            return Err(anyhow!(
                "cosign sign failed for {target}: {}",
                stderr.trim()
            ))
            .error_code(error_code::CONFIG_INVALID_PATH);
        }
    }

    let _ = std::fs::remove_file(metadata_path(ctx.package_path));

    Ok(PublishOutcome::Published {
        url: Some(image_refs[0].clone()),
    })
}

fn metadata_path(package_path: &Path) -> std::path::PathBuf {
    package_path.join(".ferrflow-buildx-metadata.json")
}

fn read_manifest_digest(path: &Path) -> Result<String> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let v: serde_json::Value =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    for key in ["containerimage.digest", "containerimage.config.digest"] {
        if let Some(s) = v.get(key).and_then(|x| x.as_str()) {
            return Ok(s.to_string());
        }
    }
    Err(anyhow!(
        "buildx metadata does not contain a containerimage.digest entry: {raw}"
    ))
}

fn manifest_exists(image_ref: &str) -> Result<bool> {
    let out = Command::new("docker")
        .arg("manifest")
        .arg("inspect")
        .arg(image_ref)
        .output()
        .with_context(|| format!("spawn docker manifest inspect {image_ref}"))?;
    Ok(out.status.success())
}

fn expand_tags(templates: &[String], new_version: &str) -> Vec<String> {
    let (major, minor) = split_major_minor(new_version);
    templates
        .iter()
        .map(|t| {
            let mut s = t.clone();
            s = s.replace("{version}", new_version);
            if let Some(m) = major {
                s = s.replace("{major}", m);
            }
            if let Some(m) = minor {
                s = s.replace("{minor}", m);
            }
            s
        })
        .collect()
}

fn split_major_minor(v: &str) -> (Option<&str>, Option<&str>) {
    let core = v
        .split_once('-')
        .map(|(c, _)| c)
        .unwrap_or(v)
        .split_once('+')
        .map(|(c, _)| c)
        .unwrap_or(v);
    let mut iter = core.split('.');
    let major = iter.next();
    let minor = iter.next();
    match (major, minor) {
        (Some(maj), Some(min)) => (Some(maj), Some(&core[..maj.len() + 1 + min.len()])),
        (Some(maj), None) => (Some(maj), None),
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RegistryConfig;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn ctx<'a>(
        registries: &'a BTreeMap<String, RegistryConfig>,
        dry_run: bool,
    ) -> PublishContext<'a> {
        let pkg_path = PathBuf::from(".");
        PublishContext {
            package_name: "auth",
            package_path: Box::leak(Box::new(pkg_path)),
            new_version: "1.2.3",
            tag: "auth@v1.2.3",
            registries,
            dry_run,
            verbose: false,
        }
    }

    #[test]
    fn dry_run_short_circuits() {
        let registries = BTreeMap::new();
        let outcome = run(
            "ghcr.io/x/auth",
            &["{version}".into()],
            &[],
            ".",
            "Dockerfile",
            DockerSign::None,
            &[],
            &ctx(&registries, true),
        )
        .expect("dry-run");
        assert!(matches!(outcome, PublishOutcome::DryRun));
    }

    #[test]
    fn expand_tags_resolves_version_major_minor() {
        let tags = expand_tags(
            &[
                "{version}".into(),
                "{major}".into(),
                "{minor}".into(),
                "latest".into(),
            ],
            "1.2.3",
        );
        assert_eq!(tags, vec!["1.2.3", "1", "1.2", "latest"]);
    }

    #[test]
    fn expand_tags_strips_prerelease_and_build_metadata_for_major_minor() {
        let tags = expand_tags(
            &["{version}".into(), "{major}".into(), "{minor}".into()],
            "1.2.3-beta.1+ci.42",
        );
        assert_eq!(tags[0], "1.2.3-beta.1+ci.42");
        assert_eq!(tags[1], "1");
        assert_eq!(tags[2], "1.2");
    }

    #[test]
    fn expand_tags_leaves_unknown_placeholders_intact() {
        let tags = expand_tags(&["{patch}".into()], "1.2.3");
        assert_eq!(tags, vec!["{patch}"]);
    }

    #[test]
    fn split_major_minor_calver_yields_partial() {
        let (maj, min) = split_major_minor("2026.06");
        assert_eq!(maj, Some("2026"));
        assert_eq!(min, Some("2026.06"));
    }

    #[test]
    fn split_major_minor_single_segment_yields_only_major() {
        let (maj, min) = split_major_minor("42");
        assert_eq!(maj, Some("42"));
        assert_eq!(min, None);
    }
}
