use std::collections::HashSet;

use crate::config::PackageConfig;

pub(super) fn tags_for_package<'a>(all_tags: &'a [String], prefix: &str) -> Vec<&'a str> {
    all_tags
        .iter()
        .filter(|t| t.starts_with(prefix))
        .map(|t| t.as_str())
        .collect()
}

pub(super) fn pick_higher_semver(file: &str, tag: &str) -> String {
    let file_clean = file.trim_start_matches('v');
    let tag_clean = tag.trim_start_matches('v');
    match (
        semver::Version::parse(file_clean),
        semver::Version::parse(tag_clean),
    ) {
        (Ok(f), Ok(t)) => {
            if t >= f {
                tag.to_string()
            } else {
                file.to_string()
            }
        }
        (Ok(_), Err(_)) => file.to_string(),
        (Err(_), Ok(_)) => tag.to_string(),
        (Err(_), Err(_)) => file.to_string(),
    }
}

pub(super) fn collect_dirty_files(repo: &git2::Repository) -> HashSet<String> {
    let mut files = HashSet::new();
    if let Ok(statuses) = repo.statuses(None) {
        for entry in statuses.iter() {
            let status = entry.status();
            if status.intersects(
                git2::Status::WT_MODIFIED
                    | git2::Status::WT_NEW
                    | git2::Status::WT_TYPECHANGE
                    | git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED,
            ) && let Some(path) = entry.path()
            {
                files.insert(path.to_string());
            }
        }
    }
    files
}

pub(super) fn auto_stage_new_files(
    repo: &git2::Repository,
    before: &HashSet<String>,
    files_to_commit: &mut Vec<String>,
) {
    let after = collect_dirty_files(repo);
    for path in after.difference(before) {
        if !files_to_commit.contains(path) {
            files_to_commit.push(path.clone());
        }
    }
}

pub(super) fn is_package_touched(
    pkg: &PackageConfig,
    changed_files: &[String],
    is_monorepo: bool,
) -> bool {
    if !is_monorepo {
        return true;
    }

    let pkg_path = pkg.path.trim_start_matches("./").trim_end_matches('/');

    if pkg_path == "." || pkg_path.is_empty() {
        return true;
    }

    let prefix = format!("{pkg_path}/");
    if changed_files.iter().any(|f| f.starts_with(&prefix)) {
        return true;
    }

    for shared in &pkg.shared_paths {
        let shared = shared.trim_end_matches('/');
        if changed_files
            .iter()
            .any(|f| f.starts_with(shared) || f == shared)
        {
            return true;
        }
    }

    false
}
