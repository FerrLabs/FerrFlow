//! `mix.exs` (Elixir) version handler.
//!
//! Targets the `version: "x.y.z"` literal inside a Mix project definition
//! (typically `def project do [ ..., version: "x.y.z", ... ] end`). We do a
//! regex replace rather than parsing Elixir — the idiomatic placement is a
//! single occurrence in the project keyword list, and a structural parse
//! would require a full Elixir lexer. Edge cases (version inside a
//! comment, version inside a dependency tuple) are accepted losses here:
//! they're extremely rare in the wild, and a misplaced match is a build
//! failure rather than a silent corruption.

use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct MixExsVersionFile;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    // Captures `version: "..."` — tolerant of whitespace and either quote
    // style. We deliberately do **not** match `:version` (which would be an
    // atom reference inside deps tuples) — the `(?!:)` isn't needed because
    // Elixir keyword keys end with `:` on the right, not `:` on the left.
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
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::MIX_EXS_READ)?;
        if !version_re().is_match(&content) {
            Err(anyhow::anyhow!(
                "No `version: \"…\"` literal found in {}",
                file_path.display()
            ))
            .error_code(error_code::MIX_EXS_VERSION_NOT_FOUND)?;
        }
        // Replace only the first match — a well-formed mix.exs has exactly
        // one version in the project definition. Matches further down (e.g.
        // accidental dep tuples) are intentionally left alone.
        let mut replaced = false;
        let new_content = version_re().replace_all(&content, |caps: &regex::Captures| {
            if replaced {
                caps.get(0).unwrap().as_str().to_string()
            } else {
                replaced = true;
                format!("{}{}{}{}", &caps[1], &caps[2], version, &caps[4])
            }
        });
        std::fs::write(file_path, new_content.as_ref())
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::MIX_EXS_WRITE)?;
        Ok(())
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
        // Sibling keyword entries stay intact.
        assert!(out.contains("app: :my_app"));
        assert!(out.contains("deps: deps()"));
    }

    #[test]
    fn write_only_replaces_first_match() {
        // Contrived: second `version:` lives in a nested config that happens
        // to share the keyword name. We only touch the first one.
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
