use anyhow::Result;
use colored::{ColoredString, Colorize};
use gix::ObjectId;
use serde::Serialize;
use std::path::Path;

use crate::changelog::{ChangelogRender, build_section_with};
use crate::config::{Config, PackageConfig, WorkspaceConfig};
use crate::conventional_commits::{
    BumpType, determine_bump, is_breaking, parse_header, parse_subject,
};
use crate::error_code::{self, ErrorCodeExt};
use crate::git::{
    get_changed_files_between, get_changed_files_for_commit, get_commits_between, get_remote_url,
    get_repo_root, open_repo, resolve_tag_name_to_commit,
};

const MAX_FILES_SHOWN: usize = 40;

pub fn run(spec: &[String], json: bool, config_path: Option<&Path>) -> Result<()> {
    let (package, range) = parse_spec(spec)?;
    let (from_ref, to_ref) = split_range(range)?;

    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;
    let pkg = resolve_package(&config, package)?;
    let is_monorepo = config.is_monorepo();

    let (from_oid, from_tag) =
        resolve_endpoint(&repo, pkg, &config.workspace, is_monorepo, from_ref)?;
    let (to_oid, to_tag) = resolve_endpoint(&repo, pkg, &config.workspace, is_monorepo, to_ref)?;

    let skip = config.workspace.effective_commit_skip_markers();
    let commits = get_commits_between(&repo, from_oid, to_oid, &skip, |repo, oid| {
        commit_touches_package(repo, pkg, is_monorepo, oid)
    })?;
    let files = scope_files_to_package(
        pkg,
        is_monorepo,
        get_changed_files_between(&repo, from_oid, to_oid).unwrap_or_default(),
    );

    let overall = commits
        .iter()
        .map(|c| determine_bump(&c.message))
        .max()
        .unwrap_or(BumpType::None);

    let forge_base = config.workspace.changelog.as_ref().and_then(|cl| {
        if cl.include_commit_links || cl.include_compare_link {
            get_remote_url(&repo, &config.workspace.remote)
                .as_deref()
                .and_then(crate::forge::web_base_url)
        } else {
            None
        }
    });
    let to_version = to_ref.trim_start_matches('v').to_string();
    let render = ChangelogRender {
        config: config.workspace.changelog.as_ref(),
        forge_base,
        last_tag: Some(from_tag.clone()),
        new_tag: Some(to_tag.clone()),
    };
    let changelog = build_section_with(&to_version, &commits, &render);

    if json {
        print_json(pkg, from_ref, to_ref, overall, &commits, &files, &changelog)?;
    } else {
        print_human(pkg, from_ref, to_ref, overall, &commits, &files, &changelog);
    }
    Ok(())
}

/// The raw range holds every commit between the two tags, including ones that
/// only touched other packages. Keeping just the ones that touched this package
/// makes the commit list, the breaking-change list and the rendered changelog
/// match what `ferrflow release` would produce for it (#752).
///
/// A commit whose changed files can't be read is kept rather than dropped —
/// over-reporting is recoverable, silently hiding a commit is not.
fn commit_touches_package(
    repo: &crate::git::Repository,
    pkg: &PackageConfig,
    is_monorepo: bool,
    oid: ObjectId,
) -> bool {
    if !is_monorepo {
        return true;
    }
    match get_changed_files_for_commit(repo, oid) {
        Ok(files) => pkg.is_touched_by(&files, true),
        Err(_) => true,
    }
}

fn scope_files_to_package(
    pkg: &PackageConfig,
    is_monorepo: bool,
    files: Vec<String>,
) -> Vec<String> {
    if !is_monorepo {
        return files;
    }
    files
        .into_iter()
        .filter(|f| pkg.is_touched_by(std::slice::from_ref(f), true))
        .collect()
}

fn parse_spec(spec: &[String]) -> Result<(Option<&str>, &str)> {
    let range = spec
        .iter()
        .find(|s| s.contains(".."))
        .map(String::as_str)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "expected a version range like `v1.0.0..v2.0.0`. Usage: ferrflow diff [package] <from>..<to>"
            )
        })
        .error_code(error_code::DIFF_BAD_RANGE)?;
    let package = spec.iter().find(|s| !s.contains("..")).map(String::as_str);
    Ok((package, range))
}

