use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::error_code::{self, ErrorCodeExt};
use crate::formats::read_version;

const MANIFEST_SCHEMA_URL: &str = "https://ferrflow.com/schema/manifest.json";
const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: u32,
    pub packages: BTreeMap<String, String>,
    pub released_at: String,
    pub commit: String,
}

impl Manifest {
    pub fn new(packages: BTreeMap<String, String>, released_at: String, commit: String) -> Self {
        Self {
            schema: MANIFEST_SCHEMA_URL.to_string(),
            version: MANIFEST_VERSION,
            packages,
            released_at,
            commit,
        }
    }

    pub fn version_of(&self, package: &str) -> Option<&str> {
        self.packages.get(package).map(String::as_str)
    }
}

pub fn now_utc_iso8601() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn manifest_path(config: &Config, root: &Path) -> Option<PathBuf> {
    config
        .workspace
        .manifest_file
        .as_ref()
        .map(|rel| root.join(rel))
}

pub fn read(path: &Path) -> Result<Manifest> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read manifest at {}", path.display()))
        .error_code(error_code::CONFIG_READ_FAILED)?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse manifest at {}", path.display()))
        .error_code(error_code::CONFIG_READ_FAILED)
}

pub fn read_if_present(path: &Path) -> Result<Option<Manifest>> {
    if path.exists() {
        read(path).map(Some)
    } else {
        Ok(None)
    }
}

pub fn write_atomic(path: &Path, manifest: &Manifest) -> Result<()> {
    let mut serialized = serde_json::to_string_pretty(manifest)?;
    serialized.push('\n');
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).ok();
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("manifest");
    let tmp_path = path.with_file_name(format!("{file_name}.{}.tmp", std::process::id()));
    std::fs::write(&tmp_path, serialized.as_bytes())
        .with_context(|| format!("failed to write manifest temp file {}", tmp_path.display()))?;
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e)
            .with_context(|| format!("failed to move manifest into place at {}", path.display()));
    }
    Ok(())
}

pub fn snapshot_from_files(config: &Config, root: &Path) -> BTreeMap<String, String> {
    snapshot_with_overrides(config, root, &BTreeMap::new())
}

pub fn snapshot_with_overrides(
    config: &Config,
    root: &Path,
    overrides: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut packages = BTreeMap::new();
    for pkg in &config.packages {
        let version = overrides.get(&pkg.name).cloned().or_else(|| {
            pkg.versioned_files
                .first()
                .and_then(|vf| read_version(vf, root).ok())
        });
        if let Some(version) = version {
            packages.insert(pkg.name.clone(), version);
        }
    }
    packages
}

pub fn initial_snapshot(config: &Config, root: &Path) -> BTreeMap<String, String> {
    config
        .packages
        .iter()
        .map(|pkg| {
            let version = pkg
                .versioned_files
                .first()
                .and_then(|vf| read_version(vf, root).ok())
                .unwrap_or_else(|| "0.0.0".to_string());
            (pkg.name.clone(), version)
        })
        .collect()
}

#[derive(Debug, PartialEq)]
pub enum Divergence {
    VersionMismatch {
        package: String,
        manifest: String,
        file: String,
    },
    MissingFromManifest(String),
    NotInConfig(String),
    PackageUnreadable(String),
}

impl Divergence {
    fn describe(&self) -> String {
        match self {
            Divergence::VersionMismatch {
                package,
                manifest,
                file,
            } => format!("{package}: manifest says {manifest} but file says {file}"),
            Divergence::MissingFromManifest(pkg) => {
                format!("{pkg}: configured package missing from manifest")
            }
            Divergence::NotInConfig(pkg) => {
                format!("{pkg}: present in manifest but not in config")
            }
            Divergence::PackageUnreadable(pkg) => {
                format!("{pkg}: could not read version from its files")
            }
        }
    }
}

