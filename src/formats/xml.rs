//! XML version-file handler.
//!
//! The original implementation used a single `<version>([^<]+)</version>` regex
//! that matched the **first** occurrence in the file. That broke on every
//! Maven `pom.xml` whose `<parent>` block precedes the project's own
//! `<groupId>`/`<artifactId>`/`<version>` — Spring Boot apps and any
//! multi-module Maven project. FerrFlow would read the parent dependency's
//! version (e.g. `spring-boot-starter-parent` 3.5.14) instead of the
//! project version, then rewrite the wrong tag on bump.
//!
//! The handler now walks the document with a tiny state machine that
//! tracks element depth, so it can target a `<version>` that is a direct
//! child of the document root (`<project>` for Maven). For weirder layouts
//! (profile blocks, Maven BOM imports, `.csproj` flavours, …) the user can
//! point at any tag via a slash-delimited selector like `/project/version`.
//!
//! No XML parser pulled in — the format is forgiving enough that a small
//! tokeniser handles everything FerrFlow needs (open / close / self-close
//! tags, comments, CDATA, processing instructions, `xmlns`/attributes).

use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use std::path::Path;

pub struct XmlVersionFile;

/// One match inside the document — byte offsets covering the inner text
/// between `<tag>` and `</tag>`.
#[derive(Debug, Clone, Copy)]
struct InnerRange {
    start: usize,
    end: usize,
}

/// Tokeniser state.
struct Scanner<'a> {
    src: &'a [u8],
    i: usize,
}

