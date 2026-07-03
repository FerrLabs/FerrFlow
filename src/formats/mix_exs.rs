use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct MixExsVersionFile;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    VERSION_RE.get_or_init(|| Regex::new(r#"(?m)(version:\s*)(["'])([^"']+)(["'])"#).unwrap())
}

impl VersionFile for MixExsVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::MIX_EXS_READ)?;
        self.read_version_from_bytes(content.as_bytes(), &file_path.display().to_string())
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        super::splice::write_via_splice(self, file_path, version, None)
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::MIX_EXS_INVALID_UTF8)?;
        version_re()
            .captures(text)
            .map(|c| c[3].to_string())
            .ok_or_else(|| anyhow::anyhow!("No `version: \"…\"` literal found in {filename}"))
            .error_code(error_code::MIX_EXS_VERSION_NOT_FOUND)
    }
}

impl super::splice::FormatPreservingEditor for MixExsVersionFile {
    fn locate_version(
        &self,
        content: &str,
        _selector: Option<&str>,
    ) -> Result<std::ops::Range<usize>> {
        version_re()
            .captures(content)
            .map(|c| c.get(3).unwrap().range())
            .ok_or_else(|| anyhow::anyhow!("No `version: \"…\"` literal found"))
            .error_code(error_code::MIX_EXS_VERSION_NOT_FOUND)
    }

    fn read_error(&self) -> crate::error_code::ErrorCode {
        error_code::MIX_EXS_READ
    }

    fn write_error(&self) -> crate::error_code::ErrorCode {
        error_code::MIX_EXS_WRITE
    }
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

    const FIXTURE: &str = r#"defmodule MyApp.MixProject do
  use Mix.Project

  def project do
    [
      app: :my_app,
      version: "0.1.0",
      elixir: "~> 1.15",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  defp deps do
    [{:phoenix, "~> 1.7"}]
  end
end
"#;

    #[test]
    fn read_canonical_project() {
        let f = write_temp(FIXTURE);
        assert_eq!(MixExsVersionFile.read_version(f.path()).unwrap(), "0.1.0");
    }

    #[test]
    fn write_canonical_project() {
        let f = write_temp(FIXTURE);
        MixExsVersionFile.write_version(f.path(), "0.2.0").unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("version: \"0.2.0\""));
        assert!(out.contains("app: :my_app"));
        assert!(out.contains("deps: deps()"));
    }

    #[test]
    fn write_only_replaces_first_match() {
        let src = "version: \"1.0.0\"\n# later:\nconfig: [version: \"2.0.0\"]\n";
        let f = write_temp(src);
        MixExsVersionFile.write_version(f.path(), "1.1.0").unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("version: \"1.1.0\""));
        assert!(out.contains("version: \"2.0.0\""));
    }

    #[test]
    fn read_single_quoted() {
        let f = write_temp("version: '3.4.5'\n");
        assert_eq!(MixExsVersionFile.read_version(f.path()).unwrap(), "3.4.5");
    }

    #[test]
    fn read_no_version_fails() {
        let f = write_temp("defmodule Foo do\nend\n");
        assert!(MixExsVersionFile.read_version(f.path()).is_err());
    }

    #[test]
    fn write_no_version_fails() {
        let f = write_temp("defmodule Foo do\nend\n");
        assert!(MixExsVersionFile.write_version(f.path(), "1.0.0").is_err());
    }
}
