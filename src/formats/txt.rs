use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::ops::Range;
use std::path::Path;

pub struct TxtVersionFile;

fn select_span(text: &str, selector: &str, origin: &str) -> Result<Range<usize>> {
    let re = compile_selector(selector)?;
    let cap = re
        .captures(text)
        .ok_or_else(|| anyhow::anyhow!("selector {selector:?} did not match anything in {origin}"))
        .error_code(error_code::TXT_VERSION_NOT_FOUND)?;
    let m = cap
        .get(1)
        .ok_or_else(|| {
            anyhow::anyhow!("selector {selector:?} matched but capture group 1 did not participate")
        })
        .error_code(error_code::TXT_VERSION_NOT_FOUND)?;

    let raw = m.as_str();
    let leading = raw.len() - raw.trim_start().len();
    let trailing = raw.len() - raw.trim_end().len();
    if leading + trailing >= raw.len() {
        Err(anyhow::anyhow!(
            "selector {selector:?} captured only whitespace in {origin}"
        ))
        .error_code(error_code::TXT_VERSION_NOT_FOUND)?;
    }

    Ok(m.start() + leading..m.end() - trailing)
}

fn select_version(text: &str, selector: &str, origin: &str) -> Result<String> {
    Ok(text[select_span(text, selector, origin)?].to_string())
}

