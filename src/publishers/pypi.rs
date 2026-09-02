use anyhow::{Context, Result, anyhow};
use std::process::Command;

use super::{PublishContext, PublishOutcome};
use crate::error_code::{self, ErrorCodeExt};

const MAX_PUBLISH_ATTEMPTS: u32 = 3;

const PUBLISH_RETRY_BACKOFF_SECS: [u64; 2] = [5, 15];

pub fn run(
    registry: Option<&str>,
    build: bool,
    trusted_publishing: bool,
    extra_args: &[String],
    ctx: &PublishContext<'_>,
) -> Result<PublishOutcome> {
    let registry_label = registry.unwrap_or("pypi.org");

    let resolved = match registry {
        Some(name) => {
            let r = ctx
                .registries
                .get(name)
                .ok_or_else(|| anyhow!(
                    "publisher pypi: registry `{name}` is not declared under `workspace.registries`"
                ))
                .error_code(error_code::CONFIG_INVALID_PATH)?;
            if let Some(env_name) = &r.token_env {
                if trusted_publishing {
                    return Err(anyhow!(
                        "publisher pypi:{name}: `trustedPublishing` and the registry `tokenEnv` \
                         (`{env_name}`) both configure authentication; keep one"
                    ))
                    .error_code(error_code::CONFIG_INVALID_PATH);
                }
                if std::env::var(env_name).is_err() {
                    return Err(anyhow!(
                        "publisher pypi:{name}: env var `{env_name}` is not set; \
                         export the registry token before running `ferrflow release`"
                    ))
                    .error_code(error_code::CONFIG_INVALID_PATH);
                }
            }
            Some(r)
        }
        None => None,
    };

    if ctx.dry_run {
        return Ok(PublishOutcome::DryRun);
    }

    if build {
        let output = Command::new("python")
            .current_dir(ctx.package_path)
            .args(["-m", "build"])
            .output()
            .with_context(|| {
                format!(
                    "spawn `python -m build` failed (is python with the `build` module in PATH?) for {}",
                    ctx.package_name
                )
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            return Err(anyhow!(
                "python -m build failed for {}: {}",
                ctx.package_name,
                first_meaningful_line(&stderr, &stdout)
            ))
            .error_code(error_code::CONFIG_INVALID_PATH);
        }
    }

    let mut cmd = Command::new("twine");
    cmd.current_dir(ctx.package_path).arg("upload");
    if let Some(url) = resolved.and_then(|r| r.url.as_deref()) {
        cmd.arg("--repository-url").arg(url);
    }
    if trusted_publishing {
        let token = super::pypi_oidc::mint(resolved.and_then(|r| r.url.as_deref()))?;
        cmd.env("TWINE_USERNAME", "__token__");
        cmd.env("TWINE_PASSWORD", token);
    } else if let Some(env_name) = resolved.and_then(|r| r.token_env.as_deref())
        && let Ok(token) = std::env::var(env_name)
    {
        cmd.env("TWINE_USERNAME", "__token__");
        cmd.env("TWINE_PASSWORD", token);
    }
    cmd.args(extra_args);
    cmd.arg("dist/*");

    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        let output = cmd.output().with_context(|| {
            format!(
                "spawn `twine upload` failed (is twine in PATH?) for {}",
                ctx.package_name
            )
        })?;
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

        if output.status.success() {
            return Ok(PublishOutcome::Published {
                url: derive_project_url(ctx.package_name, ctx.new_version, registry),
            });
        }

        if classify_already_published(&stderr, &stdout) {
            return Ok(PublishOutcome::Skipped {
                reason: format!(
                    "{}@{} already exists on {}",
                    ctx.package_name, ctx.new_version, registry_label
                ),
            });
        }

        if attempt < MAX_PUBLISH_ATTEMPTS && classify_transient(&stderr) {
            let backoff = PUBLISH_RETRY_BACKOFF_SECS
                .get((attempt - 1) as usize)
                .copied()
                .unwrap_or(15);
            tracing::warn!(
                "  ↻ {} upload hit a transient registry error on {} (attempt {attempt}/{MAX_PUBLISH_ATTEMPTS}); \
                 retrying in {backoff}s",
                ctx.package_name,
                registry_label
            );
            std::thread::sleep(std::time::Duration::from_secs(backoff));
            continue;
        }

        return Err(anyhow!(
            "twine upload failed for {} on {}: {}",
            ctx.package_name,
            registry_label,
            first_meaningful_line(&stderr, &stdout)
        ))
        .error_code(error_code::CONFIG_INVALID_PATH);
    }
}

