//! `npm publish` executor.
//!
//! Behaviour:
//! - Validates the registry's `tokenEnv` is exported before spawning
//!   npm — fails fast with a clearer message than npm's "401
//!   Unauthorized" deep inside the publish flow.
//! - Writes a transient `.npmrc` line into the package directory that
//!   maps `<registry-host>` to the token, so existing user `.npmrc`
//!   files (project + global) are not modified. Removed on exit even
//!   on failure.
//! - Idempotent: npm registries reject re-publishing a version with a
//!   distinct error ("cannot publish over the previously published
//!   versions") that we recognize and return as a successful skip.
//! - Public-registry URL surfaces in the step summary; for GitHub
//!   Packages and private mirrors we leave the URL empty (template
//!   would be wrong as often as right).

use anyhow::{Context, Result, anyhow};
use std::path::Path;
use std::process::Command;

use super::{PublishContext, PublishOutcome};
use crate::error_code::{self, ErrorCodeExt};

pub fn run(
    registry: Option<&str>,
    tag: Option<&str>,
    access: Option<&str>,
    extra_args: &[String],
    ctx: &PublishContext<'_>,
) -> Result<PublishOutcome> {
    let registry_label = registry.unwrap_or("npmjs.org");

    let resolved_registry = match registry {
        Some(name) => {
            let r = ctx
                .registries
                .get(name)
                .ok_or_else(|| anyhow!(
                    "publisher npm: registry `{name}` is not declared under `workspace.registries`"
                ))
                .error_code(error_code::CONFIG_INVALID_PATH)?;
            if let Some(env_name) = &r.token_env
                && std::env::var(env_name).is_err()
            {
                return Err(anyhow!(
                    "publisher npm:{name}: env var `{env_name}` is not set; \
                     export the registry token before running `ferrflow release`"
                ))
                .error_code(error_code::CONFIG_INVALID_PATH);
            }
            Some(r)
        }
        None => None,
    };

    if ctx.dry_run {
        return Ok(PublishOutcome::DryRun);
    }

    // Drop a scoped `.npmrc` so the auth token is picked up without
    // touching the user's `~/.npmrc`. RAII guard removes it on scope
    // exit — including on the panic path — so we don't leave a token
    // line on disk if anything blows up mid-publish.
    let _npmrc_guard = match (resolved_registry, registry) {
        (Some(r), Some(name)) if r.token_env.is_some() => {
            Some(write_scoped_npmrc(ctx.package_path, r, name)?)
        }
        _ => None,
    };

    let mut cmd = Command::new("npm");
    cmd.current_dir(ctx.package_path).arg("publish");
    if let Some(name) = registry
        && let Some(url) = resolved_registry.and_then(|r| r.url.as_deref())
    {
        cmd.arg(format!("--registry={url}"));
        let _ = name;
    }
    let resolved_tag = tag.unwrap_or("latest");
    cmd.arg(format!("--tag={resolved_tag}"));
    if let Some(a) = access {
        cmd.arg(format!("--access={a}"));
    }
    cmd.args(extra_args);

    let output = cmd.output().with_context(|| {
        format!(
            "spawn `npm publish` failed (is npm in PATH?) for {}",
            ctx.package_name
        )
    })?;
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    if output.status.success() {
        return Ok(PublishOutcome::Published {
            url: derive_npm_url(ctx.package_name, ctx.new_version, registry),
        });
    }

    if classify_already_published(&stderr) {
        return Ok(PublishOutcome::Skipped {
            reason: format!(
                "{}@{} already exists on {}",
                ctx.package_name, ctx.new_version, registry_label
            ),
        });
    }

    Err(anyhow!(
        "npm publish failed for {} on {}: {}",
        ctx.package_name,
        registry_label,
        first_meaningful_line(&stderr, &stdout)
    ))
    .error_code(error_code::CONFIG_INVALID_PATH)
}

/// RAII handle that owns a temporary `.npmrc` file inside the package
/// directory. Drop deletes it (best-effort — we don't surface the
/// remove error if the publish already failed and the user is about
/// to see a real error). Avoids leaving registry tokens on disk.
struct NpmrcGuard {
    path: std::path::PathBuf,
}

