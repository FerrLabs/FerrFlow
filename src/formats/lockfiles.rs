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

pub fn update_for_manifest(
    repo_root: &Path,
    manifest_rel: &str,
    package_name: &str,
) -> anyhow::Result<UpdateOutcome> {
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
        return Ok(run_update(
            repo_root,
            &lockfile_path,
            lockfile,
            package_name,
        ));
    }

    Ok(UpdateOutcome::NoLockfile)
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

fn run_update(
    repo_root: &Path,
    lockfile_path: &Path,
    lockfile: &Lockfile,
    package_name: &str,
) -> UpdateOutcome {
    let lockfile_dir = lockfile_path.parent().unwrap_or(repo_root);

    let mut cmd = std::process::Command::new(lockfile.program);
    cmd.current_dir(lockfile_dir);
    cmd.args(lockfile.base_args);
    if let UpdateKind::PerPackage = lockfile.update_kind {
        cmd.arg("-p").arg(package_name).arg("--offline");
    }

    let output = match cmd.output() {
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

    #[test]
    fn unsupported_manifest_is_a_noop() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Chart.yaml"), "version: 1.0.0\n").unwrap();
        let outcome = update_for_manifest(dir.path(), "Chart.yaml", "app").unwrap();
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
        let outcome = update_for_manifest(dir.path(), "Cargo.toml", "app").unwrap();
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
