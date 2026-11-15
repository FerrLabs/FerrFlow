use std::collections::BTreeMap;

use ferrflow::changelog::{self, ChangelogRender, GitLog};
use ferrflow::config::{self, Config, ConfigFileFormat, VersioningStrategy};
use ferrflow::conventional_commits::{self, BumpType};
use ferrflow::versioning;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CommitInput {
    pub message: String,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CommitInfo {
    pub hash: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub commit_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub breaking: bool,
}

#[derive(Debug, Serialize)]
pub struct PackagePreview {
    pub name: String,
    pub current: String,
    pub next: String,
    pub bump: String,
    pub commits: Vec<CommitInfo>,
    pub changelog: String,
}

pub fn parse_config(text: &str, format: Option<&str>) -> Result<Config, String> {
    match format {
        Some(name) => config::format_handler(named_format(name)?)
            .parse(text)
            .map_err(|e| e.to_string()),
        None => infer_and_parse(text),
    }
}

fn named_format(name: &str) -> Result<ConfigFileFormat, String> {
    match name.to_ascii_lowercase().as_str() {
        "json" => Ok(ConfigFileFormat::Json),
        "json5" => Ok(ConfigFileFormat::Json5),
        "toml" => Ok(ConfigFileFormat::Toml),
        other => Err(format!(
            "unknown config format '{other}' (expected json, json5, or toml)"
        )),
    }
}

fn infer_and_parse(text: &str) -> Result<Config, String> {
    let mut last_err = String::from("empty config");
    for fmt in [
        ConfigFileFormat::Json,
        ConfigFileFormat::Json5,
        ConfigFileFormat::Toml,
    ] {
        match config::format_handler(fmt).parse(text) {
            Ok(config) => return Ok(config),
            Err(e) => last_err = e.to_string(),
        }
    }
    Err(last_err)
}

pub fn compute_preview(
    config: &Config,
    commits: &[CommitInput],
    current_versions: &BTreeMap<String, String>,
) -> Vec<PackagePreview> {
    let single_package = config.packages.len() == 1;
    let mut previews = Vec::new();

    for pkg in &config.packages {
        let assigned: Vec<&CommitInput> = commits
            .iter()
            .filter(|c| single_package || commit_touches_package(&c.files, &pkg.path))
            .collect();
        if assigned.is_empty() {
            continue;
        }

        let bump = assigned
            .iter()
            .map(|c| conventional_commits::determine_bump(&c.message))
            .max()
            .unwrap_or(BumpType::None);
        if bump == BumpType::None {
            continue;
        }

        let current = current_versions
            .get(&pkg.name)
            .cloned()
            .unwrap_or_else(|| "0.0.0".to_string());
        let strategy = pkg.versioning.unwrap_or(VersioningStrategy::Semver);
        let next = versioning::compute_next_version(&current, bump, strategy)
            .unwrap_or_else(|_| current.clone());

        let git_logs: Vec<GitLog> = assigned
            .iter()
            .map(|c| GitLog {
                hash: c.hash.clone().unwrap_or_else(|| "0000000".to_string()),
                message: c.message.clone(),
            })
            .collect();
        let changelog =
            changelog::build_section_with(&next, &git_logs, &ChangelogRender::default());

        previews.push(PackagePreview {
            name: pkg.name.clone(),
            current,
            next,
            bump: bump.to_string(),
            commits: assigned.iter().map(|c| commit_info(c)).collect(),
            changelog,
        });
    }

    previews
}

fn commit_info(commit: &CommitInput) -> CommitInfo {
    let header = conventional_commits::parse_header(&commit.message);
    CommitInfo {
        hash: commit.hash.clone().unwrap_or_default(),
        commit_type: header.as_ref().map(|h| h.commit_type.to_string()),
        scope: header.as_ref().and_then(|h| h.scope.map(str::to_string)),
        breaking: conventional_commits::is_breaking(&commit.message),
    }
}

fn commit_touches_package(files: &[String], pkg_path: &str) -> bool {
    let base = pkg_path.trim_end_matches('/');
    if base.is_empty() || base == "." {
        return true;
    }
    let prefix = format!("{base}/");
    files.iter().any(|f| f == base || f.starts_with(&prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single_pkg_config() -> Config {
        parse_config(r#"{"package":[{"name":"app","path":"."}]}"#, Some("json")).unwrap()
    }

    fn commit(message: &str, files: &[&str]) -> CommitInput {
        CommitInput {
            message: message.to_string(),
            hash: Some("abc1234".to_string()),
            files: files.iter().copied().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn parse_config_infers_toml_without_hint() {
        let cfg = parse_config("[[package]]\nname = \"app\"\npath = \".\"\n", None).unwrap();
        assert_eq!(cfg.packages[0].name, "app");
    }

    #[test]
    fn parse_config_rejects_unknown_format() {
        assert!(parse_config("{}", Some("yaml")).is_err());
    }

    #[test]
    fn preview_takes_the_highest_bump_and_computes_next() {
        let cfg = single_pkg_config();
        let commits = vec![
            commit("fix: patch it", &[]),
            commit("feat!: breaking change", &[]),
        ];
        let versions = BTreeMap::from([("app".to_string(), "1.2.3".to_string())]);

        let out = compute_preview(&cfg, &commits, &versions);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bump, "major");
        assert_eq!(out[0].current, "1.2.3");
        assert_eq!(out[0].next, "2.0.0");
        assert!(out[0].commits.iter().any(|c| c.breaking));
    }

    #[test]
    fn preview_skips_packages_with_no_releasable_commits() {
        let cfg = single_pkg_config();
        let commits = vec![commit("chore: tidy", &[]), commit("docs: readme", &[])];
        let versions = BTreeMap::from([("app".to_string(), "1.0.0".to_string())]);

        assert!(compute_preview(&cfg, &commits, &versions).is_empty());
    }

    #[test]
    fn preview_assigns_monorepo_commits_by_path() {
        let cfg = parse_config(
            r#"{"package":[{"name":"api","path":"packages/api"},{"name":"web","path":"packages/web"}]}"#,
            Some("json"),
        )
        .unwrap();
        let commits = vec![
            commit("feat: api thing", &["packages/api/src/main.rs"]),
            commit("fix: web thing", &["packages/web/index.ts"]),
        ];
        let versions = BTreeMap::from([
            ("api".to_string(), "1.0.0".to_string()),
            ("web".to_string(), "2.0.0".to_string()),
        ]);

        let out = compute_preview(&cfg, &commits, &versions);

        let api = out.iter().find(|p| p.name == "api").unwrap();
        assert_eq!(api.bump, "minor");
        assert_eq!(api.next, "1.1.0");
        let web = out.iter().find(|p| p.name == "web").unwrap();
        assert_eq!(web.bump, "patch");
        assert_eq!(web.next, "2.0.1");
    }
}
