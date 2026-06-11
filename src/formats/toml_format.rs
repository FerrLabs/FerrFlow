use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use std::path::Path;
use toml_edit::{DocumentMut, Item};

pub struct TomlVersionFile;

/// Read the version string from a Cargo / pyproject / Poetry TOML
/// document, in the order that Rust monorepos / Python projects expect:
///
/// 1. `[package].version = "x"` — single-crate Cargo or pyproject (PEP 621).
/// 2. `[package].version = { workspace = true }` — Cargo workspace
///    inheritance: in this case we resolve from `[workspace.package].version`
///    inside the same file (only the workspace root Cargo.toml carries
///    that table). A member-only manifest with `version.workspace = true`
///    and no `[workspace.package]` is unreachable from FerrFlow's per-file
///    model and returns a clear, actionable error pointing at the
///    workspace root. See #523.
/// 3. `[workspace.package].version` directly — virtual workspaces (no
///    `[package]` at root) and the workspace root itself.
/// 4. `[project].version` — pyproject (PEP 621).
/// 5. `[tool.poetry].version` — pyproject (Poetry).
fn read_toml_version(doc: &DocumentMut, location: &str) -> Result<String> {
    if let Some(pkg) = doc.get("package")
        && let Some(version) = pkg.get("version")
    {
        if let Some(s) = version.as_str() {
            return Ok(s.to_string());
        }
        if is_workspace_inherit(version) {
            if let Some(s) = doc
                .get("workspace")
                .and_then(|w| w.get("package"))
                .and_then(|p| p.get("version"))
                .and_then(Item::as_str)
            {
                return Ok(s.to_string());
            }
            return Err(anyhow::anyhow!(
                "{location} declares `version.workspace = true` but no `[workspace.package].version` was found in the same file. \
                 Point FerrFlow at the workspace root Cargo.toml instead."
            ))
            .error_code(error_code::TOML_VERSION_NOT_FOUND);
        }
    }

    if let Some(s) = doc
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(Item::as_str)
    {
        return Ok(s.to_string());
    }

    if let Some(s) = doc
        .get("project")
        .and_then(|p| p.get("version"))
        .and_then(Item::as_str)
    {
        return Ok(s.to_string());
    }

    if let Some(s) = doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("version"))
        .and_then(Item::as_str)
    {
        return Ok(s.to_string());
    }

    Err(anyhow::anyhow!("No version found in {location}"))
        .error_code(error_code::TOML_VERSION_NOT_FOUND)?
}

