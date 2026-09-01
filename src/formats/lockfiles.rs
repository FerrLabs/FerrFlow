use std::path::{Path, PathBuf};

use super::join_within_repo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manifest {
    CargoToml,
    PackageJson,
    PyprojectToml,
    Gemfile,
    MixExs,
}

impl Manifest {
    pub fn from_manifest_filename(filename: &str) -> Option<Self> {
        match filename {
            "Cargo.toml" => Some(Self::CargoToml),
            "package.json" => Some(Self::PackageJson),
            "pyproject.toml" => Some(Self::PyprojectToml),
            "Gemfile" => Some(Self::Gemfile),
            "mix.exs" => Some(Self::MixExs),
            _ => None,
        }
    }

    fn lockfiles(self) -> &'static [Lockfile] {
        match self {
            Self::CargoToml => &[Lockfile {
                filename: "Cargo.lock",
                program: "cargo",
                base_args: &["update"],
                update_kind: UpdateKind::PerPackage,
            }],
            Self::PackageJson => &[
                Lockfile {
                    filename: "package-lock.json",
                    program: "npm",
                    base_args: &["install", "--package-lock-only"],
                    update_kind: UpdateKind::Whole,
                },
                Lockfile {
                    filename: "pnpm-lock.yaml",
                    program: "pnpm",
                    base_args: &["install", "--lockfile-only"],
                    update_kind: UpdateKind::Whole,
                },
                Lockfile {
                    filename: "yarn.lock",
                    program: "yarn",
                    base_args: &["install", "--mode=update-lockfile"],
                    update_kind: UpdateKind::Whole,
                },
            ],
            Self::PyprojectToml => &[
                Lockfile {
                    filename: "poetry.lock",
                    program: "poetry",
                    base_args: &["lock", "--no-update"],
                    update_kind: UpdateKind::Whole,
                },
                Lockfile {
                    filename: "uv.lock",
                    program: "uv",
                    base_args: &["lock", "--offline"],
                    update_kind: UpdateKind::Whole,
                },
            ],
            Self::Gemfile => &[Lockfile {
                filename: "Gemfile.lock",
                program: "bundle",
                base_args: &["lock"],
                update_kind: UpdateKind::Whole,
            }],
            Self::MixExs => &[Lockfile {
                filename: "mix.lock",
                program: "mix",
                base_args: &["deps.get"],
                update_kind: UpdateKind::Whole,
            }],
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum UpdateKind {
    PerPackage,
    Whole,
}

#[derive(Debug, Clone, Copy)]
struct Lockfile {
    filename: &'static str,
    program: &'static str,
    base_args: &'static [&'static str],
    update_kind: UpdateKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateOutcome {
    Updated { lockfile_rel: String },
    NoLockfile,
    NotOnPath { program: String },
    UnsupportedManifest,
    Failed { program: String, detail: String },
}

pub fn update_for_manifest(repo_root: &Path, manifest_rel: &str) -> anyhow::Result<UpdateOutcome> {
    let manifest_path = join_within_repo(repo_root, manifest_rel)?;
    let filename = match manifest_path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return Ok(UpdateOutcome::UnsupportedManifest),
    };
    let Some(manifest) = Manifest::from_manifest_filename(filename) else {
        return Ok(UpdateOutcome::UnsupportedManifest);
    };

    let manifest_dir = manifest_path.parent().unwrap_or(repo_root);

    for lockfile in manifest.lockfiles() {
        let Some(lockfile_path) = locate_lockfile(repo_root, manifest_dir, lockfile.filename)
        else {
            continue;
        };
        let per_package = match lockfile.update_kind {
            UpdateKind::PerPackage => match manifest_package_name(&manifest_path) {
                Some(name) => Some(name),
                None => return Ok(UpdateOutcome::UnsupportedManifest),
            },
            UpdateKind::Whole => None,
        };
        return Ok(run_update(
            repo_root,
            &lockfile_path,
            lockfile,
            per_package.as_deref(),
        ));
    }

    Ok(UpdateOutcome::NoLockfile)
}

/// What a lockfile says about the version of the package that owns it.
pub enum LockfileState {
    /// No lockfile alongside the manifest, or a manifest kind we do not model.
    None,
    /// A lockfile is there but does not record the owning package's own
    /// version, so there is nothing to compare. pnpm and poetry are like this.
    NoVersionRecorded {
        lockfile_rel: String,
    },
    Agrees {
        lockfile_rel: String,
    },
    Drifted {
        lockfile_rel: String,
        recorded: String,
    },
}

/// What the lockfile beside `manifest_rel` says about that package's version.
///
/// Read-only counterpart to [`update_for_manifest`]: it locates the lockfile
/// the same way but runs nothing, so it is safe on a machine without the
/// package manager installed and without registry access.
pub fn inspect_for_manifest(
    repo_root: &Path,
    manifest_rel: &str,
    manifest_version: &str,
) -> LockfileState {
    let Ok(manifest_path) = join_within_repo(repo_root, manifest_rel) else {
        return LockfileState::None;
    };
    let Some(filename) = manifest_path.file_name().and_then(|n| n.to_str()) else {
        return LockfileState::None;
    };
    let Some(manifest) = Manifest::from_manifest_filename(filename) else {
        return LockfileState::None;
    };
    let manifest_dir = manifest_path.parent().unwrap_or(repo_root);

    for lockfile in manifest.lockfiles() {
        let Some(lockfile_path) = locate_lockfile(repo_root, manifest_dir, lockfile.filename)
        else {
            continue;
        };
        let lockfile_rel = lockfile_path
            .strip_prefix(repo_root)
            .unwrap_or(&lockfile_path)
            .to_string_lossy()
            .replace('\\', "/");

        let Some(package) = manifest_package_name(&manifest_path) else {
            return LockfileState::NoVersionRecorded { lockfile_rel };
        };
        return match recorded_version(&lockfile_path, lockfile.filename, &package) {
            Some(recorded) if recorded == manifest_version => {
                LockfileState::Agrees { lockfile_rel }
            }
            Some(recorded) => LockfileState::Drifted {
                lockfile_rel,
                recorded,
            },
            None => LockfileState::NoVersionRecorded { lockfile_rel },
        };
    }

    LockfileState::None
}

/// The version a lockfile records for `package`, when the format records the
/// owning package at all. Only `Cargo.lock` does among the formats we handle:
/// pnpm, yarn and poetry lock dependencies without restating the version of
/// the package that owns them, so a drift of this shape cannot exist there.
fn recorded_version(lockfile_path: &Path, filename: &str, package: &str) -> Option<String> {
    if filename != "Cargo.lock" {
        return None;
    }
    let content = std::fs::read_to_string(lockfile_path).ok()?;
    let doc = content.parse::<toml_edit::DocumentMut>().ok()?;
    let packages = doc.get("package")?.as_array_of_tables()?;
    packages
        .iter()
        .find(|entry| entry.get("name").and_then(|n| n.as_str()) == Some(package))
        .and_then(|entry| entry.get("version"))
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
}

fn manifest_package_name(manifest_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(manifest_path).ok()?;
    let doc = content.parse::<toml_edit::DocumentMut>().ok()?;
    let name = doc.get("package")?.get("name")?.as_str()?;
    Some(name.to_string())
}

fn locate_lockfile(repo_root: &Path, manifest_dir: &Path, filename: &str) -> Option<PathBuf> {
    let mut dir = manifest_dir;
    loop {
        let candidate = dir.join(filename);
        if candidate.is_file() {
            return Some(candidate);
        }
        if dir == repo_root {
            return None;
        }
        dir = dir.parent()?;
    }
}

fn update_args(lockfile: &Lockfile, per_package: Option<&str>, offline: bool) -> Vec<String> {
    let mut args: Vec<String> = lockfile.base_args.iter().map(|a| a.to_string()).collect();
    if let Some(name) = per_package {
        args.push("-p".to_string());
        args.push(name.to_string());
        if offline {
            args.push("--offline".to_string());
        }
    }
    args
}

fn run_update(
    repo_root: &Path,
    lockfile_path: &Path,
    lockfile: &Lockfile,
    per_package: Option<&str>,
) -> UpdateOutcome {
    let lockfile_dir = lockfile_path.parent().unwrap_or(repo_root);

    let run = |offline: bool| {
        std::process::Command::new(lockfile.program)
            .current_dir(lockfile_dir)
            .args(update_args(lockfile, per_package, offline))
            .output()
    };

    let mut output = match run(per_package.is_some()) {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return UpdateOutcome::NotOnPath {
                program: lockfile.program.to_string(),
            };
        }
        Err(err) => {
            return UpdateOutcome::Failed {
                program: lockfile.program.to_string(),
                detail: err.to_string(),
            };
        }
    };

