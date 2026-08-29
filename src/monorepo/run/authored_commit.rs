use anyhow::{Context, Result};
use base64::Engine;
use std::path::Path;

use crate::forge::{AuthoredCommit, FileAddition, Forge};
use crate::git::Repository;

pub(super) fn file_changes(
    root: &Path,
    files: &[&str],
) -> Result<(Vec<FileAddition>, Vec<String>)> {
    let mut additions = Vec::new();
    let mut deletions = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // The bump loop pushes a path once per versionedFiles entry, and a file
    // carrying several selectors is a supported shape, so the same path
    // arrives here more than once. `git add` never minded; the API refuses
    // the whole commit over it. The contents are read from disk per path, so
    // the copies are identical and the first one is the file.
    for file in files {
        if !seen.insert(to_forge_path(file)) {
            continue;
        }

        let path = root.join(file);
        if path.exists() {
            let bytes = std::fs::read(&path).with_context(|| {
                format!("Failed to read {} for the release commit", path.display())
            })?;
            additions.push(FileAddition {
                path: to_forge_path(file),
                base64_contents: base64::engine::general_purpose::STANDARD.encode(&bytes),
            });
        } else {
            deletions.push(to_forge_path(file));
        }
    }

    Ok((additions, deletions))
}

/// The API addresses files from the repository root with forward slashes,
/// whatever the platform git handed us.
fn to_forge_path(file: &str) -> String {
    file.replace('\\', "/")
}

#[allow(clippy::too_many_arguments)]
pub(super) fn author_on_branch(
    forge: &dyn Forge,
    repo: &Repository,
    root: &Path,
    remote: &str,
    branch: &str,
    expected_head_oid: &str,
    files: &[&str],
    message: &str,
) -> Result<String> {
    let (additions, deletions) = file_changes(root, files)?;
    let oid = forge.create_commit_on_branch(&AuthoredCommit {
        branch,
        expected_head_oid,
        message,
        additions,
        deletions,
    })?;

    // The commit exists on the forge and not here. Everything downstream tags
    // and pushes from the local repository, so without this the tag would land
    // on the commit before the bump.
    crate::git::reset_branch_to_remote(repo, remote, branch)?;
    Ok(oid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_present_file_is_an_addition_and_a_missing_one_is_a_deletion() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "version = \"1.1.0\"").unwrap();

        let (additions, deletions) = file_changes(dir.path(), &["Cargo.toml", "gone.txt"]).unwrap();

        assert_eq!(additions.len(), 1);
        assert_eq!(additions[0].path, "Cargo.toml");
        assert_eq!(deletions, vec!["gone.txt".to_string()]);
    }

    #[test]
    fn contents_are_base64_encoded_for_the_api() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("f.txt"), "hello").unwrap();

        let (additions, _) = file_changes(dir.path(), &["f.txt"]).unwrap();

        assert_eq!(additions[0].base64_contents, "aGVsbG8=");
    }

    #[test]
    fn binary_contents_survive_the_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let bytes: Vec<u8> = (0u8..=255).collect();
        std::fs::write(dir.path().join("blob.bin"), &bytes).unwrap();

        let (additions, _) = file_changes(dir.path(), &["blob.bin"]).unwrap();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&additions[0].base64_contents)
            .unwrap();

        assert_eq!(decoded, bytes);
    }

    // Three selectors on one Chart.yaml is how a Helm chart bumps its
    // version, appVersion and image line, and the API refuses a commit whose
    // fileChanges name a path twice. This is the shape that failed LFSX's
    // v1.5.1 release under v7.9.0.
    #[test]
    fn a_path_named_by_several_entries_is_sent_once() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Chart.yaml"), "version: 1.2.3").unwrap();

        let (additions, deletions) = file_changes(
            dir.path(),
            &[
                "Chart.yaml",
                "Chart.yaml",
                "Chart.yaml",
                "gone.txt",
                "gone.txt",
            ],
        )
        .unwrap();

        assert_eq!(additions.len(), 1);
        assert_eq!(additions[0].path, "Chart.yaml");
        assert_eq!(deletions, vec!["gone.txt".to_string()]);
    }

    // The API judges uniqueness on the normalised path, so the guard has to
    // as well: a backslashed lockfile path next to a forward-slashed config
    // path is one file to GitHub and was two to a dedupe keyed on the raw
    // string.
    #[test]
    fn separator_variants_of_one_file_are_sent_once() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("crates/api")).unwrap();
        std::fs::write(dir.path().join("crates/api/Cargo.toml"), "x").unwrap();

        let (additions, deletions) = file_changes(
            dir.path(),
            &["crates/api/Cargo.toml", r"crates\api\Cargo.toml"],
        )
        .unwrap();

        assert_eq!(additions.len(), 1);
        assert_eq!(additions[0].path, "crates/api/Cargo.toml");
        assert!(deletions.is_empty(), "{deletions:?}");
    }

    #[test]
    fn nested_paths_are_sent_with_forward_slashes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("crates/api")).unwrap();
        std::fs::write(dir.path().join("crates/api/Cargo.toml"), "x").unwrap();

        let (additions, _) = file_changes(dir.path(), &["crates/api/Cargo.toml"]).unwrap();

        assert_eq!(additions[0].path, "crates/api/Cargo.toml");
    }
}