/// Returns true when an `Item` is the inline table `{ workspace = true }`.
/// Cargo's spec accepts both dotted (`version.workspace = true`) and
/// inline (`version = { workspace = true }`) forms, which toml_edit
/// normalizes to the same `InlineTable` shape.
fn is_workspace_inherit(item: &Item) -> bool {
    if let Some(t) = item.as_inline_table() {
        return t.get("workspace").and_then(|v| v.as_bool()) == Some(true);
    }
    if let Some(t) = item.as_table()
        && let Some(v) = t.get("workspace")
    {
        return v.as_bool() == Some(true);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn read_cargo_toml() {
        let f = write_temp("[package]\nname = \"foo\"\nversion = \"1.2.3\"\n");
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "1.2.3");
    }

    #[test]
    fn read_pyproject_toml() {
        let f = write_temp("[project]\nname = \"foo\"\nversion = \"0.5.0\"\n");
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "0.5.0");
    }

    #[test]
    fn read_poetry_toml() {
        let f = write_temp("[tool.poetry]\nname = \"foo\"\nversion = \"3.1.0\"\n");
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "3.1.0");
    }

    #[test]
    fn read_no_version_fails() {
        let f = write_temp("[package]\nname = \"foo\"\n");
        assert!(TomlVersionFile.read_version(f.path()).is_err());
    }

    #[test]
    fn write_cargo_toml() {
        let f = write_temp("[package]\nname = \"foo\"\nversion = \"1.0.0\"\n");
        TomlVersionFile.write_version(f.path(), "2.0.0").unwrap();
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn write_pyproject_toml() {
        let f = write_temp("[project]\nname = \"foo\"\nversion = \"1.0.0\"\n");
        TomlVersionFile.write_version(f.path(), "2.0.0").unwrap();
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn write_preserves_formatting() {
        let input = "[package]\nname = \"foo\"\nversion = \"1.0.0\"\nedition = \"2021\"\n";
        let f = write_temp(input);
        TomlVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("name = \"foo\""));
        assert!(content.contains("edition = \"2021\""));
    }

    // ---------- #523: Cargo workspace.version inheritance ----------

    #[test]
    fn read_workspace_root_with_workspace_package_version() {
        // Pure virtual workspace — no [package] section, just
        // [workspace.package].
        let f = write_temp(
            "[workspace]\nmembers = [\"crates/*\"]\n\n[workspace.package]\nversion = \"2.5.0\"\n",
        );
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "2.5.0");
    }

    #[test]
    fn read_workspace_root_with_package_inheriting_dotted() {
        // Root Cargo.toml that also acts as a member, with the dotted
        // form `version.workspace = true`.
        let f = write_temp(
            "[workspace]\nmembers = [\"members/*\"]\n\n\
             [workspace.package]\nversion = \"3.1.4\"\n\n\
             [package]\nname = \"root\"\nversion.workspace = true\n",
        );
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "3.1.4");
    }

    #[test]
    fn read_workspace_root_with_package_inheriting_inline() {
        // Same scenario, inline-table form `version = { workspace = true }`.
        let f = write_temp(
            "[workspace]\nmembers = [\"members/*\"]\n\n\
             [workspace.package]\nversion = \"3.1.4\"\n\n\
             [package]\nname = \"root\"\nversion = { workspace = true }\n",
        );
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "3.1.4");
    }

    #[test]
    fn read_member_without_workspace_table_errors_actionably() {
        // Member Cargo.toml: declares inheritance but has no
        // [workspace.package] to resolve from. Must produce a clear
        // error pointing at the workspace root, not a generic
        // "no version found".
        let f = write_temp(
            "[package]\nname = \"member\"\nversion.workspace = true\nedition = \"2021\"\n",
        );
        let err = TomlVersionFile
            .read_version(f.path())
            .expect_err("must error");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("workspace root Cargo.toml"),
            "error should point at the workspace root; got: {msg}"
        );
    }

    #[test]
    fn write_workspace_root_bumps_workspace_package_version() {
        let input = "[workspace]\nmembers = [\"crates/*\"]\n\n\
                     [workspace.package]\nversion = \"1.0.0\"\nedition = \"2021\"\n\n\
                     [package]\nname = \"root\"\nversion.workspace = true\n";
        let f = write_temp(input);
        TomlVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        // The workspace version got bumped...
        assert!(content.contains("[workspace.package]"));
        let workspace_section_start = content.find("[workspace.package]").unwrap();
        let after_workspace_section = &content[workspace_section_start..];
        assert!(
            after_workspace_section.contains("version = \"2.0.0\""),
            "workspace.package.version should be bumped"
        );
        // ...and the `[package]` inheritance marker is left untouched.
        assert!(content.contains("version.workspace = true"));
        // Read-back resolves to the new version.
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn write_virtual_workspace_bumps_workspace_package_version() {
        let input = "[workspace]\nmembers = [\"crates/*\"]\n\n\
                     [workspace.package]\nversion = \"1.0.0\"\n";
        let f = write_temp(input);
        TomlVersionFile.write_version(f.path(), "1.1.0").unwrap();
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "1.1.0");
    }

    #[test]
    fn write_member_without_workspace_table_errors_actionably() {
        let f = write_temp(
            "[package]\nname = \"member\"\nversion.workspace = true\nedition = \"2021\"\n",
        );
        let err = TomlVersionFile
            .write_version(f.path(), "2.0.0")
            .expect_err("must error");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("workspace root Cargo.toml"),
            "error should point at the workspace root; got: {msg}"
        );
    }

    #[test]
    fn read_from_bytes_workspace_root() {
        let content = "[workspace.package]\nversion = \"4.2.0\"\n";
        let v = TomlVersionFile
            .read_version_from_bytes(content.as_bytes(), "Cargo.toml")
            .unwrap();
        assert_eq!(v, "4.2.0");
    }
}