pub fn diff(manifest: &Manifest, config: &Config, root: &Path) -> Vec<Divergence> {
    let mut divergences = Vec::new();
    let mut configured = std::collections::HashSet::new();

    for pkg in &config.packages {
        configured.insert(pkg.name.as_str());
        let Some(vf) = pkg.versioned_files.first() else {
            continue;
        };
        let manifest_version = manifest.version_of(&pkg.name);
        match (read_version(vf, root).ok(), manifest_version) {
            (Some(file), Some(manifest_ver)) if file != manifest_ver => {
                divergences.push(Divergence::VersionMismatch {
                    package: pkg.name.clone(),
                    manifest: manifest_ver.to_string(),
                    file,
                });
            }
            (Some(_), Some(_)) => {}
            (Some(_), None) => {
                divergences.push(Divergence::MissingFromManifest(pkg.name.clone()));
            }
            (None, _) => {
                divergences.push(Divergence::PackageUnreadable(pkg.name.clone()));
            }
        }
    }

    for name in manifest.packages.keys() {
        if !configured.contains(name.as_str()) {
            divergences.push(Divergence::NotInConfig(name.clone()));
        }
    }

    divergences
}

pub fn validate_in_sync(config: &Config, root: &Path) -> Result<()> {
    let Some(path) = manifest_path(config, root) else {
        return Ok(());
    };
    let Some(manifest) = read_if_present(&path)? else {
        return Ok(());
    };
    let divergences = diff(&manifest, config, root);
    if divergences.is_empty() {
        return Ok(());
    }
    let detail = divergences
        .iter()
        .map(Divergence::describe)
        .collect::<Vec<_>>()
        .join("\n  ");
    Err(anyhow::anyhow!(
        "manifest out of sync with package files — run `ferrflow sync-manifest` to repair\n  {detail}"
    ))
    .error_code(error_code::CONFIG_READ_FAILED)
}