impl Drop for NpmrcGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn write_scoped_npmrc(
    package_path: &Path,
    registry: &crate::config::RegistryConfig,
    registry_name: &str,
) -> Result<NpmrcGuard> {
    let path = package_path.join(".npmrc.ferrflow");
    let url = registry.url.as_deref().ok_or_else(|| {
        anyhow!(
            "publisher npm:{registry_name}: registry has a tokenEnv but no url — \
             FerrFlow can't wire the token to a host without one"
        )
    })?;
    let env_name = registry
        .token_env
        .as_deref()
        .expect("called only when token_env is Some");
    let token = std::env::var(env_name).expect("validated by caller");
    let host = url_host(url).unwrap_or("registry.npmjs.org");
    let line = format!("//{host}/:_authToken={token}\n");
    std::fs::write(&path, line).with_context(|| format!("write {}", path.display()))?;
    Ok(NpmrcGuard { path })
}

fn url_host(url: &str) -> Option<&str> {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let host = rest.split('/').next()?;
    if host.is_empty() { None } else { Some(host) }
}

fn classify_already_published(stderr: &str) -> bool {
    let needles = [
        "cannot publish over the previously published versions",
        "you cannot publish over the previously published version",
        "already exists",
        "version already exists",
        "epublishconflict",
        "version is already published",
    ];
    let lower = stderr.to_ascii_lowercase();
    needles.iter().any(|n| lower.contains(n))
}

fn first_meaningful_line(stderr: &str, stdout: &str) -> String {
    for src in [stderr, stdout] {
        for line in src.lines().rev() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("\u{1b}[") {
                continue;
            }
            if trimmed.starts_with("npm ERR!")
                || trimmed.starts_with("error")
                || trimmed.starts_with("Error:")
            {
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

fn derive_npm_url(name: &str, version: &str, registry: Option<&str>) -> Option<String> {
    if registry.is_none() {
        Some(format!("https://www.npmjs.com/package/{name}/v/{version}"))
    } else {
        None
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
            package_name: "@ferrlabs/ui-react",
            package_path: Box::leak(Box::new(pkg_path)),
            new_version: "0.1.0",
            tag: "@ferrlabs/ui-react@v0.1.0",
            registries,
            dry_run,
            verbose: false,
        }
    }

    #[test]
    fn missing_registry_definition_is_a_clear_error() {
        let registries = BTreeMap::new();
        let err = run(
            Some("gh-packages"),
            None,
            None,
            &[],
            &ctx(&registries, true),
        )
        .expect_err("must error");
        assert!(format!("{err:?}").contains("not declared under `workspace.registries`"));
    }

    #[test]
    fn missing_token_env_var_blocks_before_invoking_npm() {
        let mut registries = BTreeMap::new();
        registries.insert(
            "gh-packages".into(),
            RegistryConfig {
                url: Some("https://npm.pkg.github.com".into()),
                token_env: Some("__FERRFLOW_NPM_TOKEN_NEVER_SET".into()),
            },
        );
        let err = run(
            Some("gh-packages"),
            None,
            None,
            &[],
            &ctx(&registries, false),
        )
        .expect_err("must error");
        assert!(format!("{err:?}").contains("is not set"));
    }

    #[test]
    fn dry_run_short_circuits_for_public_npm() {
        let registries = BTreeMap::new();
        let outcome = run(None, None, None, &[], &ctx(&registries, true)).expect("dry-run");
        assert!(matches!(outcome, PublishOutcome::DryRun));
    }

    #[test]
    fn classify_already_published_recognizes_real_phrasings() {
        assert!(classify_already_published(
            "npm ERR! 403 You cannot publish over the previously published versions"
        ));
        assert!(classify_already_published("EPUBLISHCONFLICT"));
        assert!(classify_already_published("Version already exists"));
        assert!(!classify_already_published("npm ERR! 401 Unauthorized"));
        assert!(!classify_already_published(""));
    }

    #[test]
    fn url_host_strips_scheme_and_path() {
        assert_eq!(
            url_host("https://npm.pkg.github.com"),
            Some("npm.pkg.github.com")
        );
        assert_eq!(
            url_host("https://registry.npmjs.org/"),
            Some("registry.npmjs.org")
        );
        assert_eq!(url_host("registry.npmjs.org"), Some("registry.npmjs.org"));
    }

    #[test]
    fn first_meaningful_line_picks_npm_err_over_progress() {
        let stderr = "\u{1b}[2K\nnpm WARN deprecated x\nnpm ERR! version conflict\n";
        let line = first_meaningful_line(stderr, "");
        assert!(line.contains("version conflict"));
    }

    #[test]
    fn public_npmjs_url_only_for_default_registry() {
        assert_eq!(
            derive_npm_url("foo", "1.0.0", None).as_deref(),
            Some("https://www.npmjs.com/package/foo/v/1.0.0")
        );
        assert_eq!(derive_npm_url("foo", "1.0.0", Some("gh-packages")), None);
    }
}
