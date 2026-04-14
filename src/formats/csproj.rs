use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct CsprojVersionFile;

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

    const CSPROJ: &str = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Version>1.0.0</Version>
    <RootNamespace>MyApp</RootNamespace>
  </PropertyGroup>
</Project>"#;

    const CSPROJ_NO_VERSION: &str = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>
</Project>"#;

    const CSPROJ_MULTIPLE_PROPERTY_GROUPS: &str = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Version>2.5.0</Version>
  </PropertyGroup>
  <PropertyGroup Condition="'$(Configuration)'=='Release'">
    <Optimize>true</Optimize>
  </PropertyGroup>
</Project>"#;

    const CSPROJ_PACKAGE_VERSION: &str = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <Version>3.0.0</Version>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="Newtonsoft.Json" Version="13.0.1" />
  </ItemGroup>
</Project>"#;

    #[test]
    fn read_version() {
        let f = write_temp(CSPROJ);
        assert_eq!(CsprojVersionFile.read_version(f.path()).unwrap(), "1.0.0");
    }

    #[test]
    fn read_no_version_fails() {
        let f = write_temp(CSPROJ_NO_VERSION);
        assert!(CsprojVersionFile.read_version(f.path()).is_err());
    }

    #[test]
    fn write_version() {
        let f = write_temp(CSPROJ);
        CsprojVersionFile.write_version(f.path(), "2.0.0").unwrap();
        assert_eq!(CsprojVersionFile.read_version(f.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn write_preserves_other_content() {
        let f = write_temp(CSPROJ);
        CsprojVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("<TargetFramework>net8.0</TargetFramework>"));
        assert!(content.contains("<RootNamespace>MyApp</RootNamespace>"));
    }

    #[test]
    fn write_multiple_property_groups() {
        let f = write_temp(CSPROJ_MULTIPLE_PROPERTY_GROUPS);
        CsprojVersionFile.write_version(f.path(), "3.0.0").unwrap();
        assert_eq!(CsprojVersionFile.read_version(f.path()).unwrap(), "3.0.0");
    }

    #[test]
    fn does_not_touch_package_reference_version() {
        let f = write_temp(CSPROJ_PACKAGE_VERSION);
        CsprojVersionFile.write_version(f.path(), "4.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("<Version>4.0.0</Version>"));
        assert!(content.contains("Version=\"13.0.1\""));
    }

    #[test]
    fn write_no_version_fails() {
        let f = write_temp(CSPROJ_NO_VERSION);
        assert!(CsprojVersionFile.write_version(f.path(), "1.0.0").is_err());
    }
}

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    VERSION_RE.get_or_init(|| Regex::new(r"<Version>([^<]+)</Version>").unwrap())
}

impl VersionFile for CsprojVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::CSPROJ_READ)?;

        version_re()
            .captures(&content)
            .map(|c| c[1].trim().to_string())
            .ok_or_else(|| anyhow::anyhow!("No <Version> tag found in {}", file_path.display()))
            .error_code(error_code::CSPROJ_VERSION_NOT_FOUND)
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::CSPROJ_READ)?;

        let mut count = 0;
        let new_content = version_re().replace(&content, |_: &regex::Captures| {
            count += 1;
            format!("<Version>{version}</Version>")
        });

        if count == 0 {
            Err(anyhow::anyhow!(
                "No <Version> tag found to update in {}",
                file_path.display()
            ))
            .error_code(error_code::CSPROJ_VERSION_NOT_FOUND)?;
        }

        std::fs::write(file_path, new_content.as_ref())
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::CSPROJ_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::CSPROJ_INVALID_UTF8)?;
        version_re()
            .captures(text)
            .map(|c| c[1].trim().to_string())
            .ok_or_else(|| anyhow::anyhow!("No <Version> tag found in {filename}"))
            .error_code(error_code::CSPROJ_VERSION_NOT_FOUND)
    }
}
