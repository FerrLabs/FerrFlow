use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use std::path::Path;

pub struct XmlVersionFile;

#[derive(Debug, Clone, Copy)]
struct InnerRange {
    start: usize,
    end: usize,
}

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

    fn skip_special(&mut self) -> bool {
        let src = self.src;
        let rest = &src[self.i..];
        for (open, close) in DELIMITED_SPECIALS {
            if rest.starts_with(open) {
                self.i += open.len();
                self.i = self
                    .find_at(close)
                    .map_or(src.len(), |end| end + close.len());
                return true;
            }
        }
        if rest.starts_with(b"<!") {
            self.i += 2;
            self.i = self.find_at(b">").map_or(src.len(), |end| end + 1);
            return true;
        }
        false
    }

    fn find_at(&self, needle: &[u8]) -> Option<usize> {
        find_subslice(&self.src[self.i..], needle).map(|p| self.i + p)
    }

    fn next_tag(&mut self) -> Option<Tag> {
        loop {
            self.i += self.src[self.i..]
                .iter()
                .take_while(|&&c| c != b'<')
                .count();
            if self.done() {
                return None;
            }
            if self.skip_special() {
                continue;
            }
            let start = self.i;
            match self.read_tag() {
                Some((name, kind, end)) => {
                    self.i = end;
                    return Some(Tag {
                        name,
                        kind,
                        start,
                        end,
                    });
                }
                None => self.i += 1,
            }
        }
    }

    fn read_tag(&self) -> Option<(String, TagKind, usize)> {
        debug_assert_eq!(self.src.get(self.i).copied(), Some(b'<'));
        let mut p = self.i + 1;
        let closing = self.src.get(p) == Some(&b'/');
        if closing {
            p += 1;
        }
        let (name, after_name) = self.read_name(p)?;
        let (gt, self_close) = self.find_tag_end(after_name)?;
        let kind = match (closing, self_close) {
            (true, _) => TagKind::Close,
            (false, true) => TagKind::SelfClose,
            (false, false) => TagKind::Open,
        };
        Some((name, kind, gt + 1))
    }

    fn read_name(&self, start: usize) -> Option<(String, usize)> {
        let len = self.src[start..]
            .iter()
            .take_while(|c| !matches!(c, b' ' | b'\t' | b'\n' | b'\r' | b'/' | b'>'))
            .count();
        if len == 0 {
            return None;
        }
        let end = start + len;
        let name = std::str::from_utf8(&self.src[start..end]).ok()?.to_string();
        Some((name, end))
    }

    fn find_tag_end(&self, from: usize) -> Option<(usize, bool)> {
        let mut in_quote: Option<u8> = None;
        for (p, &c) in self.src.iter().enumerate().skip(from) {
            match (in_quote, c) {
                (Some(q), c) if c == q => in_quote = None,
                (Some(_), _) => {}
                (None, b'"' | b'\'') => in_quote = Some(c),
                (None, b'/') => {
                    if let Some(gt) = self.gt_after_slash(p) {
                        return Some((gt, true));
                    }
                }
                (None, b'>') => return Some((p, false)),
                _ => {}
            }
        }
        None
    }

    fn gt_after_slash(&self, slash: usize) -> Option<usize> {
        let blanks = self.src[slash + 1..]
            .iter()
            .take_while(|c| matches!(c, b' ' | b'\t'))
            .count();
        let gt = slash + 1 + blanks;
        (self.src.get(gt) == Some(&b'>')).then_some(gt)
    }
}

const DELIMITED_SPECIALS: [(&[u8], &[u8]); 3] =
    [(b"<!--", b"-->"), (b"<![CDATA[", b"]]>"), (b"<?", b"?>")];

struct Tag {
    name: String,
    kind: TagKind,
    start: usize,
    end: usize,
}

fn inner_range(src: &[u8], open: &Tag) -> Option<InnerRange> {
    let close = find_matching_close(&src[open.end..], &open.name)?;
    Some(InnerRange {
        start: open.end,
        end: open.end + close,
    })
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

fn find_target(content: &str, selector: Option<&str>) -> Option<InnerRange> {
    let path: Vec<&str> = match selector {
        Some(sel) if sel.starts_with("//") => {
            let name = sel.trim_start_matches('/');
            return find_first_named(content, name, None);
        }
        Some(sel) => sel.trim_start_matches('/').split('/').collect(),
        None => {
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

fn walk_path(content: &str, path: &[&str]) -> Option<InnerRange> {
    let mut s = Scanner::new(content);
    let mut stack: Vec<String> = Vec::new();
    while let Some(tag) = s.next_tag() {
        match tag.kind {
            TagKind::Open => {
                stack.push(tag.name.clone());
                if stack.iter().map(String::as_str).eq(path.iter().copied()) {
                    return inner_range(s.src, &tag);
                }
            }
            TagKind::Close => {
                stack.pop();
            }
            TagKind::SelfClose => {}
        }
    }
    None
}

fn find_first_named(content: &str, name: &str, min_depth: Option<usize>) -> Option<InnerRange> {
    let mut s = Scanner::new(content);
    let mut depth: usize = 0;
    while let Some(tag) = s.next_tag() {
        match tag.kind {
            TagKind::Open if tag.name == name && min_depth.is_none_or(|m| depth == m) => {
                return inner_range(s.src, &tag);
            }
            TagKind::Open => depth += 1,
            TagKind::Close => depth = depth.saturating_sub(1),
            TagKind::SelfClose => {}
        }
    }
    None
}

fn find_root_child_named(content: &str, name: &str) -> Option<InnerRange> {
    find_first_named(content, name, Some(1))
}

fn find_matching_close(after_open: &[u8], name: &str) -> Option<usize> {
    let mut s = Scanner {
        src: after_open,
        i: 0,
    };
    let mut depth: usize = 0;
    while let Some(tag) = s.next_tag() {
        if tag.name != name {
            continue;
        }
        match tag.kind {
            TagKind::Open => depth += 1,
            TagKind::Close if depth == 0 => return Some(tag.start),
            TagKind::Close => depth -= 1,
            TagKind::SelfClose => {}
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
        self.read_version_from_bytes_with_selector(content, filename, None)
    }

    fn read_version_from_bytes_with_selector(
        &self,
        content: &[u8],
        filename: &str,
        selector: Option<&str>,
    ) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::XML_INVALID_UTF8)?;
        let range = find_target(text, selector)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No matching version tag found in {filename} (selector: {selector:?})"
                )
            })
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
        assert!(content.contains("<version>3.0</version>"));
    }

    #[test]
    fn read_skips_parent_block_in_spring_boot_pom() {
        let f = write_temp(SPRING_BOOT_POM);
        assert_eq!(XmlVersionFile.read_version(f.path()).unwrap(), "3.6.0");
    }

    #[test]
    fn write_skips_parent_block_in_spring_boot_pom() {
        let f = write_temp(SPRING_BOOT_POM);
        XmlVersionFile.write_version(f.path(), "3.7.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("<version>3.7.0</version>"));
        assert!(content.contains("<version>3.5.14</version>"));
        assert_eq!(content.matches("<version>").count(), 2);
    }

    #[test]
    fn explicit_selector_targets_specific_path() {
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

    #[test]
    fn read_bytes_uses_the_selector() {
        let content = br#"<Project><PropertyGroup><AssemblyVersion>9.9.9</AssemblyVersion><Version>1.2.3</Version></PropertyGroup></Project>"#;
        let v = XmlVersionFile
            .read_version_from_bytes_with_selector(content, "app.csproj", Some("//AssemblyVersion"))
            .unwrap();
        assert_eq!(v, "9.9.9");
    }
}