fn compile_selector(selector: &str) -> Result<Regex> {
    let re = Regex::new(selector)
        .with_context(|| format!("invalid regex selector: {selector:?}"))
        .error_code(error_code::TXT_VERSION_NOT_FOUND)?;
    if re.captures_len() != 2 {
        Err(anyhow::anyhow!(
            "regex selector must contain exactly one capture group: {selector:?}"
        ))
        .error_code(error_code::TXT_VERSION_NOT_FOUND)?;
    }
    Ok(re)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::VersionFile;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn read_version_from_txt() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "1.2.3").unwrap();
        let v = TxtVersionFile.read_version(f.path()).unwrap();
        assert_eq!(v, "1.2.3");
    }

    #[test]
    fn read_version_trims_whitespace() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "  0.4.1\n\n").unwrap();
        let v = TxtVersionFile.read_version(f.path()).unwrap();
        assert_eq!(v, "0.4.1");
    }

    #[test]
    fn read_empty_file_fails() {
        let f = NamedTempFile::new().unwrap();
        let result = TxtVersionFile.read_version(f.path());
        assert!(result.is_err());
    }

    #[test]
    fn write_version_to_txt() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "1.0.0").unwrap();
        TxtVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(content, "2.0.0\n");
    }

    #[test]
    fn read_with_regex_selector_picks_capture_group() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "name=foo\nVERSION=4.5.6\nother=ignored\n").unwrap();
        let v = TxtVersionFile
            .read_version_with_selector(f.path(), Some(r"(?m)^VERSION=(.+)$"))
            .unwrap();
        assert_eq!(v, "4.5.6");
    }

    #[test]
    fn write_with_regex_selector_replaces_capture_only() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "name=foo\nVERSION=1.0.0\nother=ignored\n").unwrap();
        TxtVersionFile
            .write_version_with_selector(f.path(), "2.0.0", Some(r"(?m)^VERSION=(.+)$"))
            .unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(content, "name=foo\nVERSION=2.0.0\nother=ignored\n");
    }

    #[test]
    fn selector_with_no_match_errors() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "no version here").unwrap();
        let result = TxtVersionFile.read_version_with_selector(f.path(), Some(r"^VERSION=(.+)$"));
        assert!(result.is_err());
    }

    #[test]
    fn selector_with_wrong_capture_count_errors() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "VERSION=1.0.0").unwrap();
        let result = TxtVersionFile.read_version_with_selector(f.path(), Some(r"VERSION=.+"));
        assert!(result.is_err());
        let result = TxtVersionFile.read_version_with_selector(f.path(), Some(r"(VERSION)=(.+)"));
        assert!(result.is_err());
    }

    #[test]
    fn read_bytes_uses_the_selector() {
        let content = b"a: 1\nbundleVersion: 1.2.3\nb: 2\n";
        let v = TxtVersionFile
            .read_version_from_bytes_with_selector(
                content,
                "Settings.asset",
                Some("(?m)^bundleVersion: (.+)$"),
            )
            .unwrap();
        assert_eq!(v, "1.2.3");
    }

    #[test]
    fn read_bytes_reports_a_selector_that_matches_nothing() {
        let err = TxtVersionFile
            .read_version_from_bytes_with_selector(
                b"a: 1\n",
                "Settings.asset",
                Some("(?m)^bundleVersion: (.+)$"),
            )
            .unwrap_err();
        assert!(
            format!("{err:#}").contains("did not match anything"),
            "{err:#}"
        );
    }

    #[test]
    fn read_bytes_without_a_selector_still_reads_the_whole_file() {
        let v = TxtVersionFile
            .read_version_from_bytes_with_selector(b"  1.2.3\n", "VERSION", None)
            .unwrap();
        assert_eq!(v, "1.2.3");
    }

    #[test]
    fn a_selector_capture_is_trimmed_on_both_sides() {
        const PADDING: &str = "   ";
        let content = format!(
            "version = 1.2.3{PADDING}
"
        );
        let v = TxtVersionFile
            .read_version_from_bytes_with_selector(
                content.as_bytes(),
                "VERSION",
                Some("(?m)^version =(.+)$"),
            )
            .unwrap();
        assert_eq!(
            v, "1.2.3",
            "a selector that captures the padding must not ship it"
        );
    }

    #[test]
    fn a_selector_that_captures_only_whitespace_is_an_error() {
        const BLANK: &str = " ";
        let content = format!(
            "version ={BLANK}
other = 1
"
        );
        let err = TxtVersionFile
            .read_version_from_bytes_with_selector(
                content.as_bytes(),
                "VERSION",
                Some("(?m)^version =(.*)$"),
            )
            .unwrap_err();
        let rendered = format!("{err:#}");
        assert!(rendered.contains("only whitespace"), "{rendered}");
    }

    #[test]
    fn a_selector_whose_group_never_participates_carries_a_code() {
        let err = TxtVersionFile
            .read_version_from_bytes_with_selector(
                b"version
",
                "VERSION",
                Some("(?m)^version(?: =(.*))?$"),
            )
            .unwrap_err();
        let rendered = format!("{err:#}");
        assert!(rendered.contains("did not participate"), "{rendered}");
        assert!(
            rendered.contains(&error_code::TXT_VERSION_NOT_FOUND.to_string()),
            "every other error on this path carries its code: {rendered}"
        );
    }

    #[test]
    fn a_write_keeps_the_padding_the_read_ignored() {
        const PADDING: &str = "   ";
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "version = 1.2.3{PADDING}").unwrap();
        writeln!(f, "other = 1").unwrap();

        let selector = Some("(?m)^version =(.+)$");
        let read = TxtVersionFile
            .read_version_with_selector(f.path(), selector)
            .unwrap();
        assert_eq!(read, "1.2.3");

        TxtVersionFile
            .write_version_with_selector(f.path(), "2.0.0", selector)
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(f.path()).unwrap(),
            format!("version = 2.0.0{PADDING}\nother = 1\n"),
            "the write may only replace what the read returned"
        );
    }

    #[test]
    fn a_release_through_a_padded_selector_is_idempotent_on_the_padding() {
        const PADDING: &str = "  ";
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "v ={PADDING}1.0.0{PADDING}").unwrap();

        let selector = Some("(?m)^v =(.+)$");
        for version in ["1.1.0", "1.2.0", "1.3.0"] {
            TxtVersionFile
                .write_version_with_selector(f.path(), version, selector)
                .unwrap();
        }

        assert_eq!(
            std::fs::read_to_string(f.path()).unwrap(),
            format!("v ={PADDING}1.3.0{PADDING}\n"),
            "three releases must not erode the padding one character at a time"
        );
    }

    #[test]
    fn the_span_is_cut_on_character_boundaries_not_bytes() {
        const WIDE: &str = "\u{3000}\u{3000}";
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "v ={WIDE}1.0.0{WIDE}").unwrap();

        let selector = Some("(?m)^v =(.+)$");
        assert_eq!(
            TxtVersionFile
                .read_version_with_selector(f.path(), selector)
                .unwrap(),
            "1.0.0"
        );

        TxtVersionFile
            .write_version_with_selector(f.path(), "2.0.0", selector)
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(f.path()).unwrap(),
            format!("v ={WIDE}2.0.0{WIDE}\n"),
            "three-byte whitespace must not be counted as one"
        );
    }

    #[test]
    fn a_write_whose_group_never_participates_carries_a_code() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "version").unwrap();
        let err = TxtVersionFile
            .write_version_with_selector(f.path(), "2.0.0", Some("(?m)^version(?: =(.*))?$"))
            .unwrap_err();
        let rendered = format!("{err:#}");
        assert!(rendered.contains("did not participate"), "{rendered}");
        assert!(
            rendered.contains(&error_code::TXT_VERSION_NOT_FOUND.to_string()),
            "the write path must fail like the read path: {rendered}"
        );
    }

    #[test]
    fn a_write_onto_a_whitespace_only_capture_is_an_error() {
        const BLANK: &str = " ";
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "version ={BLANK}").unwrap();
        writeln!(f, "other = 1").unwrap();
        let before = std::fs::read_to_string(f.path()).unwrap();

        let err = TxtVersionFile
            .write_version_with_selector(f.path(), "2.0.0", Some("(?m)^version =(.*)$"))
            .unwrap_err();

        assert!(format!("{err:#}").contains("only whitespace"), "{err:#}");
        assert_eq!(
            std::fs::read_to_string(f.path()).unwrap(),
            before,
            "a refused write leaves the file alone"
        );
    }
}

