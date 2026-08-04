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

pub(super) fn collect_dirty_files(repo: &crate::git::Repository) -> HashSet<String> {
    let mut files = HashSet::new();
    let workdir = match repo.workdir() {
        Some(p) => p,
        None => return files,
    };
    let output = std::process::Command::new("git")
        .current_dir(workdir)
        .args(["status", "--porcelain=v1", "-z", "--no-renames"])
        .output();
    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return files,
    };
    let raw = String::from_utf8_lossy(&output.stdout);
    for entry in raw.split('\0') {
        if entry.len() < 4 {
            continue;
        }
        let xy = &entry[..2];
        let path = entry[3..].to_string();
        let x = xy.as_bytes()[0];
        let y = xy.as_bytes()[1];
        let modified = matches!(y, b'M' | b'?' | b'T' | b'A') || matches!(x, b'A' | b'M');
        if modified {
            files.insert(path);
        }
    }
    files
}

pub(super) fn auto_stage_new_files(
    repo: &crate::git::Repository,
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
    pkg.is_touched_by(changed_files, is_monorepo)
}