fn split_range(range: &str) -> Result<(&str, &str)> {
    range
        .split_once("..")
        .filter(|(from, to)| !from.is_empty() && !to.is_empty())
        .ok_or_else(|| anyhow::anyhow!("range must be `<from>..<to>`, both sides non-empty"))
        .error_code(error_code::DIFF_BAD_RANGE)
}

fn resolve_package<'a>(config: &'a Config, name: Option<&str>) -> Result<&'a PackageConfig> {
    if config.packages.is_empty() {
        return Err(anyhow::anyhow!(
            "No packages configured. Run `ferrflow init` to create a config."
        ))
        .error_code(error_code::QUERY_NO_PACKAGES);
    }
    match name {
        Some(n) => config
            .packages
            .iter()
            .find(|p| p.name == n)
            .ok_or_else(|| anyhow::anyhow!("package '{n}' not found"))
            .error_code(error_code::QUERY_PACKAGE_NOT_FOUND),
        None if config.packages.len() == 1 => Ok(&config.packages[0]),
        None => Err(anyhow::anyhow!(
            "this is a monorepo — name the package: ferrflow diff <package> <from>..<to>"
        ))
        .error_code(error_code::DIFF_PACKAGE_REQUIRED),
    }
}

/// Resolve a range endpoint to a commit. Tries the endpoint as a literal tag
/// first (real tag / `v1.4.0` in a single-package repo), then as the package's
/// tag for that version (`api@v1.4.0`).
fn resolve_endpoint(
    repo: &crate::git::Repository,
    pkg: &PackageConfig,
    workspace: &WorkspaceConfig,
    is_monorepo: bool,
    endpoint: &str,
) -> Result<(ObjectId, String)> {
    let version = endpoint.strip_prefix('v').unwrap_or(endpoint);
    let candidates = [
        endpoint.to_string(),
        pkg.tag_for_version(workspace, is_monorepo, version),
        pkg.tag_for_version(workspace, is_monorepo, endpoint),
    ];
    for cand in &candidates {
        if let Some(oid) = resolve_tag_name_to_commit(repo, cand) {
            return Ok((oid, cand.clone()));
        }
    }
    Err(anyhow::anyhow!(
        "could not resolve '{endpoint}' to a tag (tried: {}). Pass an existing tag name or version.",
        candidates.join(", ")
    ))
    .error_code(error_code::DIFF_ENDPOINT_UNRESOLVED)
}

fn bump_label(bump: BumpType) -> ColoredString {
    match bump {
        BumpType::Major => "major".red().bold(),
        BumpType::Minor => "minor".yellow(),
        BumpType::Patch => "patch".cyan(),
        BumpType::None => "none".dimmed(),
    }
}

fn print_human(
    pkg: &PackageConfig,
    from: &str,
    to: &str,
    overall: BumpType,
    commits: &[crate::git::GitLog],
    files: &[String],
    changelog: &str,
) {
    println!(
        "{}  {} → {}  ({})\n",
        pkg.name.bold(),
        from.cyan(),
        to.green().bold(),
        bump_label(overall)
    );

    println!("{}", format!("Commits ({})", commits.len()).bold());
    if commits.is_empty() {
        println!("  {}", "(none)".dimmed());
    }
    for c in commits {
        println!(
            "  {:<5}  {}  {}",
            bump_label(determine_bump(&c.message)),
            c.hash.dimmed(),
            parse_subject(&c.message)
        );
    }

    let breaking: Vec<&crate::git::GitLog> =
        commits.iter().filter(|c| is_breaking(&c.message)).collect();
    if !breaking.is_empty() {
        println!(
            "\n{}",
            format!("Breaking changes ({})", breaking.len())
                .red()
                .bold()
        );
        for c in &breaking {
            println!("  {} {}", "!".red().bold(), parse_subject(&c.message));
        }
    }

    println!("\n{}", format!("Files changed ({})", files.len()).bold());
    for f in files.iter().take(MAX_FILES_SHOWN) {
        println!("  {f}");
    }
    if files.len() > MAX_FILES_SHOWN {
        println!(
            "  {}",
            format!("… and {} more", files.len() - MAX_FILES_SHOWN).dimmed()
        );
    }

    println!("\n{}", "Changelog".bold());
    for line in changelog.trim_end().lines() {
        println!("  {line}");
    }
}

