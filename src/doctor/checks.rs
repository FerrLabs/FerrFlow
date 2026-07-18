use std::path::Path;
use std::process::Command;

use crate::config::{Config, ForgeKind};
use crate::formats::read_version;
use crate::git::{Repository, collect_all_tags, get_remote_url};
use crate::validate::ValidationLevel;

use super::report::{Check, Section, Status};

fn remote_name(config: Option<&Config>) -> &str {
    config
        .map(|c| c.workspace.remote.as_str())
        .filter(|remote| !remote.is_empty())
        .unwrap_or("origin")
}

pub(super) fn repo_section(
    repo: Option<&Repository>,
    config: Option<&Config>,
    root: &Path,
) -> Section {
    let mut checks = Vec::new();
    let Some(repo) = repo else {
        checks.push(Check::error(
            "git repository",
            Some("not a git repository — run `git init` in the project root".into()),
        ));
        return Section::new("Repo", checks);
    };
    checks.push(Check::ok("git repository", None));

    match repo.head_id() {
        Ok(id) => checks.push(Check::ok(
            "commit history",
            Some(format!("HEAD at {}", &id.to_string()[..7])),
        )),
        Err(_) => checks.push(Check::error(
            "commit history",
            Some("no commits yet — make an initial commit before releasing".into()),
        )),
    }

    match dirty_count(root) {
        Some(0) => checks.push(Check::ok("working tree clean", None)),
        Some(n) => checks.push(Check::warn(
            "working tree clean",
            Some(format!(
                "{n} uncommitted change{}; a release refuses a dirty tree",
                if n == 1 { "" } else { "s" }
            )),
        )),
        None => checks.push(Check::info(
            "working tree clean",
            Some("could not read `git status`".into()),
        )),
    }

    let remote = remote_name(config);
    match get_remote_url(repo, remote) {
        Some(url) => checks.push(Check::ok(
            "remote configured",
            Some(format!("{remote} → {}", redact_url(&url))),
        )),
        None => checks.push(Check::warn(
            "remote configured",
            Some(format!(
                "no '{remote}' remote — releases push tags and commits there"
            )),
        )),
    }

    let tag_count = collect_all_tags(repo).len();
    if tag_count > 0 {
        checks.push(Check::ok(
            "tags present",
            Some(format!(
                "{tag_count} local tag{}",
                if tag_count == 1 { "" } else { "s" }
            )),
        ));
    } else {
        checks.push(Check::info(
            "tags present",
            Some("no tags yet — the first release creates them".into()),
        ));
    }

    Section::new("Repo", checks)
}

pub(super) fn config_section(
    config: Option<&Config>,
    config_error: Option<&str>,
    discovered: &[std::path::PathBuf],
    root: &Path,
) -> Section {
    let mut checks = Vec::new();

    match discovered.len() {
        0 => checks.push(Check::warn(
            "config file",
            Some(
                "none found — run `ferrflow init` (otherwise FerrFlow auto-detects version files)"
                    .into(),
            ),
        )),
        1 => checks.push(Check::ok("config file", Some(filename(&discovered[0])))),
        _ => {
            let list = discovered
                .iter()
                .map(|p| filename(p))
                .collect::<Vec<_>>()
                .join(", ");
            checks.push(Check::error(
                "config file",
                Some(format!(
                    "multiple config files found ({list}) — ambiguous; pass --config to choose one"
                )),
            ));
            return Section::new("Config", checks);
        }
    }

    if let Some(err) = config_error {
        checks.push(Check::error("config parses", Some(err.to_string())));
        return Section::new("Config", checks);
    }

    let Some(config) = config else {
        return Section::new("Config", checks);
    };

    checks.push(Check::ok(
        "config parses",
        Some(format!(
            "{} package{}",
            config.packages.len(),
            if config.packages.len() == 1 { "" } else { "s" }
        )),
    ));

    for entry in crate::validate::local_entries(config, root) {
        let status = match entry.level {
            ValidationLevel::Error => Status::Error,
            ValidationLevel::Warning => Status::Warn,
            ValidationLevel::Suggestion => Status::Info,
        };
        checks.push(Check::new(entry.path, status, Some(entry.message)));
    }

    Section::new("Config", checks)
}

