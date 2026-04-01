use crate::config::Config;
use crate::formats::read_version;
use crate::git::{find_last_tag_name, get_repo_root, open_repo};
use anyhow::{Result, bail};
use serde::Serialize;

#[derive(Serialize)]
struct VersionEntry {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct TagEntry {
    name: String,
    tag: Option<String>,
}

pub fn version(
    config_path: Option<&std::path::Path>,
    package: Option<&str>,
    json: bool,
) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if config.packages.is_empty() {
        bail!("No packages configured. Run `ferrflow init` to create a config.");
    }

    if let Some(name) = package {
        let pkg = config
            .packages
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow::anyhow!("package '{}' not found", name))?;

        let version = pkg
            .versioned_files
            .first()
            .map(|vf| read_version(vf, &root))
            .transpose()?
            .unwrap_or_else(|| "unknown".to_string());

        if json {
            println!(
                "{}",
                serde_json::to_string(&VersionEntry {
                    name: pkg.name.clone(),
                    version,
                })?
            );
        } else {
            println!("{version}");
        }
        return Ok(());
    }

    // No package specified
    if config.packages.len() == 1 {
        let pkg = &config.packages[0];
        let version = pkg
            .versioned_files
            .first()
            .map(|vf| read_version(vf, &root))
            .transpose()?
            .unwrap_or_else(|| "unknown".to_string());

        if json {
            println!(
                "{}",
                serde_json::to_string(&VersionEntry {
                    name: pkg.name.clone(),
                    version,
                })?
            );
        } else {
            println!("{version}");
        }
    } else {
        let entries: Vec<VersionEntry> = config
            .packages
            .iter()
            .map(|pkg| {
                let version = pkg
                    .versioned_files
                    .first()
                    .and_then(|vf| read_version(vf, &root).ok())
                    .unwrap_or_else(|| "unknown".to_string());
                VersionEntry {
                    name: pkg.name.clone(),
                    version,
                }
            })
            .collect();

        if json {
            println!("{}", serde_json::to_string(&entries)?);
        } else {
            for e in &entries {
                println!("{}\t{}", e.name, e.version);
            }
        }
    }

    Ok(())
}

pub fn tag(config_path: Option<&std::path::Path>, package: Option<&str>, json: bool) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if config.packages.is_empty() {
        bail!("No packages configured. Run `ferrflow init` to create a config.");
    }

    if let Some(name) = package {
        let pkg = config
            .packages
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow::anyhow!("package '{}' not found", name))?;

        let prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
        let last_tag = find_last_tag_name(&repo, &prefix, config.workspace.orphaned_tag_strategy)?;

        if json {
            println!(
                "{}",
                serde_json::to_string(&TagEntry {
                    name: pkg.name.clone(),
                    tag: last_tag,
                })?
            );
        } else {
            println!("{}", last_tag.unwrap_or_else(|| "none".to_string()));
        }
        return Ok(());
    }

    // No package specified
    if config.packages.len() == 1 {
        let pkg = &config.packages[0];
        let prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
        let last_tag = find_last_tag_name(&repo, &prefix, config.workspace.orphaned_tag_strategy)?;

        if json {
            println!(
                "{}",
                serde_json::to_string(&TagEntry {
                    name: pkg.name.clone(),
                    tag: last_tag,
                })?
            );
        } else {
            println!("{}", last_tag.unwrap_or_else(|| "none".to_string()));
        }
    } else {
        let entries: Vec<TagEntry> = config
            .packages
            .iter()
            .map(|pkg| {
                let prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
                let tag =
                    find_last_tag_name(&repo, &prefix, config.workspace.orphaned_tag_strategy)
                        .unwrap_or(None);
                TagEntry {
                    name: pkg.name.clone(),
                    tag,
                }
            })
            .collect();

        if json {
            println!("{}", serde_json::to_string(&entries)?);
        } else {
            for e in &entries {
                println!("{}\t{}", e.name, e.tag.as_deref().unwrap_or("none"));
            }
        }
    }

    Ok(())
}
