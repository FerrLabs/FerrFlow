//! `cargo publish` executor.
//!
//! Behaviour:
//! - Resolves the target registry: when `registry` is `Some(name)`,
//!   look up `workspace.registries.<name>` to validate that the
//!   referenced token env var is exported. Missing env var ⇒ clear
//!   error (we don't want to discover this after `cargo publish` has
//!   uploaded everything to crates.io by mistake).
//! - Runs `cargo publish` (or `cargo publish --dry-run` for dry-runs)
//!   in the package directory.
//! - Treats "already uploaded" / "already in use" as a successful
//!   idempotent skip — this is the load-bearing property for retry
//!   semantics + crash-resume (#549). cargo's exit code is 101 in
//!   both the real-error and already-published cases, so we have to
//!   pattern-match on stderr; tests pin the recognized phrasings.

use anyhow::{Context, Result, anyhow};
use std::process::Command;

use super::{PublishContext, PublishOutcome};
use crate::error_code::{self, ErrorCodeExt};

pub fn run(
    registry: Option<&str>,
    allow_dirty: bool,
    ctx: &PublishContext<'_>,
) -> Result<PublishOutcome> {
    let registry_label = registry.unwrap_or("crates-io");

    // Validate that the configured token env var is exported so we
    // fail fast before invoking cargo, with a clearer message than
    // cargo's own "token not found".
    if let Some(name) = registry {
        let r = ctx
            .registries
            .get(name)
            .ok_or_else(|| anyhow!(
                "publisher cargo: registry `{name}` is not declared under `workspace.registries`"
            ))
            .error_code(error_code::CONFIG_INVALID_PATH)?;
        if let Some(env_name) = &r.token_env
            && std::env::var(env_name).is_err()
        {
            return Err(anyhow!(
                "publisher cargo:{name}: env var `{env_name}` is not set; \
                 export the registry token before running `ferrflow release`"
            ))
            .error_code(error_code::CONFIG_INVALID_PATH);
        }
    }

    if ctx.dry_run {
        return Ok(PublishOutcome::DryRun);
    }

    let mut cmd = Command::new("cargo");
    cmd.current_dir(ctx.package_path).arg("publish");
    if let Some(name) = registry {
        cmd.arg("--registry").arg(name);
    }
    if allow_dirty {
        cmd.arg("--allow-dirty");
    }

    let output = cmd.output().with_context(|| {
        format!(
            "spawn `cargo publish` failed (is cargo in PATH?) for {}",
            ctx.package_name
        )
    })?;
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    if output.status.success() {
        return Ok(PublishOutcome::Published {
            url: derive_crate_url(ctx.package_name, ctx.new_version, registry),
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
        "cargo publish failed for {} on {}: {}",
        ctx.package_name,
        registry_label,
        first_meaningful_line(&stderr, &stdout)
    ))
    .error_code(error_code::CONFIG_INVALID_PATH)
}

/// Recognize the various phrasings cargo + private registries use to
/// say "this version is already on the registry". Tested with cases
/// pulled from real failure logs across crates.io, Kellnr, and
/// Cloudsmith.
fn classify_already_published(stderr: &str) -> bool {
    let needles = [
        "is already uploaded",
        "already exists",
        "crate version is already on registry",
        "already published",
        "version already exists",
    ];
    let lower = stderr.to_ascii_lowercase();
    needles.iter().any(|n| lower.contains(n))
}

/// Pick the first non-empty, non-progress-spinner line from cargo's
/// output so the error message users see is informative instead of
/// the trailing "exit code 101" wrapper.
fn first_meaningful_line(stderr: &str, stdout: &str) -> String {
    for src in [stderr, stdout] {
        for line in src.lines().rev() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Strip ANSI color escapes that some Cargo versions emit.
            if trimmed.starts_with("\u{1b}[") {
                continue;
            }
            if trimmed.starts_with("error:") || trimmed.starts_with("warning:") {
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

/// crates.io has a stable URL shape. Custom registries vary, so we
/// only render the URL for the canonical public registry — better no
/// link than a wrong one in the step summary.
fn derive_crate_url(name: &str, version: &str, registry: Option<&str>) -> Option<String> {
    if registry.is_none() {
        Some(format!("https://crates.io/crates/{name}/{version}"))
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
    ) -> (PublishContext<'a>, PathBuf) {
        let pkg_path = PathBuf::from(".");
        let pc = PublishContext {
            package_name: "ferrlabs-auth",
            package_path: Box::leak(Box::new(pkg_path.clone())),
            new_version: "0.1.0",
            tag: "ferrlabs-auth@v0.1.0",
            registries,
            dry_run,
            verbose: false,
        };
        (pc, pkg_path)
    }

    #[test]
    fn missing_registry_definition_is_a_clear_error() {
        let registries = BTreeMap::new();
        let (c, _) = ctx(&registries, true);
        let err = run(Some("kellnr"), false, &c).expect_err("must error");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("not declared under `workspace.registries`"),
            "expected helpful diagnostic, got: {msg}"
        );
    }

    #[test]
    fn missing_token_env_var_blocks_before_invoking_cargo() {
        let mut registries = BTreeMap::new();
        registries.insert(
            "kellnr".into(),
            RegistryConfig {
                url: Some("https://kellnr.test".into()),
                token_env: Some("__FERRFLOW_TEST_TOKEN_THAT_IS_NEVER_SET".into()),
            },
        );
        let (c, _) = ctx(&registries, false);
        let err = run(Some("kellnr"), false, &c).expect_err("must error");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("is not set"),
            "expected env-var diagnostic, got: {msg}"
        );
    }

    #[test]
    fn dry_run_short_circuits_after_validation() {
        let registries = BTreeMap::new();
        let (c, _) = ctx(&registries, true);
        let outcome = run(None, false, &c).expect("public registry dry-run");
        assert!(matches!(outcome, PublishOutcome::DryRun));
    }

    #[test]
    fn dry_run_passes_through_when_env_is_set() {
        let mut registries = BTreeMap::new();
        registries.insert(
            "kellnr".into(),
            RegistryConfig {
                url: None,
                token_env: Some("PATH".into()), // always exported
            },
        );
        let (c, _) = ctx(&registries, true);
        let outcome = run(Some("kellnr"), false, &c).expect("dry-run with env");
        assert!(matches!(outcome, PublishOutcome::DryRun));
    }

    #[test]
    fn classify_already_published_recognizes_real_phrasings() {
        assert!(classify_already_published(
            "error: crate version `0.1.0` is already uploaded"
        ));
        assert!(classify_already_published("Crate Version Already Exists"));
        assert!(classify_already_published(
            "Already published this version, refusing"
        ));
        assert!(!classify_already_published("error: network unreachable"));
        assert!(!classify_already_published(""));
    }

    #[test]
    fn first_meaningful_line_picks_error_over_spinner() {
        let stderr =
            "\u{1b}[2K\u{1b}[K\nerror: failed to publish: cargo lock conflict\n   exit code 101\n";
        let line = first_meaningful_line(stderr, "");
        assert!(line.contains("cargo lock conflict"));
    }

    #[test]
    fn crates_io_url_only_emitted_for_default_registry() {
        assert_eq!(
            derive_crate_url("foo", "1.0.0", None).as_deref(),
            Some("https://crates.io/crates/foo/1.0.0")
        );
        assert_eq!(derive_crate_url("foo", "1.0.0", Some("kellnr")), None);
    }
}
