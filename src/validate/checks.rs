use std::collections::HashMap;

use crate::config::{Config, FileFormat};
use crate::formats::get_handler;

use super::result::{ValidationEntry, ValidationLevel};
use super::source::FileSource;

pub(super) fn check_duplicate_names(config: &Config) -> Vec<ValidationEntry> {
    let mut seen: HashMap<&str, &str> = HashMap::new();
    let mut entries = Vec::new();
    for pkg in &config.packages {
        if let Some(prev_path) = seen.insert(&pkg.name, &pkg.path) {
            entries.push(ValidationEntry {
                level: ValidationLevel::Error,
                path: "(config)".to_string(),
                message: format!(
                    "duplicate package name \"{}\" (paths: \"{}\", \"{}\")",
                    pkg.name, prev_path, pkg.path
                ),
            });
        }
    }
    entries
}

pub(super) fn check_duplicate_paths(config: &Config) -> Vec<ValidationEntry> {
    let mut seen: HashMap<String, &str> = HashMap::new();
    let mut entries = Vec::new();
    for pkg in &config.packages {
        let normalized = pkg.path.trim_end_matches('/').to_string();
        if let Some(prev_name) = seen.insert(normalized, &pkg.name) {
            entries.push(ValidationEntry {
                level: ValidationLevel::Error,
                path: "(config)".to_string(),
                message: format!(
                    "duplicate package path \"{}\" (packages: \"{}\", \"{}\")",
                    pkg.path, prev_name, pkg.name
                ),
            });
        }
    }
    entries
}

pub(super) fn check_tag_templates(config: &Config) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    let is_monorepo = config.packages.len() > 1;

    let mut check_template = |template: &str, context: &str| {
        if !template.contains("{version}") {
            entries.push(ValidationEntry {
                level: ValidationLevel::Error,
                path: context.to_string(),
                message: format!("tag template \"{template}\" must contain {{version}}"),
            });
        }
        if is_monorepo && !template.contains("{name}") {
            entries.push(ValidationEntry {
                level: ValidationLevel::Warning,
                path: context.to_string(),
                message: format!(
                    "tag template \"{template}\" does not contain {{name}} — tags will collide in monorepo"
                ),
            });
        }
    };

    if let Some(ref tmpl) = config.workspace.tag_template {
        check_template(tmpl, "workspace.tagTemplate");
    }
    for pkg in &config.packages {
        if let Some(ref tmpl) = pkg.tag_template {
            check_template(tmpl, &format!("{}.tagTemplate", pkg.name));
        }
    }

    let mut check_latest = |template: &str, context: &str| {
        if template.contains("{version}") {
            entries.push(ValidationEntry {
                level: ValidationLevel::Error,
                path: context.to_string(),
                message: format!(
                    "latest tag template \"{template}\" contains {{version}}, which is not substituted: the alias is a name, not a version"
                ),
            });
        }
        if is_monorepo && !template.contains("{name}") {
            entries.push(ValidationEntry {
                level: ValidationLevel::Warning,
                path: context.to_string(),
                message: format!(
                    "latest tag template \"{template}\" does not contain {{name}}: every package would overwrite the same ref"
                ),
            });
        }
    };

    if let Some(ref tmpl) = config.workspace.latest_tag {
        check_latest(tmpl, "workspace.latestTag");
    }
    for pkg in &config.packages {
        if let Some(ref tmpl) = pkg.latest_tag {
            check_latest(tmpl, &format!("{}.latestTag", pkg.name));
        }
    }
    entries
}

pub(super) fn check_package_paths(
    config: &Config,
    source: &dyn FileSource,
) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    for pkg in &config.packages {
        if pkg.path == "." {
            continue;
        }
        match source.path_exists(&pkg.path) {
            Ok(true) => {}
            Ok(false) => entries.push(ValidationEntry {
                level: ValidationLevel::Error,
                path: pkg.path.clone(),
                message: format!("package path \"{}\" does not exist", pkg.path),
            }),
            Err(e) => entries.push(ValidationEntry {
                level: ValidationLevel::Error,
                path: pkg.path.clone(),
                message: format!("cannot check path \"{}\": {e}", pkg.path),
            }),
        }
    }
    entries
}

pub(super) fn check_versioned_files_exist(
    config: &Config,
    source: &dyn FileSource,
) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    for pkg in &config.packages {
        for vf in &pkg.versioned_files {
            match source.path_exists(&vf.path) {
                Ok(true) => {}
                Ok(false) => entries.push(ValidationEntry {
                    level: ValidationLevel::Error,
                    path: vf.path.clone(),
                    message: format!("versioned file \"{}\" does not exist", vf.path),
                }),
                Err(e) => entries.push(ValidationEntry {
                    level: ValidationLevel::Error,
                    path: vf.path.clone(),
                    message: format!("cannot check file \"{}\": {e}", vf.path),
                }),
            }
        }
    }
    entries
}

type PackageVersionMap = HashMap<String, Vec<(String, String)>>;