pub fn sync_cwd(config_path: Option<&Path>) -> Result<()> {
    use crate::git::{get_repo_root, open_repo};

    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    let Some(path) = manifest_path(&config, &root) else {
        return Err(anyhow::anyhow!(
            "manifest mode is not enabled — set `workspace.manifest_file` in your config first"
        ))
        .error_code(error_code::CONFIG_NOT_FOUND);
    };

    let packages = snapshot_from_files(&config, &root);
    let commit = repo
        .head_id()
        .ok()
        .map(|id| {
            let full = id.to_string();
            full[..7.min(full.len())].to_string()
        })
        .unwrap_or_default();
    let manifest = Manifest::new(packages, now_utc_iso8601(), commit);
    write_atomic(&path, &manifest)?;
    tracing::info!(
        "Wrote {} package version(s) to {}",
        manifest.packages.len(),
        path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FileFormat, PackageConfig, VersionedFile};

    fn pkg(name: &str, path: &str) -> PackageConfig {
        PackageConfig {
            name: name.to_string(),
            path: ".".to_string(),
            versioned_files: vec![VersionedFile {
                path: path.to_string(),
                format: FileFormat::Json,
                selector: None,
            }],
            changelog: None,
            shared_paths: Vec::new(),
            depends_on: vec![],
            versioning: None,
            tag_template: None,
            hooks: None,
            floating_tags: None,
            publishers: vec![],
            update_lockfiles: None,
        }
    }

    fn write_pkg_json(root: &Path, path: &str, version: &str) {
        std::fs::write(
            root.join(path),
            format!("{{\"name\": \"x\", \"version\": \"{version}\"}}"),
        )
        .unwrap();
    }

    fn config_with(packages: Vec<PackageConfig>, manifest_file: Option<&str>) -> Config {
        let mut config = Config {
            packages,
            ..Default::default()
        };
        config.workspace.manifest_file = manifest_file.map(str::to_string);
        config
    }

    #[test]
    fn manifest_serializes_sorted_with_schema_and_version() {
        let mut packages = BTreeMap::new();
        packages.insert("web".to_string(), "2.0.0".to_string());
        packages.insert("api".to_string(), "1.3.0".to_string());
        let manifest = Manifest::new(
            packages,
            "2026-05-22T18:30:00Z".to_string(),
            "abc1234".to_string(),
        );
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let api_at = json.find("\"api\"").unwrap();
        let web_at = json.find("\"web\"").unwrap();
        assert!(api_at < web_at, "packages must be sorted");
        assert!(json.contains("\"$schema\": \"https://ferrflow.com/schema/manifest.json\""));
        assert!(json.contains("\"version\": 1"));
    }

    #[test]
    fn write_atomic_round_trips_and_leaves_no_temp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".ferrflow.manifest.json");
        let mut packages = BTreeMap::new();
        packages.insert("api".to_string(), "1.0.0".to_string());
        let manifest = Manifest::new(packages, "t".to_string(), "sha".to_string());
        write_atomic(&path, &manifest).unwrap();
        let back = read(&path).unwrap();
        assert_eq!(back, manifest);
        let leftovers = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("tmp"))
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn snapshot_reads_every_configured_package() {
        let dir = tempfile::tempdir().unwrap();
        write_pkg_json(dir.path(), "api.json", "1.3.0");
        write_pkg_json(dir.path(), "web.json", "2.0.0");
        let config = config_with(
            vec![pkg("api", "api.json"), pkg("web", "web.json")],
            Some(".ferrflow.manifest.json"),
        );
        let snap = snapshot_from_files(&config, dir.path());
        assert_eq!(snap.get("api").map(String::as_str), Some("1.3.0"));
        assert_eq!(snap.get("web").map(String::as_str), Some("2.0.0"));
    }

    #[test]
    fn diff_flags_version_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        write_pkg_json(dir.path(), "api.json", "1.1.0");
        let config = config_with(vec![pkg("api", "api.json")], Some("m.json"));
        let mut packages = BTreeMap::new();
        packages.insert("api".to_string(), "1.0.0".to_string());
        let manifest = Manifest::new(packages, "t".to_string(), "sha".to_string());
        let divergences = diff(&manifest, &config, dir.path());
        assert_eq!(
            divergences,
            vec![Divergence::VersionMismatch {
                package: "api".to_string(),
                manifest: "1.0.0".to_string(),
                file: "1.1.0".to_string(),
            }]
        );
    }

    #[test]
    fn diff_clean_when_aligned() {
        let dir = tempfile::tempdir().unwrap();
        write_pkg_json(dir.path(), "api.json", "1.0.0");
        let config = config_with(vec![pkg("api", "api.json")], Some("m.json"));
        let mut packages = BTreeMap::new();
        packages.insert("api".to_string(), "1.0.0".to_string());
        let manifest = Manifest::new(packages, "t".to_string(), "sha".to_string());
        assert!(diff(&manifest, &config, dir.path()).is_empty());
    }

    #[test]
    fn diff_flags_missing_and_extra_packages() {
        let dir = tempfile::tempdir().unwrap();
        write_pkg_json(dir.path(), "api.json", "1.0.0");
        let config = config_with(vec![pkg("api", "api.json")], Some("m.json"));
        let mut packages = BTreeMap::new();
        packages.insert("ghost".to_string(), "9.9.9".to_string());
        let manifest = Manifest::new(packages, "t".to_string(), "sha".to_string());
        let divergences = diff(&manifest, &config, dir.path());
        assert!(divergences.contains(&Divergence::MissingFromManifest("api".to_string())));
        assert!(divergences.contains(&Divergence::NotInConfig("ghost".to_string())));
    }

    #[test]
    fn validate_in_sync_passes_when_manifest_absent() {
        let dir = tempfile::tempdir().unwrap();
        write_pkg_json(dir.path(), "api.json", "1.0.0");
        let config = config_with(
            vec![pkg("api", "api.json")],
            Some(".ferrflow.manifest.json"),
        );
        assert!(validate_in_sync(&config, dir.path()).is_ok());
    }

    #[test]
    fn validate_in_sync_noop_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let config = config_with(vec![pkg("api", "api.json")], None);
        assert!(validate_in_sync(&config, dir.path()).is_ok());
    }

    #[test]
    fn validate_in_sync_errors_on_divergence() {
        let dir = tempfile::tempdir().unwrap();
        write_pkg_json(dir.path(), "api.json", "1.1.0");
        let path = dir.path().join("m.json");
        let mut packages = BTreeMap::new();
        packages.insert("api".to_string(), "1.0.0".to_string());
        write_atomic(&path, &Manifest::new(packages, "t".into(), "sha".into())).unwrap();
        let config = config_with(vec![pkg("api", "api.json")], Some("m.json"));
        let err = validate_in_sync(&config, dir.path()).unwrap_err();
        assert!(format!("{err:?}").contains("sync-manifest"));
    }
}