#[derive(Serialize)]
struct CommitJson {
    hash: String,
    subject: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    commit_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    breaking: bool,
    bump: String,
}

#[derive(Serialize)]
struct DiffJson<'a> {
    package: &'a str,
    from: &'a str,
    to: &'a str,
    bump: String,
    commits: Vec<CommitJson>,
    breaking: Vec<String>,
    files_changed: &'a [String],
    changelog: &'a str,
}

fn print_json(
    pkg: &PackageConfig,
    from: &str,
    to: &str,
    overall: BumpType,
    commits: &[crate::git::GitLog],
    files: &[String],
    changelog: &str,
) -> Result<()> {
    let commit_json: Vec<CommitJson> = commits
        .iter()
        .map(|c| {
            let header = parse_header(&c.message);
            CommitJson {
                hash: c.hash.clone(),
                subject: parse_subject(&c.message).to_string(),
                commit_type: header.as_ref().map(|h| h.commit_type.to_string()),
                scope: header.as_ref().and_then(|h| h.scope.map(str::to_string)),
                breaking: is_breaking(&c.message),
                bump: determine_bump(&c.message).to_string(),
            }
        })
        .collect();
    let breaking: Vec<String> = commits
        .iter()
        .filter(|c| is_breaking(&c.message))
        .map(|c| parse_subject(&c.message).to_string())
        .collect();

    let out = DiffJson {
        package: &pkg.name,
        from,
        to,
        bump: overall.to_string(),
        commits: commit_json,
        breaking,
        files_changed: files,
        changelog,
    };
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_spec_finds_range_and_optional_package() {
        let two = vec!["api".to_string(), "v1.0.0..v2.0.0".to_string()];
        assert_eq!(parse_spec(&two).unwrap(), (Some("api"), "v1.0.0..v2.0.0"));

        let one = vec!["v1.0.0..v2.0.0".to_string()];
        assert_eq!(parse_spec(&one).unwrap(), (None, "v1.0.0..v2.0.0"));

        // package can be given after the range too
        let rev = vec!["v1.0.0..v2.0.0".to_string(), "api".to_string()];
        assert_eq!(parse_spec(&rev).unwrap(), (Some("api"), "v1.0.0..v2.0.0"));
    }

    #[test]
    fn parse_spec_requires_a_range() {
        let no_range = vec!["api".to_string(), "v1.0.0".to_string()];
        assert!(parse_spec(&no_range).is_err());
    }

    #[test]
    fn split_range_rejects_empty_sides() {
        assert_eq!(split_range("v1.0.0..v2.0.0").unwrap(), ("v1.0.0", "v2.0.0"));
        assert!(split_range("..v2.0.0").is_err());
        assert!(split_range("v1.0.0..").is_err());
        assert!(split_range("v1.0.0").is_err());
    }

    fn scoped_pkg() -> PackageConfig {
        serde_json::from_str(r#"{"name":"api","path":"packages/api","sharedPaths":["proto"]}"#)
            .expect("valid package json")
    }

    #[test]
    fn file_list_is_scoped_to_the_package_in_a_monorepo() {
        let files = vec![
            "packages/api/src/main.rs".to_string(),
            "packages/web/app.ts".to_string(),
            "proto/schema.proto".to_string(),
        ];
        let scoped = scope_files_to_package(&scoped_pkg(), true, files);
        assert_eq!(
            scoped,
            vec![
                "packages/api/src/main.rs".to_string(),
                "proto/schema.proto".to_string()
            ],
            "the sibling package's file must be dropped, the shared path kept"
        );
    }

    #[test]
    fn file_list_is_untouched_in_a_single_package_repo() {
        let files = vec!["anything/at/all.rs".to_string()];
        let scoped = scope_files_to_package(&scoped_pkg(), false, files.clone());
        assert_eq!(scoped, files);
    }
}
