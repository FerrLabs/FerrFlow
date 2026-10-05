use anyhow::{Context, Result, anyhow};
use std::process::Command;

use super::{PublishContext, PublishOutcome};
use crate::error_code::{self, ErrorCodeExt};

const MAX_PUBLISH_ATTEMPTS: u32 = 3;

const PUBLISH_RETRY_BACKOFF_SECS: [u64; 2] = [5, 15];

pub fn run(
    registry: Option<&str>,
    allow_dirty: bool,
    no_verify: bool,
    extra_args: &[String],
    ctx: &PublishContext<'_>,
) -> Result<PublishOutcome> {
    let registry_label = registry.unwrap_or("crates-io");

    if let Some(name) = registry {
        let r = ctx
            .registries
            .get(name)
            .ok_or_else(|| anyhow!(
                "publisher cargo: registry `{name}` is not declared under `workspace.registries`"
            ))
            .error_code(error_code::PUBLISHER_MISCONFIGURED)?;
        if let Some(env_name) = &r.token_env
            && std::env::var(env_name).is_err()
        {
            return Err(anyhow!(
                "publisher cargo:{name}: env var `{env_name}` is not set; \
                 export the registry token before running `ferrflow release`"
            ))
            .error_code(error_code::PUBLISHER_MISCONFIGURED);
        }
    }

    if ctx.dry_run {
        return Ok(PublishOutcome::DryRun);
    }

    let mut cmd = publish_command(registry, allow_dirty, no_verify, extra_args, ctx);

    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
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
                url: derive_crate_url(&published_name(ctx), ctx.new_version, registry),
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

        if attempt < MAX_PUBLISH_ATTEMPTS && classify_transient(&stderr) {
            let backoff = PUBLISH_RETRY_BACKOFF_SECS
                .get((attempt - 1) as usize)
                .copied()
                .unwrap_or(15);
            tracing::warn!(
                "  ↻ {} publish hit a transient registry error on {} (attempt {attempt}/{MAX_PUBLISH_ATTEMPTS}); \
                 retrying in {backoff}s — likely index lag on a just-published dependency",
                ctx.package_name,
                registry_label
            );
            std::thread::sleep(std::time::Duration::from_secs(backoff));
            continue;
        }

        return Err(anyhow!(
            "cargo publish failed for {} on {}: {}",
            ctx.package_name,
            registry_label,
            failure_report(&stderr, &stdout)
        ))
        .error_code(error_code::PUBLISH_FAILED);
    }
}

fn publish_command(
    registry: Option<&str>,
    allow_dirty: bool,
    no_verify: bool,
    extra_args: &[String],
    ctx: &PublishContext<'_>,
) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(ctx.package_path).arg("publish");
    if let Some(name) = registry {
        cmd.arg("--registry").arg(name);
    }
    if allow_dirty {
        cmd.arg("--allow-dirty");
    }
    if no_verify {
        cmd.arg("--no-verify");
    }
    cmd.args(extra_args);
    cmd
}

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

fn classify_transient(stderr: &str) -> bool {
    let needles = [
        "no matching package named",
        "failed to select a version",
        "required by package",
        "failed to verify package",
        "spurious network error",
        "error trying to connect",
        "connection reset",
        "connection refused",
        "timed out",
        "502 bad gateway",
        "503 service",
        "504 gateway",
    ];
    let lower = stderr.to_ascii_lowercase();
    needles.iter().any(|n| lower.contains(n))
}

fn failure_report(stderr: &str, stdout: &str) -> String {
    let stderr = strip_ansi(stderr);
    let stdout = strip_ansi(stdout);
    for marker in ["error:", "warning:"] {
        for src in [&stderr, &stdout] {
            if let Some(block) = last_block_from(src, marker) {
                return block;
            }
        }
    }
    stderr
        .lines()
        .rfind(|l| !l.trim().is_empty())
        .unwrap_or("(no output)")
        .trim()
        .to_string()
}

fn last_block_from(text: &str, marker: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines
        .iter()
        .rposition(|l| l.trim_start().starts_with(marker))?;
    let block: Vec<&str> = lines[start..]
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    Some(block.join("\n"))
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        if chars.next() == Some('[') {
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        }
    }
    out
}