impl<'a> Scanner<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            i: 0,
        }
    }

    fn done(&self) -> bool {
        self.i >= self.src.len()
    }

    /// Skip a `<!-- ... -->` comment, `<![CDATA[...]]>` block, or
    /// `<? ... ?>` processing instruction starting at `self.i` (which
    /// points at the leading `<`). Returns true if one was consumed.
    fn skip_special(&mut self) -> bool {
        let rest = &self.src[self.i..];
        if rest.starts_with(b"<!--") {
            self.i += 4;
            if let Some(end) = self.find_at(b"-->") {
                self.i = end + 3;
            } else {
                self.i = self.src.len();
            }
            return true;
        }
        if rest.starts_with(b"<![CDATA[") {
            self.i += 9;
            if let Some(end) = self.find_at(b"]]>") {
                self.i = end + 3;
            } else {
                self.i = self.src.len();
            }
            return true;
        }
        if rest.starts_with(b"<?") {
            self.i += 2;
            if let Some(end) = self.find_at(b"?>") {
                self.i = end + 2;
            } else {
                self.i = self.src.len();
            }
            return true;
        }
        // <!DOCTYPE ...> — naive: eat until the matching '>'
        if rest.starts_with(b"<!") {
            self.i += 2;
            while self.i < self.src.len() && self.src[self.i] != b'>' {
                self.i += 1;
            }
            if self.i < self.src.len() {
                self.i += 1;
            }
            return true;
        }
        false
    }

    fn find_at(&self, needle: &[u8]) -> Option<usize> {
        find_subslice(&self.src[self.i..], needle).map(|p| self.i + p)
    }

    /// Parse a tag starting at `self.i` (which must point at `<`). Returns
    /// `(name, kind, end_index_after_close_bracket)`.
    fn read_tag(&mut self) -> Option<(String, TagKind, usize)> {
        debug_assert_eq!(self.src.get(self.i).copied(), Some(b'<'));
        let mut p = self.i + 1;
        let kind_close = if self.src.get(p).copied() == Some(b'/') {
            p += 1;
            true
        } else {
            false
        };

        // Read the name.
        let name_start = p;
        while p < self.src.len() {
            let c = self.src[p];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b'/' || c == b'>' {
                break;
            }
            p += 1;
        }
        if p == name_start {
            return None;
        }
        let name = std::str::from_utf8(&self.src[name_start..p])
            .ok()?
            .to_string();

        // Skip past attributes to the closing '>'.
        let mut self_close = false;
        let mut in_quote: Option<u8> = None;
        while p < self.src.len() {
            let c = self.src[p];
            match in_quote {
                Some(q) if c == q => in_quote = None,
                Some(_) => {}
                None => match c {
                    b'"' | b'\'' => in_quote = Some(c),
                    b'/' => {
                        // Could be the self-close marker. Peek next non-space.
                        let mut q = p + 1;
                        while q < self.src.len() && (self.src[q] == b' ' || self.src[q] == b'\t') {
                            q += 1;
                        }
                        if q < self.src.len() && self.src[q] == b'>' {
                            self_close = true;
                            p = q;
                            break;
                        }
                    }
                    b'>' => break,
                    _ => {}
                },
            }
            p += 1;
        }
        if p >= self.src.len() {
            return None;
        }
        let end = p + 1; // after '>'
        let kind = if kind_close {
            TagKind::Close
        } else if self_close {
            TagKind::SelfClose
        } else {
            TagKind::Open
        };
        Some((name, kind, end))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TagKind {
    Open,
    Close,
    SelfClose,
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return if needle.is_empty() { Some(0) } else { None };
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Walk the document and find the inner-text range of the first tag that
/// matches `selector`. The selector is interpreted as:
///
/// - `Some("/a/b/c")` — absolute path of element names from the root. The
///   first segment must equal the document root element. Empty leading
///   segment from the leading slash is ignored.
/// - `Some("//tag")` — first occurrence of `<tag>` at any depth (anywhere).
/// - `None` — Maven-aware default: first `<version>` that is a direct
///   child of the document root. Falls back to the first `<version>`
///   anywhere (preserving legacy behaviour) when none is found at depth 1.
fn find_target(content: &str, selector: Option<&str>) -> Option<InnerRange> {
    let path: Vec<&str> = match selector {
        Some(sel) if sel.starts_with("//") => {
            let name = sel.trim_start_matches('/');
            return find_first_named(content, name, None);
        }
        Some(sel) => sel.trim_start_matches('/').split('/').collect(),
        None => {
            // Default: try Maven-aware first, then the legacy first-match.
            if let Some(r) = find_root_child_named(content, "version") {
                return Some(r);
            }
            return find_first_named(content, "version", None);
        }
    };

    if path.is_empty() {
        return None;
    }
    walk_path(content, &path)
}

/// Find the first `<name>` whose ancestry from the root matches `path`
/// exactly. `path[0]` is the root element name; subsequent entries are
/// the nested children.
fn walk_path(content: &str, path: &[&str]) -> Option<InnerRange> {
    let mut s = Scanner::new(content);
    let mut stack: Vec<String> = Vec::new();

    while !s.done() {
        // Skip text until we hit '<'.
        while s.i < s.src.len() && s.src[s.i] != b'<' {
            s.i += 1;
        }
        if s.done() {
            break;
        }
        if s.skip_special() {
            continue;
        }
        let Some((name, kind, end)) = s.read_tag() else {
            s.i += 1;
            continue;
        };
        match kind {
            TagKind::Open => {
                stack.push(name.clone());
                if stack.len() == path.len()
                    && stack.iter().zip(path.iter()).all(|(a, b)| a.as_str() == *b)
                {
                    // Inner text starts at `end`. Find matching close tag
                    // for `name`, accounting for nested same-name tags.
                    let inner_start = end;
                    let close_idx = find_matching_close(&s.src[end..], &name)?;
                    return Some(InnerRange {
                        start: inner_start,
                        end: end + close_idx,
                    });
                }
                s.i = end;
            }
            TagKind::Close => {
                stack.pop();
                s.i = end;
            }
            TagKind::SelfClose => {
                s.i = end;
            }
        }
    }
    None
}

/// Find first `<name>` element (anywhere, or at depth 1 if `min_depth` is
/// `Some(1)` — depth here is 0-indexed, so depth 1 means a direct child of
/// the root element).
fn find_first_named(content: &str, name: &str, min_depth: Option<usize>) -> Option<InnerRange> {
    let mut s = Scanner::new(content);
    let mut depth: usize = 0;
    while !s.done() {
        while s.i < s.src.len() && s.src[s.i] != b'<' {
            s.i += 1;
        }
        if s.done() {
            break;
        }
        if s.skip_special() {
            continue;
        }
        let Some((tag, kind, end)) = s.read_tag() else {
            s.i += 1;
            continue;
        };
        match kind {
            TagKind::Open => {
                if tag == name && min_depth.is_none_or(|m| depth == m) {
                    let inner_start = end;
                    let close_idx = find_matching_close(&s.src[end..], &tag)?;
                    return Some(InnerRange {
                        start: inner_start,
                        end: end + close_idx,
                    });
                }
                depth += 1;
                s.i = end;
            }
            TagKind::Close => {
                depth = depth.saturating_sub(1);
                s.i = end;
            }
            TagKind::SelfClose => {
                s.i = end;
            }
        }
    }
    None
}

fn find_root_child_named(content: &str, name: &str) -> Option<InnerRange> {
    find_first_named(content, name, Some(1))
}

/// Given bytes starting just after an opening `<tag>`, find the byte
/// offset of the matching `</tag>` opening `<`. Handles nested same-name
/// tags by counting depth.
fn find_matching_close(after_open: &[u8], name: &str) -> Option<usize> {
    let mut s = Scanner {
        src: after_open,
        i: 0,
    };
    let mut depth: usize = 0;
    while !s.done() {
        while s.i < s.src.len() && s.src[s.i] != b'<' {
            s.i += 1;
        }
        if s.done() {
            return None;
        }
        let lt = s.i;
        if s.skip_special() {
            continue;
        }
        let Some((tag, kind, end)) = s.read_tag() else {
            s.i += 1;
            continue;
        };
        match kind {
            TagKind::Open => {
                if tag == name {
                    depth += 1;
                }
                s.i = end;
            }
            TagKind::Close => {
                if tag == name {
                    if depth == 0 {
                        return Some(lt);
                    }
                    depth -= 1;
                }
                s.i = end;
            }
            TagKind::SelfClose => s.i = end,
        }
    }
    None
}

impl VersionFile for XmlVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        self.read_version_with_selector(file_path, None)
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        self.write_version_with_selector(file_path, version, None)
    }

    fn read_version_with_selector(
        &self,
        file_path: &Path,
        selector: Option<&str>,
    ) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::XML_READ)?;
        let range = find_target(&content, selector)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No matching version tag found in {} (selector: {:?})",
                    file_path.display(),
                    selector
                )
            })
            .error_code(error_code::XML_VERSION_NOT_FOUND)?;
        Ok(content[range.start..range.end].trim().to_string())
    }

    fn write_version_with_selector(
        &self,
        file_path: &Path,
        version: &str,
        selector: Option<&str>,
    ) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::XML_READ)?;
        let range = find_target(&content, selector)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No matching version tag found to update in {} (selector: {:?})",
                    file_path.display(),
                    selector
                )
            })
            .error_code(error_code::XML_VERSION_NOT_FOUND)?;
        let mut new_content = String::with_capacity(content.len() + version.len());
        new_content.push_str(&content[..range.start]);
        new_content.push_str(version);
        new_content.push_str(&content[range.end..]);
        std::fs::write(file_path, new_content)
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::XML_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::XML_INVALID_UTF8)?;
        let range = find_target(text, None)
            .ok_or_else(|| anyhow::anyhow!("No <version> tag found in {filename}"))
            .error_code(error_code::XML_VERSION_NOT_FOUND)?;
        Ok(text[range.start..range.end].trim().to_string())
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

    const POM: &str = r#"<?xml version="1.0"?>
