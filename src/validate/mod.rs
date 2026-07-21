use std::collections::BTreeMap;
use std::path::Path;

use crate::config::Config;

#[cfg(feature = "cli")]
use crate::error_code::{self, ErrorCodeExt};
#[cfg(feature = "cli")]
use crate::git::{get_repo_root, open_repo};
#[cfg(feature = "cli")]
use anyhow::Result;

mod checks;
#[cfg(feature = "cli")]
mod output;
mod result;
mod source;

#[allow(unused_imports)]
pub use result::{EntryOutput, ValidationEntry, ValidationLevel, ValidationResult};
pub use source::{FileSource, LocalSource, MemorySource, load_config_from_source};
#[cfg(feature = "cli")]
pub use source::{GitHubSource, GitLabSource, RemoteProvider, parse_repo_spec};

use checks::{
    check_changelog_paths, check_duplicate_names, check_duplicate_paths, check_groups,
    check_package_paths, check_shared_paths, check_suggestions, check_tag_templates,
    check_version_consistency, check_versioned_files, check_versioned_files_exist,
};
#[cfg(feature = "cli")]
use output::output_result;

#[cfg(feature = "cli")]
pub fn run(
    config_path: Option<&Path>,
    json: bool,
    repo: Option<&str>,
    git_ref: Option<&str>,
) -> Result<()> {
    if git_ref.is_some() && repo.is_none() {
        Err(anyhow::anyhow!("--ref requires --repo"))
            .error_code(error_code::VALIDATE_REF_REQUIRES_REPO)?;
    }

    let source: Box<dyn FileSource> = if let Some(repo_spec) = repo {
        let (provider, owner, repo_name) = parse_repo_spec(repo_spec)?;
        match provider {
            RemoteProvider::GitHub => Box::new(GitHubSource {
                owner,
                repo: repo_name,
                git_ref: git_ref.map(|s| s.to_string()),
                token: std::env::var("FERRFLOW_TOKEN")
                    .ok()
                    .or_else(|| std::env::var("GITHUB_TOKEN").ok()),
            }),
            RemoteProvider::GitLab => Box::new(GitLabSource {
                owner,
                repo: repo_name,
                git_ref: git_ref.map(|s| s.to_string()),
                token: std::env::var("FERRFLOW_TOKEN")
                    .ok()
                    .or_else(|| std::env::var("GITLAB_TOKEN").ok()),
            }),
        }
    } else {
        let repo = open_repo(&std::env::current_dir()?)?;
        let root = get_repo_root(&repo)?;
        Box::new(LocalSource { root })
    };

    let config_path_str = config_path.and_then(|p| p.to_str());

    let (config, config_filename) = match load_config_from_source(source.as_ref(), config_path_str)
    {
        Ok(result) => result,
        Err(e) => {
            let mut result = ValidationResult::from_entries(vec![ValidationEntry {
                level: ValidationLevel::Error,
                path: "(config)".to_string(),
                message: e.to_string(),
            }]);
            result.config_file = None;
            return output_result(&result, json);
        }
    };

    let entries = collect_entries(&config, source.as_ref());

    let mut result = ValidationResult::from_entries(entries);
    result.config_file = Some(config_filename);
    result.package_count = config.packages.len();
    output_result(&result, json)
}

fn collect_entries(
    config: &crate::config::Config,
    source: &dyn FileSource,
) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    entries.extend(check_duplicate_names(config));
    entries.extend(check_duplicate_paths(config));
    entries.extend(check_tag_templates(config));
    entries.extend(check_package_paths(config, source));
    entries.extend(check_versioned_files_exist(config, source));
    let (file_entries, versions) = check_versioned_files(config, source);
    entries.extend(file_entries);
    entries.extend(check_version_consistency(&versions));
    entries.extend(check_changelog_paths(config, source));
    entries.extend(check_shared_paths(config, source));
    entries.extend(check_groups(config, &versions));
    entries.extend(check_suggestions(config));
    entries
}

#[cfg_attr(feature = "cli", allow(dead_code))]
pub fn validate_files(config: &Config, files: BTreeMap<String, Vec<u8>>) -> ValidationResult {
    let source = MemorySource::new(files);
    let mut result = ValidationResult::from_entries(collect_entries(config, &source));
    result.package_count = config.packages.len();
    result
}

pub fn local_entries(config: &Config, root: &Path) -> Vec<ValidationEntry> {
    let source = LocalSource {
        root: root.to_path_buf(),
    };
    collect_entries(config, &source)
}

#[cfg(test)]
mod tests;