pub(super) fn versioning_section(config: Option<&Config>, root: &Path) -> Section {
    let mut checks = Vec::new();
    let Some(config) = config else {
        checks.push(Check::info(
            "strategy",
            Some("skipped — no parsed config".into()),
        ));
        return Section::new("Versioning", checks);
    };
    if config.packages.is_empty() {
        checks.push(Check::info(
            "strategy",
            Some("no packages configured".into()),
        ));
        return Section::new("Versioning", checks);
    }

    match config.workspace.versioning {
        Some(strategy) => checks.push(Check::info(
            "strategy",
            Some(format!(
                "declared: {}",
                format!("{strategy:?}").to_lowercase()
            )),
        )),
        None => checks.push(Check::info(
            "strategy",
            Some("auto-detected from tags (semver by default)".into()),
        )),
    }

    // On-disk versions only — the tag walk that computes last-tag / next
    // version lives in `ferrflow status` / `check`, and its orphaned-tag
    // warnings would drown out a diagnostic report.
    for pkg in &config.packages {
        let version = pkg
            .versioned_files
            .first()
            .and_then(|vf| read_version(vf, root).ok())
            .unwrap_or_else(|| "unknown".to_string());
        checks.push(Check::ok(pkg.name.clone(), Some(format!("v{version}"))));
    }

    checks.push(Check::info(
        "next versions",
        Some("run `ferrflow check` to preview the next version for each package".into()),
    ));

    Section::new("Versioning", checks)
}

pub(super) fn forge_section(
    repo: Option<&Repository>,
    config: Option<&Config>,
    online: bool,
) -> Section {
    let mut checks = Vec::new();

    let remote = remote_name(config);
    let url = repo.and_then(|r| get_remote_url(r, remote));
    let configured = config.map(|c| c.workspace.forge).unwrap_or(ForgeKind::Auto);
    let detected = url.as_deref().and_then(crate::forge::detect_forge_from_url);
    let forge = match configured {
        ForgeKind::Auto => detected,
        explicit => Some(explicit),
    };

    match forge {
        Some(kind) => {
            let source = if configured == ForgeKind::Auto {
                "from remote URL"
            } else {
                "from config"
            };
            checks.push(Check::ok(
                "forge",
                Some(format!("{} ({source})", forge_label(kind))),
            ));
        }
        None => checks.push(Check::warn(
            "forge",
            Some("could not detect the forge from the remote URL — set workspace.forge".into()),
        )),
    }

    let (var, present) = token_env(forge);
    if present {
        checks.push(Check::ok("auth token", Some(format!("{var} is set"))));
    } else {
        checks.push(Check::warn(
            "auth token",
            Some("no token in env (FERRFLOW_TOKEN / GITHUB_TOKEN / GITLAB_TOKEN) — API pushes and releases will fail".into()),
        ));
    }

    if online {
        checks.push(online_check(forge));
    }

    Section::new("Forge", checks)
}

pub(super) fn ci_section(root: &Path) -> Section {
    let mut checks = Vec::new();

    let gh_workflows = root.join(".github").join("workflows");
    let gitlab = root.join(".gitlab-ci.yml");
    let forgejo = root.join(".forgejo").join("workflows");

    if gh_workflows.is_dir() {
        checks.push(Check::ok("workflows", Some(".github/workflows/".into())));
        checks.push(ferrflow_action_check(&gh_workflows));
    } else if gitlab.is_file() {
        checks.push(Check::ok("workflows", Some(".gitlab-ci.yml".into())));
    } else if forgejo.is_dir() {
        checks.push(Check::ok("workflows", Some(".forgejo/workflows/".into())));
    } else {
        checks.push(Check::info(
            "workflows",
            Some("no CI workflows detected".into()),
        ));
    }

    Section::new("CI", checks)
}