<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.example</groupId>
  <artifactId>myapp</artifactId>
  <version>1.0.0</version>
</project>"#;

    const SPRING_BOOT_POM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <parent>
        <groupId>org.springframework.boot</groupId>
        <artifactId>spring-boot-starter-parent</artifactId>
        <version>3.5.14</version>
    </parent>
    <groupId>com.homepedia</groupId>
    <artifactId>homepedia-backend</artifactId>
    <version>3.6.0</version>
    <packaging>pom</packaging>
</project>"#;

    #[test]
    fn read_pom_version() {
        let f = write_temp(POM);
        assert_eq!(XmlVersionFile.read_version(f.path()).unwrap(), "1.0.0");
    }

    #[test]
    fn read_no_version_fails() {
        let f = write_temp("<project><groupId>com.example</groupId></project>");
        assert!(XmlVersionFile.read_version(f.path()).is_err());
    }

    #[test]
    fn write_pom_version() {
        let f = write_temp(POM);
        XmlVersionFile.write_version(f.path(), "2.0.0").unwrap();
        assert_eq!(XmlVersionFile.read_version(f.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn write_replaces_first_version_tag_only() {
        let xml = "<project><version>1.0.0</version><dependencies><dependency><version>3.0</version></dependency></dependencies></project>";
        let f = write_temp(xml);
        XmlVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("<version>2.0.0</version>"));
        // The nested dependency version is untouched.
        assert!(content.contains("<version>3.0</version>"));
    }

    #[test]
    fn read_skips_parent_block_in_spring_boot_pom() {
        // Regression: with the legacy first-match regex, this returned
        // `3.5.14` (Spring Boot parent's version). We now want the
        // project's own `<version>3.6.0</version>`.
        let f = write_temp(SPRING_BOOT_POM);
        assert_eq!(XmlVersionFile.read_version(f.path()).unwrap(), "3.6.0");
    }

    #[test]
    fn write_skips_parent_block_in_spring_boot_pom() {
        let f = write_temp(SPRING_BOOT_POM);
        XmlVersionFile.write_version(f.path(), "3.7.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        // Project version got bumped.
        assert!(content.contains("<version>3.7.0</version>"));
        // Parent block was left alone — Spring Boot's version intact.
        assert!(content.contains("<version>3.5.14</version>"));
        // No duplicate version tags either side of the parent block.
        assert_eq!(content.matches("<version>").count(), 2);
    }

    #[test]
    fn explicit_selector_targets_specific_path() {
        // Force-select the parent's version even when a Maven default
        // would otherwise prefer the project version.
        let f = write_temp(SPRING_BOOT_POM);
        let v = XmlVersionFile
            .read_version_with_selector(f.path(), Some("/project/parent/version"))
            .unwrap();
        assert_eq!(v, "3.5.14");
    }

    #[test]
    fn explicit_selector_writes_through() {
        let f = write_temp(SPRING_BOOT_POM);
        XmlVersionFile
            .write_version_with_selector(f.path(), "9.9.9", Some("/project/parent/version"))
            .unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("<version>9.9.9</version>"));
        assert!(content.contains("<version>3.6.0</version>")); // project version untouched
    }

    #[test]
    fn double_slash_selector_finds_first_anywhere() {
        let f = write_temp(SPRING_BOOT_POM);
        let v = XmlVersionFile
            .read_version_with_selector(f.path(), Some("//version"))
            .unwrap();
        // First <version> in document order — that's the parent's.
        assert_eq!(v, "3.5.14");
    }

    #[test]
    fn comments_and_pi_are_ignored() {
        let xml = r#"<?xml version="1.0"?>
<!-- a leading comment with <version>fake</version> inside -->
<project>
    <!-- another <version>also-fake</version> -->
    <version>4.2.0</version>
</project>"#;
        let f = write_temp(xml);
        assert_eq!(XmlVersionFile.read_version(f.path()).unwrap(), "4.2.0");
    }

    #[test]
    fn read_from_bytes_uses_default_heuristic() {
        let v = XmlVersionFile
            .read_version_from_bytes(SPRING_BOOT_POM.as_bytes(), "pom.xml")
            .unwrap();
        assert_eq!(v, "3.6.0");
    }
}