fn classify_already_published(stderr: &str, stdout: &str) -> bool {
    let needles = [
        "file already exists",
        "already exists",
        "this filename has already been used",
        "conflict",
    ];
    let haystack = format!("{stderr}\n{stdout}").to_lowercase();
    needles.iter().any(|n| haystack.contains(n))
}

fn classify_transient(stderr: &str) -> bool {
    let needles = [
        "502 bad gateway",
        "503 service unavailable",
        "504 gateway timeout",
        "connection reset",
        "connection timed out",
        "temporarily unavailable",
    ];
    let haystack = stderr.to_lowercase();
    needles.iter().any(|n| haystack.contains(n))
}

fn first_meaningful_line(stderr: &str, stdout: &str) -> String {
    for src in [stderr, stdout] {
        for line in src.lines().rev() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("\u{1b}[") {
                continue;
            }
            if trimmed.starts_with("ERROR") || trimmed.starts_with("error") {
                return trimmed.to_string();
            }
        }
    }
    stderr
        .lines()
        .rfind(|l| !l.trim().is_empty())
        .unwrap_or("(no output)")
        .to_string()
}

fn derive_project_url(name: &str, version: &str, registry: Option<&str>) -> Option<String> {
    registry
        .is_none()
        .then(|| format!("https://pypi.org/project/{name}/{version}/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RegistryConfig;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn ctx(registries: &BTreeMap<String, RegistryConfig>, dry_run: bool) -> PublishContext<'_> {
        PublishContext {
            package_name: "ferrlabs-sdk",
            package_path: Box::leak(Box::new(PathBuf::from("."))),
            new_version: "0.1.0",
            tag: "ferrlabs-sdk@v0.1.0",
            registries,
            dry_run,
            verbose: false,
        }
    }

    #[test]
    fn missing_registry_definition_is_a_clear_error() {
        let registries = BTreeMap::new();
        let err = run(Some("internal"), false, false, &[], &ctx(&registries, true))
            .expect_err("must error");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("not declared under `workspace.registries`"),
            "expected helpful diagnostic, got: {msg}"
        );
    }

    #[test]
    fn missing_token_env_var_blocks_before_invoking_twine() {
        let mut registries = BTreeMap::new();
        registries.insert(
            "internal".to_string(),
            RegistryConfig {
                url: Some("https://pypi.internal/simple".to_string()),
                token_env: Some("A_TOKEN_THAT_IS_NOT_SET_XYZ".to_string()),
            },
        );
        let err = run(Some("internal"), false, false, &[], &ctx(&registries, true))
            .expect_err("must error");
        let msg = format!("{err:?}");
        assert!(msg.contains("is not set"), "got: {msg}");
    }

    #[test]
    fn trusted_publishing_and_a_registry_token_together_are_refused() {
        let mut registries = BTreeMap::new();
        registries.insert(
            "internal".to_string(),
            RegistryConfig {
                url: Some("https://pypi.internal/simple".to_string()),
                token_env: Some("A_TOKEN_THAT_IS_NOT_SET_XYZ".to_string()),
            },
        );
        let err = run(Some("internal"), false, true, &[], &ctx(&registries, true))
            .expect_err("must error");
        let msg = format!("{err:?}");
        assert!(msg.contains("both configure authentication"), "got: {msg}");
    }

    #[test]
    fn dry_run_short_circuits_after_validation() {
        let registries = BTreeMap::new();
        let outcome = run(None, true, false, &[], &ctx(&registries, true)).expect("dry run is ok");
        assert!(matches!(outcome, PublishOutcome::DryRun));
    }

    #[test]
    fn an_existing_file_on_the_index_reads_as_already_published() {
        let stderr = "ERROR    HTTPError: 400 Bad Request from https://upload.pypi.org/legacy/\n\
                      File already exists. See https://pypi.org/help/#file-name-reuse";
        assert!(classify_already_published(stderr, ""));
        assert!(!classify_already_published("ERROR 401 Unauthorized", ""));
    }

    #[test]
    fn a_gateway_error_is_transient_but_an_auth_failure_is_not() {
        assert!(classify_transient("503 Service Unavailable"));
        assert!(!classify_transient(
            "403 Forbidden: invalid or non-existent authentication"
        ));
    }

    #[test]
    fn the_project_url_is_only_emitted_for_the_default_index() {
        assert_eq!(
            derive_project_url("ferrlabs-sdk", "1.0.0", None).as_deref(),
            Some("https://pypi.org/project/ferrlabs-sdk/1.0.0/")
        );
        assert_eq!(
            derive_project_url("ferrlabs-sdk", "1.0.0", Some("internal")),
            None
        );
    }
}