impl super::VersionFile for TxtVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("failed to read {}", file_path.display()))
            .error_code(error_code::TXT_READ)?;
        let version = content.trim();
        if version.is_empty() {
            Err(anyhow::anyhow!(
                "no version found in {}",
                file_path.display()
            ))
            .error_code(error_code::TXT_VERSION_NOT_FOUND)?;
        }
        Ok(version.to_string())
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        std::fs::write(file_path, format!("{version}\n"))
            .with_context(|| format!("failed to write {}", file_path.display()))
            .error_code(error_code::TXT_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::TXT_INVALID_UTF8)?;
        let version = text.trim();
        if version.is_empty() {
            Err(anyhow::anyhow!("no version found in {filename}"))
                .error_code(error_code::TXT_VERSION_NOT_FOUND)?;
        }
        Ok(version.to_string())
    }

    fn read_version_from_bytes_with_selector(
        &self,
        content: &[u8],
        filename: &str,
        selector: Option<&str>,
    ) -> Result<String> {
        let Some(sel) = selector else {
            return self.read_version_from_bytes(content, filename);
        };
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::TXT_INVALID_UTF8)?;
        select_version(text, sel, filename)
    }

    fn read_version_with_selector(
        &self,
        file_path: &Path,
        selector: Option<&str>,
    ) -> Result<String> {
        let Some(sel) = selector else {
            return self.read_version(file_path);
        };
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("failed to read {}", file_path.display()))
            .error_code(error_code::TXT_READ)?;
        select_version(&content, sel, &file_path.display().to_string())
    }

    fn write_version_with_selector(
        &self,
        file_path: &Path,
        version: &str,
        selector: Option<&str>,
    ) -> Result<()> {
        let Some(sel) = selector else {
            return self.write_version(file_path, version);
        };
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("failed to read {}", file_path.display()))
            .error_code(error_code::TXT_READ)?;
        let span = select_span(&content, sel, &file_path.display().to_string())?;
        let mut new_content = String::with_capacity(content.len() + version.len());
        new_content.push_str(&content[..span.start]);
        new_content.push_str(version);
        new_content.push_str(&content[span.end..]);
        std::fs::write(file_path, new_content)
            .with_context(|| format!("failed to write {}", file_path.display()))
            .error_code(error_code::TXT_WRITE)?;
        Ok(())
    }
}
