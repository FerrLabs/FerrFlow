use anyhow::Result;
use std::path::Path;

use crate::error_code::{self, ErrorCodeExt};
use crate::git::{get_repo_root, open_repo};

mod checks;
mod output;
mod result;
mod source;

#[allow(unused_imports)]
pub use result::{EntryOutput, ValidationEntry, ValidationLevel, ValidationResult};
pub use source::{
    FileSource, GitHubSource, GitLabSource, LocalSource, RemoteProvider, load_config_from_source,
    parse_repo_spec,
};

use checks::{
    check_changelog_paths, check_duplicate_names, check_duplicate_paths, check_package_paths,
    check_shared_paths, check_suggestions, check_tag_templates, check_version_consistency,
    check_versioned_files, check_versioned_files_exist,
};
use output::output_result;

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

    let mut entries = Vec::new();
    entries.extend(check_duplicate_names(&config));
    entries.extend(check_duplicate_paths(&config));
    entries.extend(check_tag_templates(&config));
    entries.extend(check_package_paths(&config, source.as_ref()));
    entries.extend(check_versioned_files_exist(&config, source.as_ref()));
    let (file_entries, versions) = check_versioned_files(&config, source.as_ref());
    entries.extend(file_entries);
    entries.extend(check_version_consistency(&versions));
    entries.extend(check_changelog_paths(&config, source.as_ref()));
    entries.extend(check_shared_paths(&config, source.as_ref()));
    entries.extend(check_suggestions(&config));

    let mut result = ValidationResult::from_entries(entries);
    result.config_file = Some(config_filename);
    result.package_count = config.packages.len();
    output_result(&result, json)
}

#[cfg(test)]
mod tests;