    if !output.status.success()
        && per_package.is_some()
        && let Ok(retry) = run(false)
    {
        output = retry;
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return UpdateOutcome::Failed {
            program: lockfile.program.to_string(),
            detail: first_meaningful_line(&stderr),
        };
    }

    let lockfile_rel = lockfile_path
        .strip_prefix(repo_root)
        .unwrap_or(lockfile_path)
        .to_string_lossy()
        .replace('\\', "/");

    UpdateOutcome::Updated { lockfile_rel }
}

fn first_meaningful_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_manifest_kind_from_filename() {
        assert_eq!(
            Manifest::from_manifest_filename("Cargo.toml"),
            Some(Manifest::CargoToml)
        );
        assert_eq!(
            Manifest::from_manifest_filename("package.json"),
            Some(Manifest::PackageJson)
        );
        assert_eq!(Manifest::from_manifest_filename("Chart.yaml"), None);
    }

    fn cargo() -> Lockfile {
        Manifest::CargoToml.lockfiles()[0]
    }

    #[test]
    fn the_retry_drops_offline_so_a_cold_registry_cache_can_be_filled() {
        let first = update_args(&cargo(), Some("ferrgames-discord"), true);
        assert_eq!(first, ["update", "-p", "ferrgames-discord", "--offline"]);

        let retry = update_args(&cargo(), Some("ferrgames-discord"), false);
        assert_eq!(
            retry,
            ["update", "-p", "ferrgames-discord"],
            "the release job checks out and installs the toolchain but never builds, \
             so the registry cache is empty and --offline cannot resolve dependencies"
        );
    }

    #[test]
    fn whole_lockfile_updates_never_get_offline_or_a_package() {
        let args = update_args(&cargo(), None, true);
        assert_eq!(args, ["update"]);
    }

    #[test]
    fn cargo_updates_the_crate_name_not_the_ferrflow_package_name() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let member = root.join("crates").join("api");
        std::fs::create_dir_all(member.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nresolver = \"3\"\nmembers = [\"crates/api\"]\n",
        )
        .unwrap();
        std::fs::write(
            member.join("Cargo.toml"),
            "[package]\nname = \"ferrgames-api\"\nversion = \"2.0.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(member.join("src").join("lib.rs"), "").unwrap();
        std::fs::write(
            root.join("Cargo.lock"),
            "version = 4\n\n[[package]]\nname = \"ferrgames-api\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();

        let outcome = update_for_manifest(root, "crates/api/Cargo.toml").unwrap();

        assert_eq!(
            outcome,
            UpdateOutcome::Updated {
                lockfile_rel: "Cargo.lock".to_string()
            },
            "the ferrflow package is named `api` but the crate is `ferrgames-api`; \
             passing the ferrflow name to `cargo update -p` fails and leaves the lock stale"
        );
        let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
        assert!(lock.contains("2.0.0"), "lockfile not bumped: {lock}");
    }

    #[test]
    fn unsupported_manifest_is_a_noop() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Chart.yaml"), "version: 1.0.0\n").unwrap();
        let outcome = update_for_manifest(dir.path(), "Chart.yaml").unwrap();
        assert_eq!(outcome, UpdateOutcome::UnsupportedManifest);
    }

    #[test]
    fn no_lockfile_when_sibling_absent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let outcome = update_for_manifest(dir.path(), "Cargo.toml").unwrap();
        assert_eq!(outcome, UpdateOutcome::NoLockfile);
    }

    #[test]
    fn locate_lockfile_walks_up_to_repo_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let member = root.join("crates").join("app");
        std::fs::create_dir_all(&member).unwrap();
        std::fs::write(root.join("Cargo.lock"), "# lock\n").unwrap();

        let found = locate_lockfile(root, &member, "Cargo.lock").unwrap();
        assert_eq!(found, root.join("Cargo.lock"));
    }

    #[test]
    fn locate_lockfile_does_not_escape_repo_root() {
        let outer = tempfile::tempdir().unwrap();
        std::fs::write(outer.path().join("Cargo.lock"), "# lock\n").unwrap();
        let repo_root = outer.path().join("repo");
        let member = repo_root.join("crates").join("app");
        std::fs::create_dir_all(&member).unwrap();

        assert!(locate_lockfile(&repo_root, &member, "Cargo.lock").is_none());
    }
}