/// The crate name cargo publishes under, which is `[package].name` in the
/// manifest and not the FerrFlow package name.
///
/// A workspace routinely uses short FerrFlow names against prefixed crate names
/// (`bridge` for `idlewarden-bridge`), and short crate names are long since
/// taken on crates.io, so the wrong one links a stranger's crate rather than
/// 404ing. Falls back to the FerrFlow name when the manifest cannot be read or
/// carries no `[package]` table, which is no worse than what it replaces.
fn published_name(ctx: &PublishContext<'_>) -> String {
    std::fs::read_to_string(ctx.package_path.join("Cargo.toml"))
        .ok()
        .and_then(|raw| raw.parse::<toml_edit::DocumentMut>().ok())
        .and_then(|doc| {
            // `doc["package"]` panics with "index not found" when the manifest
            // has no [package] table, which is exactly what a workspace root
            // looks like. Index by get() so that falls back instead.
            doc.get("package")?
                .get("name")?
                .as_str()
                .map(str::to_string)
        })
        .unwrap_or_else(|| ctx.package_name.to_string())
}

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
        let err = run(Some("kellnr"), false, false, &[], &c).expect_err("must error");
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
        let err = run(Some("kellnr"), false, false, &[], &c).expect_err("must error");
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
        let outcome = run(None, false, false, &[], &c).expect("public registry dry-run");
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
        let outcome = run(Some("kellnr"), false, false, &[], &c).expect("dry-run with env");
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
    fn classify_transient_recognizes_index_lag_and_network() {
        assert!(classify_transient(
            "error: failed to verify package tarball\n\nCaused by:\n  no matching package named `ferrlabs-errors` found\n  ... required by package `ferrlabs-auth v0.8.0`"
        ));
        assert!(classify_transient(
            "error: failed to select a version for the requirement `ferrlabs-db = \"^0.4\"`"
        ));
        assert!(classify_transient("error: 503 Service Unavailable"));
        assert!(!classify_transient(
            "error: crate version `0.1.0` is already uploaded"
        ));
        assert!(!classify_transient("error: missing field `description`"));
        assert!(!classify_transient(""));
    }

    #[test]
    fn failure_report_picks_error_over_spinner() {
        let stderr =
            "\u{1b}[2K\u{1b}[K\nerror: failed to publish: cargo lock conflict\n   exit code 101\n";
        let report = failure_report(stderr, "");
        assert!(report.starts_with("error: failed to publish: cargo lock conflict"));
    }

    #[test]
    fn failure_report_keeps_the_cause_when_cargo_colours_its_output() {
        let stderr = "\u{1b}[1m\u{1b}[92m   Packaging\u{1b}[0m core v2.0.0\n\
                      \u{1b}[1m\u{1b}[91merror\u{1b}[0m: failed to prepare local package for uploading\n\
                      \n\
                      Caused by:\n  \
                      no matching package named `core` found\n  \
                      location searched: forgejo index\n  \
                      required by package `gapline v2.0.0 (/workspace/gapline/cli)`\n";

        assert_eq!(
            failure_report(stderr, ""),
            "error: failed to prepare local package for uploading\n\
             Caused by:\n\
             no matching package named `core` found\n\
             location searched: forgejo index\n\
             required by package `gapline v2.0.0 (/workspace/gapline/cli)`"
        );
    }

    #[test]
    fn crates_io_url_only_emitted_for_default_registry() {
        assert_eq!(
            derive_crate_url("foo", "1.0.0", None).as_deref(),
            Some("https://crates.io/crates/foo/1.0.0")
        );
        assert_eq!(derive_crate_url("foo", "1.0.0", Some("kellnr")), None);
    }

    fn ctx_at<'a>(
        registries: &'a BTreeMap<String, RegistryConfig>,
        package_path: &'a std::path::Path,
        ferrflow_name: &'a str,
    ) -> PublishContext<'a> {
        PublishContext {
            package_name: ferrflow_name,
            package_path,
            new_version: "26.8.27",
            tag: "v26.8.27",
            registries,
            dry_run: false,
            verbose: false,
        }
    }

    #[test]
    fn the_published_name_comes_from_the_manifest_not_the_ferrflow_name() {
        // The shape from #948: short FerrFlow names against prefixed crates.
        // `crates.io/crates/bridge` is a real, unrelated crate, so the wrong
        // name links somebody else's project rather than 404ing.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"idlewarden-bridge\"\nversion = \"26.8.27\"\n",
        )
        .unwrap();
        let registries = BTreeMap::new();

        let name = published_name(&ctx_at(&registries, dir.path(), "bridge"));

        assert_eq!(name, "idlewarden-bridge");
    }

    #[test]
    fn a_manifest_with_no_package_table_falls_back_instead_of_panicking() {
        // A workspace root manifest has [workspace] and no [package]. toml_edit
        // indexing a missing key must not bring the publish down.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();
        let registries = BTreeMap::new();

        let name = published_name(&ctx_at(&registries, dir.path(), "bridge"));

        assert_eq!(name, "bridge");
    }

    #[test]
    fn an_absent_manifest_falls_back_to_the_ferrflow_name() {
        let dir = tempfile::tempdir().unwrap();
        let registries = BTreeMap::new();

        let name = published_name(&ctx_at(&registries, dir.path(), "bridge"));

        assert_eq!(
            name, "bridge",
            "no manifest is no worse than before, so it must not fail the publish"
        );
    }

    #[test]
    fn an_unparseable_manifest_falls_back_too() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package\nname = broken").unwrap();
        let registries = BTreeMap::new();

        let name = published_name(&ctx_at(&registries, dir.path(), "bridge"));

        assert_eq!(name, "bridge");
    }

    #[test]
    fn a_matching_name_is_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"ferrflow\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let registries = BTreeMap::new();

        let name = published_name(&ctx_at(&registries, dir.path(), "ferrflow"));

        assert_eq!(name, "ferrflow");
    }

    fn args_of(cmd: &Command) -> Vec<String> {
        cmd.get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn a_private_registry_publish_passes_every_flag_to_cargo() {
        let registries = BTreeMap::new();
        let dir = tempfile::tempdir().unwrap();
        let c = ctx_at(&registries, dir.path(), "bridge");

        let cmd = publish_command(Some("kellnr"), true, true, &["--locked".to_string()], &c);

        assert_eq!(cmd.get_program(), "cargo");
        assert_eq!(
            args_of(&cmd),
            vec![
                "publish",
                "--registry",
                "kellnr",
                "--allow-dirty",
                "--no-verify",
                "--locked"
            ]
        );
        assert_eq!(cmd.get_current_dir(), Some(dir.path()));
    }

    #[test]
    fn a_default_publish_adds_no_optional_flags() {
        let registries = BTreeMap::new();
        let dir = tempfile::tempdir().unwrap();
        let c = ctx_at(&registries, dir.path(), "bridge");

        let cmd = publish_command(None, false, false, &[], &c);

        assert_eq!(args_of(&cmd), vec!["publish"]);
    }

    #[test]
    fn failure_report_prefers_an_error_to_a_later_warning() {
        let stderr = "   Packaging foo v1.0.0\nerror: first problem\n   Uploading\n";
        assert_eq!(
            failure_report(stderr, ""),
            "error: first problem\nUploading"
        );
        assert_eq!(
            failure_report("   Packaging foo\nwarning: last word\n", ""),
            "warning: last word"
        );
    }

    #[test]
    fn failure_report_falls_back_to_stdout_then_the_last_stderr_line() {
        assert_eq!(
            failure_report("   Updating index\n", "error: from stdout\n"),
            "error: from stdout"
        );
        assert_eq!(
            failure_report("   Updating index\n   Uploading foo\n\n", "done\n"),
            "Uploading foo"
        );
        assert_eq!(failure_report("", ""), "(no output)");
    }

    #[test]
    fn the_crate_url_uses_the_manifest_name() {
        let url = derive_crate_url("idlewarden-bridge", "26.8.27", None).unwrap();

        assert_eq!(url, "https://crates.io/crates/idlewarden-bridge/26.8.27");
    }
}
