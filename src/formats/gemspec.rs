//! `*.gemspec` (Ruby) version handler.
//!
//! Matches assignments of the form `s.version = "x.y.z"` or
//! `spec.version = 'x.y.z'` inside a `Gem::Specification.new` block. The
//! receiver name varies by convention (`s`, `spec`, `gem`, …) so we accept
//! any identifier; the unique bit is the `.version =` suffix.
//!
//! A gemspec can also set the version from a constant
//! (`s.version = MyGem::VERSION`) loaded from `lib/my_gem/version.rb`. That
//! pattern isn't supported here — users in that setup should version the
//! `version.rb` file via [`super::txt::TxtVersionFile`] or a
//! regex-targeted file (follow-up work if the demand shows up).

use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct GemspecVersionFile;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    // `<ident>.version = "x.y.z"` — accepts single or double quotes, any
    // amount of whitespace around `=`.
    VERSION_RE.get_or_init(|| Regex::new(r#"(\.version\s*=\s*)(["'])([^"']+)(["'])"#).unwrap())
}

impl VersionFile for GemspecVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::GEMSPEC_READ)?;
        self.read_version_from_bytes(content.as_bytes(), &file_path.display().to_string())
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::GEMSPEC_READ)?;
        if !version_re().is_match(&content) {
            Err(anyhow::anyhow!(
                "No `.version = \"…\"` assignment found in {}",
                file_path.display()
            ))
            .error_code(error_code::GEMSPEC_VERSION_NOT_FOUND)?;
        }
        let new_content = version_re().replace(&content, |caps: &regex::Captures| {
            format!("{}{}{}{}", &caps[1], &caps[2], version, &caps[4])
        });
        std::fs::write(file_path, new_content.as_ref())
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::GEMSPEC_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::GEMSPEC_INVALID_UTF8)?;
        version_re()
            .captures(text)
            .map(|c| c[3].to_string())
            .ok_or_else(|| anyhow::anyhow!("No `.version = \"…\"` assignment found in {filename}"))
            .error_code(error_code::GEMSPEC_VERSION_NOT_FOUND)
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

    const FIXTURE: &str = r#"Gem::Specification.new do |s|
  s.name        = "my_gem"
  s.version     = "0.1.0"
  s.summary     = "A gem"
  s.authors     = ["Ada"]
  s.files       = Dir["lib/**/*.rb"]
end
"#;

    #[test]
    fn read_canonical_gemspec() {
        let f = write_temp(FIXTURE);
        assert_eq!(GemspecVersionFile.read_version(f.path()).unwrap(), "0.1.0");
    }

    #[test]
    fn write_preserves_double_quotes() {
        let f = write_temp(FIXTURE);
        GemspecVersionFile.write_version(f.path(), "0.2.0").unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("s.version     = \"0.2.0\""));
        // Sibling assignments untouched.
        assert!(out.contains("s.name        = \"my_gem\""));
    }

    #[test]
    fn write_preserves_single_quotes() {
        let f = write_temp("spec.version = '1.0.0'\n");
        GemspecVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("spec.version = '2.0.0'"));
    }

    #[test]
    fn read_arbitrary_receiver_name() {
        let f = write_temp("gem.version = \"1.2.3\"\n");
        assert_eq!(GemspecVersionFile.read_version(f.path()).unwrap(), "1.2.3");
    }

    #[test]
    fn read_no_version_fails() {
        let f = write_temp("Gem::Specification.new do |s|\n  s.name = \"x\"\nend\n");
        assert!(GemspecVersionFile.read_version(f.path()).is_err());
    }
}