impl VersionFile for TomlVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::TOML_READ)?;
        let doc: DocumentMut = content
            .parse()
            .with_context(|| format!("Invalid TOML in {}", file_path.display()))
            .error_code(error_code::TOML_PARSE)?;
        read_toml_version(&doc, &file_path.display().to_string())
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::TOML_READ)?;
        let mut doc: DocumentMut = content
            .parse()
            .with_context(|| format!("Invalid TOML in {}", file_path.display()))
            .error_code(error_code::TOML_PARSE)?;

        // Decision: when `[package].version` is the workspace inheritance
        // marker we bump `[workspace.package].version` instead (if present
        // in the same file). This is what a workspace-root Cargo.toml that
        // both defines `[workspace.package]` AND a `[package]` with
        // `version.workspace = true` looks like, and bumping the workspace
        // value cascades through every member without us touching them.
        let mut written = false;

        if let Some(pkg_version) = doc.get("package").and_then(|p| p.get("version"))
            && pkg_version.as_str().is_none()
            && is_workspace_inherit(pkg_version)
            && let Some(workspace_version) = doc
                .get_mut("workspace")
                .and_then(|w| w.get_mut("package"))
                .and_then(|p| p.get_mut("version"))
            && workspace_version.is_str()
        {
            *workspace_version = toml_edit::value(version);
            written = true;
        }

        if !written
            && let Some(pkg) = doc.get_mut("package")
            && let Some(v) = pkg.get_mut("version")
            && v.is_str()
        {
            *v = toml_edit::value(version);
            written = true;
        }

        if !written
            && let Some(workspace_version) = doc
                .get_mut("workspace")
                .and_then(|w| w.get_mut("package"))
                .and_then(|p| p.get_mut("version"))
            && workspace_version.is_str()
        {
            *workspace_version = toml_edit::value(version);
            written = true;
        }

        if !written
            && let Some(proj) = doc.get_mut("project")
            && let Some(v) = proj.get_mut("version")
        {
            *v = toml_edit::value(version);
            written = true;
        }

        if !written
            && let Some(tool) = doc.get_mut("tool")
            && let Some(poetry) = tool.get_mut("poetry")
            && let Some(v) = poetry.get_mut("version")
        {
            *v = toml_edit::value(version);
            written = true;
        }

        if !written {
            // Surface the inheritance case specifically — the generic
            // "could not find" message points users in the wrong
            // direction when they actually have a workspace member.
            if let Some(version_item) = doc.get("package").and_then(|p| p.get("version"))
                && is_workspace_inherit(version_item)
            {
                Err(anyhow::anyhow!(
                    "Cannot bump {}: this Cargo.toml inherits its version from the workspace \
                     (`version.workspace = true`) but no `[workspace.package].version` is \
                     defined in the same file. Point FerrFlow at the workspace root Cargo.toml \
                     instead — bumping it will cascade to every member.",
                    file_path.display()
                ))
                .error_code(error_code::TOML_VERSION_NOT_FOUND)?;
            }
            Err(anyhow::anyhow!(
                "Could not find version field to update in {}",
                file_path.display()
            ))
            .error_code(error_code::TOML_VERSION_NOT_FOUND)?;
        }

        std::fs::write(file_path, doc.to_string())
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::TOML_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::TOML_INVALID_UTF8)?;
        let doc: DocumentMut = text
            .parse()
            .with_context(|| format!("Invalid TOML in {filename}"))
            .error_code(error_code::TOML_PARSE)?;
        read_toml_version(&doc, filename)
    }
}