fn ferrflow_action_check(workflows: &Path) -> Check {
    let Ok(entries) = std::fs::read_dir(workflows) else {
        return Check::info(
            "FerrFlow action",
            Some("could not read workflows directory".into()),
        );
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_yaml = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == "yml" || e == "yaml");
        if !is_yaml {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        for token in content.split_whitespace() {
            let reference = token.trim_matches(|c| c == '"' || c == '\'');
            for prefix in ["FerrLabs/FerrFlow@", "FerrFlow-Org/ferrflow@"] {
                if let Some(version) = reference.strip_prefix(prefix) {
                    return Check::ok("FerrFlow action", Some(format!("{prefix}{version}")));
                }
            }
        }
    }
    Check::info(
        "FerrFlow action",
        Some("no FerrLabs/FerrFlow action reference found".into()),
    )
}

fn online_check(forge: Option<ForgeKind>) -> Check {
    match forge {
        Some(ForgeKind::Github) => match github_rate_limit() {
            Ok((remaining, limit)) => Check::ok(
                "forge reachable",
                Some(format!("GitHub API: {remaining}/{limit} calls remaining")),
            ),
            Err(err) => Check::warn(
                "forge reachable",
                Some(format!("GitHub API check failed: {err}")),
            ),
        },
        Some(kind) => Check::info(
            "forge reachable",
            Some(format!(
                "--online check not implemented for {}",
                forge_label(kind)
            )),
        ),
        None => Check::info("forge reachable", Some("no forge to reach".into())),
    }
}

fn github_rate_limit() -> anyhow::Result<(u64, u64)> {
    let token = std::env::var("FERRFLOW_TOKEN")
        .ok()
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .filter(|t| !t.is_empty())
        .ok_or_else(|| anyhow::anyhow!("no token in env"))?;

    let body: serde_json::Value = ureq::get("https://api.github.com/rate_limit")
        .header("Authorization", &format!("Bearer {token}"))
        .header("User-Agent", "ferrflow-doctor")
        .header("Accept", "application/vnd.github+json")
        .call()?
        .body_mut()
        .read_json()?;

    let rate = &body["rate"];
    let remaining = rate["remaining"].as_u64().unwrap_or(0);
    let limit = rate["limit"].as_u64().unwrap_or(0);
    Ok((remaining, limit))
}

fn token_env(forge: Option<ForgeKind>) -> (&'static str, bool) {
    let is_set = |var: &str| std::env::var(var).is_ok_and(|v| !v.is_empty());
    if is_set("FERRFLOW_TOKEN") {
        return ("FERRFLOW_TOKEN", true);
    }
    match forge {
        Some(ForgeKind::Gitlab) => ("GITLAB_TOKEN", is_set("GITLAB_TOKEN")),
        _ => ("GITHUB_TOKEN", is_set("GITHUB_TOKEN")),
    }
}

fn forge_label(kind: ForgeKind) -> &'static str {
    match kind {
        ForgeKind::Github => "GitHub",
        ForgeKind::Gitlab => "GitLab",
        ForgeKind::Gitea => "Gitea/Forgejo",
        ForgeKind::Auto => "auto",
    }
}

fn dirty_count(root: &Path) -> Option<usize> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Some(text.lines().filter(|l| !l.trim().is_empty()).count())
}

fn redact_url(url: &str) -> String {
    for scheme in ["https://", "http://"] {
        if let Some(rest) = url.strip_prefix(scheme)
            && let Some((authority, path)) = rest.split_once('/')
            && let Some((_userinfo, host)) = authority.rsplit_once('@')
        {
            return format!("{scheme}{host}/{path}");
        }
    }
    url.to_string()
}

fn filename(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("(unknown)")
        .to_string()
}
