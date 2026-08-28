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

    let npmrc = match (resolved_registry, registry) {
        (Some(r), Some(name)) if r.token_env.is_some() => Some(write_scoped_npmrc(r, name)?),
        _ => None,
    };

    let mut cmd = build_publish_command(
        npmrc.as_ref(),
        resolved_registry,
        registry,
        tag,
        access,
        extra_args,
        ctx,
    );

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
            url: derive_npm_url(&published_name(ctx), ctx.new_version, registry),
        });
    }

    if classify_already_published(&stderr) {
        return Ok(PublishOutcome::Skipped {
            reason: format!(
                "{}@{} already exists on {}",
                published_name(ctx),
                ctx.new_version,
                registry_label
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

#[allow(clippy::too_many_arguments)]
fn build_publish_command(
    npmrc: Option<&NpmrcGuard>,
    resolved_registry: Option<&crate::config::RegistryConfig>,
    registry: Option<&str>,
    tag: Option<&str>,
    access: Option<&str>,
    extra_args: &[String],
    ctx: &PublishContext<'_>,
) -> Command {
    let mut cmd = Command::new("npm");
    cmd.current_dir(ctx.package_path).arg("publish");
    if let Some(npmrc) = npmrc {
        cmd.env("NPM_CONFIG_USERCONFIG", npmrc.path());
    }
    if registry.is_some()
        && let Some(url) = resolved_registry.and_then(|r| r.url.as_deref())
    {
        cmd.arg(format!("--registry={url}"));
    }
    cmd.arg(format!("--tag={}", tag.unwrap_or("latest")));
    if let Some(a) = access {
        cmd.arg(format!("--access={a}"));
    }
    cmd.args(extra_args);
    cmd
}

#[derive(Debug)]
struct NpmrcGuard {
    _dir: tempfile::TempDir,
    path: std::path::PathBuf,
}

impl NpmrcGuard {
    fn path(&self) -> &Path {
        &self.path
    }
}

fn write_scoped_npmrc(
    registry: &crate::config::RegistryConfig,
    registry_name: &str,
) -> Result<NpmrcGuard> {
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

    let dir = tempfile::tempdir().context("create temp dir for the scoped npm config")?;
    let path = dir.path().join(".npmrc");
    let line = format!("//{host}/:_authToken={token}\n");
    write_private(&path, &line).with_context(|| format!("write {}", path.display()))?;
    Ok(NpmrcGuard { _dir: dir, path })
}

#[cfg(unix)]
fn write_private(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents.as_bytes())
}

#[cfg(not(unix))]
fn write_private(path: &Path, contents: &str) -> std::io::Result<()> {
    std::fs::write(path, contents)
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

/// The name npm publishes under, which is the one in `package.json` and not the
/// FerrFlow package name.
///
/// They differ by default in a monorepo: the FerrFlow name is short (`site`,
/// `api`, `core`) while the npm name is scoped (`@acme/site`). Those short names
/// are all taken on npm, so using the wrong one links a stranger's package
/// rather than 404ing. Falls back to the FerrFlow name when the manifest cannot
/// be read, which is no worse than what it replaces.
fn published_name(ctx: &PublishContext<'_>) -> String {
    std::fs::read_to_string(ctx.package_path.join("package.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|manifest| manifest.get("name")?.as_str().map(str::to_string))
        .unwrap_or_else(|| ctx.package_name.to_string())
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

    fn scoped_registry() -> RegistryConfig {
        RegistryConfig {
            url: Some("https://npm.pkg.github.com".into()),
            token_env: Some("FERRFLOW_TEST_NPM_TOKEN".into()),
        }
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_token<T>(f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: a test-only var name used by these tests alone, and
        unsafe { std::env::set_var("FERRFLOW_TEST_NPM_TOKEN", "npm-secret-xyz") };
        let out = f();
        unsafe { std::env::remove_var("FERRFLOW_TEST_NPM_TOKEN") };
        out
    }

    #[test]
    fn publish_command_points_npm_at_the_scoped_npmrc() {
        with_token(|| {
            let registry = scoped_registry();
            let guard = write_scoped_npmrc(&registry, "gh-packages").expect("write npmrc");
            let registries = BTreeMap::new();
            let cmd = build_publish_command(
                Some(&guard),
                Some(&registry),
                Some("gh-packages"),
                None,
                None,
                &[],
                &ctx(&registries, false),
            );

            let userconfig = cmd
                .get_envs()
                .find(|(k, _)| *k == "NPM_CONFIG_USERCONFIG")
                .and_then(|(_, v)| v)
                .expect("NPM_CONFIG_USERCONFIG must be set, or npm ignores the token");
            let path = std::path::Path::new(userconfig);

            assert_eq!(
                path.file_name().and_then(|n| n.to_str()),
                Some(".npmrc"),
                "npm only reads files named `.npmrc`"
            );
            let contents = std::fs::read_to_string(path).expect("npmrc readable");
            assert_eq!(
                contents.trim(),
                "//npm.pkg.github.com/:_authToken=npm-secret-xyz"
            );
        });
    }

    #[test]
    fn scoped_npmrc_lives_outside_the_package_directory() {
        with_token(|| {
            let guard = write_scoped_npmrc(&scoped_registry(), "gh-packages").expect("write");
            let pkg_dir = std::path::Path::new(".").canonicalize().unwrap();
            assert!(
                !guard.path().starts_with(&pkg_dir),
                "the token file must not sit in the package dir — a later docker \
                 publisher would build it into an image layer: {}",
                guard.path().display()
            );
        });
    }

    #[test]
    fn scoped_npmrc_is_removed_when_the_guard_drops() {
        with_token(|| {
            let path = {
                let guard = write_scoped_npmrc(&scoped_registry(), "gh-packages").expect("write");
                guard.path().to_path_buf()
            };
            assert!(!path.exists(), "token file survived the guard");
        });
    }

    #[cfg(unix)]
    #[test]
    fn scoped_npmrc_is_not_world_readable() {
        use std::os::unix::fs::PermissionsExt;
        with_token(|| {
            let guard = write_scoped_npmrc(&scoped_registry(), "gh-packages").expect("write");
            let mode = std::fs::metadata(guard.path())
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0, "registry token readable by group/other");
        });
    }

    #[test]
    fn scoped_npmrc_requires_a_registry_url() {
        with_token(|| {
            let registry = RegistryConfig {
                url: None,
                token_env: Some("FERRFLOW_TEST_NPM_TOKEN".into()),
            };
            let err = write_scoped_npmrc(&registry, "gh-packages").expect_err("must error");
            assert!(format!("{err:?}").contains("no url"));
        });
    }

    #[test]
    fn public_npm_publish_sets_no_userconfig() {
        let registries = BTreeMap::new();
        let cmd =
            build_publish_command(None, None, None, None, None, &[], &ctx(&registries, false));
        assert!(
            cmd.get_envs().all(|(k, _)| k != "NPM_CONFIG_USERCONFIG"),
            "unscoped publish must not shadow the user's own npm config"
        );
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

    fn ctx_at<'a>(
        registries: &'a BTreeMap<String, RegistryConfig>,
        package_path: &'a std::path::Path,
        ferrflow_name: &'a str,
    ) -> PublishContext<'a> {
        PublishContext {
            package_name: ferrflow_name,
            package_path,
            new_version: "1.0.0",
            tag: "v1.0.0",
            registries,
            dry_run: false,
            verbose: false,
        }
    }

    #[test]
    fn the_published_name_comes_from_package_json_not_the_ferrflow_name() {
        // The default monorepo shape: a short FerrFlow name against a scoped npm
        // name. Using the FerrFlow one links `npmjs.com/package/site`, which is
        // a stranger's package rather than a 404.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{ "name": "@acme/site", "version": "1.0.0" }"#,
        )
        .unwrap();
        let registries = BTreeMap::new();

        let name = published_name(&ctx_at(&registries, dir.path(), "site"));

        assert_eq!(name, "@acme/site");
    }

    #[test]
    fn a_scoped_name_stays_literal_in_the_url() {
        // npmjs.com wants `@acme/site`, not `%40acme%2Fsite`.
        let url = derive_npm_url("@acme/site", "1.0.0", None).unwrap();

        assert_eq!(url, "https://www.npmjs.com/package/@acme/site/v/1.0.0");
    }

    #[test]
    fn an_unreadable_manifest_falls_back_to_the_ferrflow_name() {
        let dir = tempfile::tempdir().unwrap();
        let registries = BTreeMap::new();

        let name = published_name(&ctx_at(&registries, dir.path(), "site"));

        assert_eq!(
            name, "site",
            "no manifest is no worse than before, so it must not fail the publish"
        );
    }

    #[test]
    fn a_manifest_without_a_name_falls_back_too() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{ "version": "1.0.0" }"#).unwrap();
        let registries = BTreeMap::new();

        let name = published_name(&ctx_at(&registries, dir.path(), "site"));

        assert_eq!(name, "site");
    }

    #[test]
    fn a_matching_name_is_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{ "name": "ferrflow", "version": "1.0.0" }"#,
        )
        .unwrap();
        let registries = BTreeMap::new();

        let name = published_name(&ctx_at(&registries, dir.path(), "ferrflow"));

        assert_eq!(name, "ferrflow");
    }
}