pub(super) fn check_versioned_files(
    config: &Config,
    source: &dyn FileSource,
) -> (Vec<ValidationEntry>, PackageVersionMap) {
    let mut entries = Vec::new();
    let mut versions: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for pkg in &config.packages {
        for vf in &pkg.versioned_files {
            if vf.format == FileFormat::GoMod {
                entries.push(ValidationEntry {
                    level: ValidationLevel::Warning,
                    path: vf.path.clone(),
                    message:
                        "go.mod version is derived from git tags, cannot validate file content"
                            .to_string(),
                });
                continue;
            }

            let content = match source.read_file(&vf.path) {
                Ok(Some(c)) => c,
                Ok(None) => continue,
                Err(e) => {
                    entries.push(ValidationEntry {
                        level: ValidationLevel::Error,
                        path: vf.path.clone(),
                        message: format!("cannot read \"{}\": {e}", vf.path),
                    });
                    continue;
                }
            };

            let handler = get_handler(&vf.format);
            match handler.read_version_from_bytes(&content, &vf.path) {
                Ok(version) => {
                    versions
                        .entry(pkg.name.clone())
                        .or_default()
                        .push((vf.path.clone(), version));
                }
                Err(e) => entries.push(ValidationEntry {
                    level: ValidationLevel::Error,
                    path: vf.path.clone(),
                    message: format!("cannot read version from \"{}\": {e}", vf.path),
                }),
            }
        }
    }
    (entries, versions)
}

pub(super) fn check_version_consistency(
    versions: &HashMap<String, Vec<(String, String)>>,
) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    for (pkg_name, file_versions) in versions {
        if file_versions.len() < 2 {
            continue;
        }
        let first_version = &file_versions[0].1;
        for (file_path, version) in &file_versions[1..] {
            if version != first_version {
                entries.push(ValidationEntry {
                    level: ValidationLevel::Error,
                    path: pkg_name.clone(),
                    message: format!(
                        "version mismatch: \"{}\" has \"{}\", \"{}\" has \"{}\"",
                        file_versions[0].0, first_version, file_path, version
                    ),
                });
            }
        }
    }
    entries
}

pub(super) fn check_changelog_paths(
    config: &Config,
    source: &dyn FileSource,
) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    for pkg in &config.packages {
        if let Some(ref changelog) = pkg.changelog {
            match source.path_exists(changelog) {
                Ok(true) => {}
                Ok(false) => entries.push(ValidationEntry {
                    level: ValidationLevel::Warning,
                    path: changelog.clone(),
                    message: format!("changelog \"{}\" does not exist yet", changelog),
                }),
                Err(e) => entries.push(ValidationEntry {
                    level: ValidationLevel::Warning,
                    path: changelog.clone(),
                    message: format!("cannot check changelog \"{}\": {e}", changelog),
                }),
            }
        }
    }
    entries
}

pub(super) fn check_shared_paths(config: &Config, source: &dyn FileSource) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    for pkg in &config.packages {
        for sp in &pkg.shared_paths {
            match source.path_exists(sp) {
                Ok(true) => {}
                Ok(false) => entries.push(ValidationEntry {
                    level: ValidationLevel::Warning,
                    path: sp.clone(),
                    message: format!("shared path \"{}\" does not exist", sp),
                }),
                Err(e) => entries.push(ValidationEntry {
                    level: ValidationLevel::Warning,
                    path: sp.clone(),
                    message: format!("cannot check shared path \"{}\": {e}", sp),
                }),
            }
        }
    }
    entries
}

pub(super) fn check_groups(config: &Config, versions: &PackageVersionMap) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();

    if let Err(errors) = config.validate_groups() {
        for message in errors {
            entries.push(ValidationEntry {
                level: ValidationLevel::Error,
                path: "(config)".to_string(),
                message,
            });
        }
        return entries;
    }

    for group in config.package_groups() {
        if group.kind != crate::config::GroupKind::Fixed {
            continue;
        }
        let member_versions: Vec<(&String, &String)> = group
            .members
            .iter()
            .filter_map(|name| {
                versions
                    .get(name)
                    .and_then(|f| f.first())
                    .map(|(_, v)| (name, v))
            })
            .collect();
        if let Some((_, first)) = member_versions.first()
            && member_versions.iter().any(|(_, v)| v != first)
        {
            let listed = member_versions
                .iter()
                .map(|(name, v)| format!("{name}={v}"))
                .collect::<Vec<_>>()
                .join(", ");
            entries.push(ValidationEntry {
                level: ValidationLevel::Warning,
                path: "(config)".to_string(),
                message: format!(
                    "fixed group has drifted versions ({listed}); the next release will realign them"
                ),
            });
        }
    }

    entries
}

pub(super) fn check_suggestions(config: &Config) -> Vec<ValidationEntry> {
    let mut entries = Vec::new();
    for pkg in &config.packages {
        if pkg.versioned_files.is_empty() {
            entries.push(ValidationEntry {
                level: ValidationLevel::Suggestion,
                path: pkg.name.clone(),
                message: "no versionedFiles declared, ferrflow will use auto-detection".to_string(),
            });
        }
    }
    if config.workspace.tag_template.is_none() {
        let default = if config.packages.len() > 1 {
            "{name}@v{version}"
        } else {
            "v{version}"
        };
        entries.push(ValidationEntry {
            level: ValidationLevel::Suggestion,
            path: "workspace.tagTemplate".to_string(),
            message: format!("not set, using default \"{default}\""),
        });
    }
    entries
}
