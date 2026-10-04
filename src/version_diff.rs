use anyhow::Result;
use colored::{ColoredString, Colorize};
use gix::ObjectId;
use serde::Serialize;
use std::io::Write;
use std::path::Path;

use crate::changelog::{ChangelogRender, build_section_with};
use crate::config::CommitFormats;
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
    let nested = config.nested_package_paths(pkg);

    let (from_oid, from_tag) =
        resolve_endpoint(&repo, pkg, &config.workspace, is_monorepo, from_ref)?;
    let (to_oid, to_tag) = resolve_endpoint(&repo, pkg, &config.workspace, is_monorepo, to_ref)?;

    let skip = config.workspace.effective_commit_skip_markers();
    let commits = get_commits_between(&repo, from_oid, to_oid, &skip, |repo, oid| {
        commit_touches_package(repo, pkg, is_monorepo, &nested, oid)
    })?;
    let files = scope_files_to_package(
        pkg,
        is_monorepo,
        &nested,
        get_changed_files_between(&repo, from_oid, to_oid).unwrap_or_default(),
    );

    let overall = commits
        .iter()
        .map(|c| determine_bump(&c.message, &config.workspace.commit_formats))
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
        formats: Some(&config.workspace.commit_formats),
        forge_base,
        last_tag: Some(from_tag.clone()),
        new_tag: Some(to_tag.clone()),
    };
    let changelog = build_section_with(&to_version, &commits, &render);

    let report = DiffReport {
        pkg,
        from: from_ref,
        to: to_ref,
        overall,
        commits: &commits,
        files: &files,
        changelog: &changelog,
        formats: &config.workspace.commit_formats,
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&json_report(&report))?);
    } else {
        write_human(&mut std::io::stdout().lock(), &report)?;
    }
    Ok(())
}

fn commit_touches_package(
    repo: &crate::git::Repository,
    pkg: &PackageConfig,
    is_monorepo: bool,
    nested: &[String],
    oid: ObjectId,
) -> bool {
    if !is_monorepo {
        return true;
    }
    match get_changed_files_for_commit(repo, oid) {
        Ok(files) => pkg.is_touched_by(&files, true, nested),
        Err(_) => true,
    }
}

fn scope_files_to_package(
    pkg: &PackageConfig,
    is_monorepo: bool,
    nested: &[String],
    files: Vec<String>,
) -> Vec<String> {
    if !is_monorepo {
        return files;
    }
    files
        .into_iter()
        .filter(|f| pkg.is_touched_by(std::slice::from_ref(f), true, nested))
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

struct DiffReport<'a> {
    pkg: &'a PackageConfig,
    from: &'a str,
    to: &'a str,
    overall: BumpType,
    commits: &'a [crate::git::GitLog],
    files: &'a [String],
    changelog: &'a str,
    formats: &'a CommitFormats,
}

fn bump_label(bump: BumpType) -> ColoredString {
    match bump {
        BumpType::Major => "major".red().bold(),
        BumpType::Minor => "minor".yellow(),
        BumpType::Patch => "patch".cyan(),
        BumpType::None => "none".dimmed(),
    }
}

fn write_human(out: &mut impl Write, r: &DiffReport<'_>) -> std::io::Result<()> {
    let DiffReport {
        pkg,
        from,
        to,
        overall,
        commits,
        files,
        changelog,
        formats,
    } = *r;

    writeln!(
        out,
        "{}  {} → {}  ({})\n",
        pkg.name.bold(),
        from.cyan(),
        to.green().bold(),
        bump_label(overall)
    )?;

    writeln!(out, "{}", format!("Commits ({})", commits.len()).bold())?;
    if commits.is_empty() {
        writeln!(out, "  {}", "(none)".dimmed())?;
    }
    for c in commits {
        writeln!(
            out,
            "  {:<5}  {}  {}",
            bump_label(determine_bump(&c.message, formats)),
            c.hash.dimmed(),
            parse_subject(&c.message)
        )?;
    }

    let breaking: Vec<&crate::git::GitLog> = commits
        .iter()
        .filter(|c| is_breaking(&c.message, formats))
        .collect();
    if !breaking.is_empty() {
        writeln!(
            out,
            "\n{}",
            format!("Breaking changes ({})", breaking.len())
                .red()
                .bold()
        )?;
        for c in &breaking {
            writeln!(out, "  {} {}", "!".red().bold(), parse_subject(&c.message))?;
        }
    }

    writeln!(
        out,
        "\n{}",
        format!("Files changed ({})", files.len()).bold()
    )?;
    for f in files.iter().take(MAX_FILES_SHOWN) {
        writeln!(out, "  {f}")?;
    }
    if files.len() > MAX_FILES_SHOWN {
        writeln!(
            out,
            "  {}",
            format!("… and {} more", files.len() - MAX_FILES_SHOWN).dimmed()
        )?;
    }

    writeln!(out, "\n{}", "Changelog".bold())?;
    for line in changelog.trim_end().lines() {
        writeln!(out, "  {line}")?;
    }
    Ok(())
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

fn json_report<'a>(r: &DiffReport<'a>) -> DiffJson<'a> {
    let DiffReport {
        pkg,
        from,
        to,
        overall,
        commits,
        files,
        changelog,
        formats,
    } = *r;

    let commit_json: Vec<CommitJson> = commits
        .iter()
        .map(|c| {
            let header = parse_header(&c.message);
            CommitJson {
                hash: c.hash.clone(),
                subject: parse_subject(&c.message).to_string(),
                commit_type: header.as_ref().map(|h| h.commit_type.to_string()),
                scope: header.as_ref().and_then(|h| h.scope.map(str::to_string)),
                breaking: is_breaking(&c.message, formats),
                bump: determine_bump(&c.message, formats).to_string(),
            }
        })
        .collect();
    let breaking: Vec<String> = commits
        .iter()
        .filter(|c| is_breaking(&c.message, formats))
        .map(|c| parse_subject(&c.message).to_string())
        .collect();

    DiffJson {
        package: &pkg.name,
        from,
        to,
        bump: overall.to_string(),
        commits: commit_json,
        breaking,
        files_changed: files,
        changelog,
    }
}

#[cfg(test)]
mod tests;
