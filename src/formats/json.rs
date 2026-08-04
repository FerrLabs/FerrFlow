use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use std::path::Path;

pub struct JsonVersionFile;

/// Find the byte span of the *string contents* (between the surrounding
/// quotes) of a top-level key's value in a JSON document.
///
/// Returns the start..end byte offsets of the value text, so the caller
/// can do `content[..start] + new_value + content[end..]` to replace just
/// that span — preserving every byte of indentation, key order, escape
/// characters in other strings, and trailing-newline state in the
/// surrounding document.
///
/// Returns `None` when:
/// - the document is not an object,
/// - the requested key is missing at the top level,
/// - the value of the requested key is not a string.
///
/// In any of those cases the caller should fall back to the serde
/// round-trip path.
///
/// Caller must ensure `content` parses as JSON (we do that with
/// `serde_json::from_str` before calling this). This walker doesn't try
/// to be lenient with malformed input — it returns `None` and lets the
/// fallback handle it.
fn find_top_level_string_value_span(content: &str, target_key: &str) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    let n = bytes.len();
    let mut i = skip_ws(bytes, 0);
    if i >= n || bytes[i] != b'{' {
        return None;
    }
    i += 1;

    loop {
        i = skip_ws(bytes, i);
        if i >= n {
            return None;
        }
        if bytes[i] == b'}' {
            return None;
        }
        if bytes[i] == b',' {
            i += 1;
            continue;
        }
        if bytes[i] != b'"' {
            return None;
        }
        let (key_start, key_end, after_key) = read_string(bytes, i)?;
        let key = std::str::from_utf8(&bytes[key_start..key_end]).ok()?;
        i = skip_ws(bytes, after_key);
        if i >= n || bytes[i] != b':' {
            return None;
        }
        i = skip_ws(bytes, i + 1);
        if i >= n {
            return None;
        }

        if key == target_key {
            if bytes[i] != b'"' {
                return None;
            }
            let (val_start, val_end, _) = read_string(bytes, i)?;
            return Some((val_start, val_end));
        }

        i = skip_value(bytes, i)?;
    }
}

/// Returns the byte index of the first non-whitespace byte at or after
/// `start`, or `bytes.len()` if all remaining bytes are whitespace.
/// Span of the string value at `section.key`, e.g. `dependencies.core`.
///
/// Same contract as [`find_top_level_string_value_span`] — the caller has
/// already validated the JSON — but descends one level so a dependency
/// constraint can be spliced without reformatting the file.
pub(super) fn find_nested_string_value_span(
    content: &str,
    section: &str,
    key: &str,
) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    let section_start = find_object_value_start(bytes, 0, section)?;
    let (start, end) = find_object_value_start(bytes, section_start, key)
        .and_then(|value_start| read_string(bytes, value_start).map(|(s, e, _)| (s, e)))?;
    Some((start, end))
}

/// Offset of the value for `target_key` in the object beginning at `from`.
fn find_object_value_start(bytes: &[u8], from: usize, target_key: &str) -> Option<usize> {
    let n = bytes.len();
    let mut i = skip_ws(bytes, from);
    if i >= n || bytes[i] != b'{' {
        return None;
    }
    i += 1;

    loop {
        i = skip_ws(bytes, i);
        if i >= n || bytes[i] == b'}' {
            return None;
        }
        if bytes[i] == b',' {
            i += 1;
            continue;
        }
        if bytes[i] != b'"' {
            return None;
        }
        let (key_start, key_end, after_key) = read_string(bytes, i)?;
        let key = std::str::from_utf8(&bytes[key_start..key_end]).ok()?;
        i = skip_ws(bytes, after_key);
        if i >= n || bytes[i] != b':' {
            return None;
        }
        i = skip_ws(bytes, i + 1);
        if i >= n {
            return None;
        }
        if key == target_key {
            return Some(i);
        }
        i = skip_value(bytes, i)?;
    }
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Read a JSON string starting at the opening quote at index `i`.
/// Returns `(content_start, content_end, index_after_closing_quote)`,
/// where `content_start..content_end` covers the string's bytes between
/// the quotes (escapes still encoded), or `None` on truncated input.
fn read_string(bytes: &[u8], i: usize) -> Option<(usize, usize, usize)> {
    let n = bytes.len();
    if i >= n || bytes[i] != b'"' {
        return None;
    }
    let start = i + 1;
    let mut j = start;
    while j < n {
        match bytes[j] {
            b'\\' => {
                if j + 1 >= n {
                    return None;
                }
                j += 2;
            }
            b'"' => return Some((start, j, j + 1)),
            _ => j += 1,
        }
    }
    None
}

/// Skip past a JSON value (string, number, bool, null, object, array)
/// starting at byte `i`. Returns the index right after the value, or
/// `None` on truncated/malformed input.
fn skip_value(bytes: &[u8], i: usize) -> Option<usize> {
    let n = bytes.len();
    if i >= n {
        return None;
    }
    match bytes[i] {
        b'"' => read_string(bytes, i).map(|(_, _, after)| after),
        b'{' | b'[' => {
            let open = bytes[i];
            let close = if open == b'{' { b'}' } else { b']' };
            let mut depth: i32 = 1;
            let mut j = i + 1;
            while j < n && depth > 0 {
                match bytes[j] {
                    b'"' => {
                        let (_, _, after) = read_string(bytes, j)?;
                        j = after;
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    c if c == close => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            if depth == 0 { Some(j) } else { None }
        }
        _ => {
            let mut j = i;
            while j < n && !matches!(bytes[j], b',' | b'}' | b']') {
                j += 1;
            }
            Some(j)
        }
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

    #[test]
    fn read_version_from_package_json() {
        let f = write_temp(r#"{"name":"foo","version":"1.2.3"}"#);
        let handler = JsonVersionFile;
        assert_eq!(handler.read_version(f.path()).unwrap(), "1.2.3");
    }

    #[test]
    fn read_version_missing_field() {
        let f = write_temp(r#"{"name":"foo"}"#);
        let handler = JsonVersionFile;
        assert!(handler.read_version(f.path()).is_err());
    }

    #[test]
    fn write_version_updates_field() {
        let f = write_temp(r#"{"name":"foo","version":"1.0.0"}"#);
        let handler = JsonVersionFile;
        handler.write_version(f.path(), "2.0.0").unwrap();
        assert_eq!(handler.read_version(f.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn write_preserves_other_fields() {
        let f = write_temp(r#"{"name":"foo","version":"1.0.0","private":true}"#);
        let handler = JsonVersionFile;
        handler.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["name"], "foo");
        assert_eq!(v["private"], true);
        assert_eq!(v["version"], "2.0.0");
    }

    #[test]
    fn write_preserves_tab_indentation() {
        let original =
            "{\n\t\"name\": \"foo\",\n\t\"version\": \"1.0.0\",\n\t\"private\": true\n}\n";
        let f = write_temp(original);
        JsonVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let after = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(
            after, "{\n\t\"name\": \"foo\",\n\t\"version\": \"2.0.0\",\n\t\"private\": true\n}\n",
            "tab indentation and trailing newline must be preserved"
        );
    }

    #[test]
    fn write_preserves_four_space_indentation() {
        let original =
            "{\n    \"name\": \"foo\",\n    \"version\": \"1.0.0\",\n    \"private\": true\n}\n";
        let f = write_temp(original);
        JsonVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let after = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(
            after,
            "{\n    \"name\": \"foo\",\n    \"version\": \"2.0.0\",\n    \"private\": true\n}\n"
        );
    }

    #[test]
    fn write_preserves_missing_trailing_newline() {
        let original = "{\n  \"name\": \"foo\",\n  \"version\": \"1.0.0\"\n}";
        let f = write_temp(original);
        JsonVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let after = std::fs::read_to_string(f.path()).unwrap();
        assert!(
            !after.ends_with('\n'),
            "must not invent a trailing newline that wasn't there"
        );
        assert_eq!(
            after,
            "{\n  \"name\": \"foo\",\n  \"version\": \"2.0.0\"\n}"
        );
    }

    #[test]
    fn write_single_line_diff() {
        let original = "{\n  \"name\": \"foo\",\n  \"version\": \"1.0.0\",\n  \"private\": true,\n  \"scripts\": {\n    \"build\": \"tsc\"\n  }\n}\n";
        let f = write_temp(original);
        JsonVersionFile.write_version(f.path(), "1.0.1").unwrap();
        let after = std::fs::read_to_string(f.path()).unwrap();
        let differing_lines: Vec<(usize, &str, &str)> = original
            .lines()
            .zip(after.lines())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, (a, b))| (i, a, b))
            .collect();
        assert_eq!(
            differing_lines.len(),
            1,
            "expected a single-line diff, got {differing_lines:?}"
        );
        assert!(differing_lines[0].1.contains("1.0.0"));
        assert!(differing_lines[0].2.contains("1.0.1"));
    }

    #[test]
    fn write_ignores_nested_version_keys() {
        // package.json frequently has nested `"version"` keys inside
        // dependencies, engines, etc. The top-level bump must not touch
        // those.
        let original = "{\n  \"name\": \"app\",\n  \"version\": \"1.0.0\",\n  \"dependencies\": {\n    \"left-pad\": {\n      \"version\": \"9.9.9\"\n    }\n  }\n}\n";
        let f = write_temp(original);
        JsonVersionFile.write_version(f.path(), "1.0.1").unwrap();
        let after = std::fs::read_to_string(f.path()).unwrap();
        assert!(after.contains("\"version\": \"1.0.1\""));
        assert!(
            after.contains("\"version\": \"9.9.9\""),
            "nested dependency version must remain untouched"
        );
    }

    #[test]
    fn write_preserves_key_order() {
        let f = write_temp(
            r#"{"name":"foo","version":"1.0.0","private":true,"description":"x","scripts":{"build":"tsc"}}"#,
        );
        let handler = JsonVersionFile;
        handler.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();

        let name_pos = content.find("\"name\"").unwrap();
        let version_pos = content.find("\"version\"").unwrap();
        let private_pos = content.find("\"private\"").unwrap();
        let description_pos = content.find("\"description\"").unwrap();
        let scripts_pos = content.find("\"scripts\"").unwrap();

        assert!(name_pos < version_pos, "name must come before version");
        assert!(
            version_pos < private_pos,
            "version must come before private"
        );
        assert!(
            private_pos < description_pos,
            "private must come before description"
        );
        assert!(
            description_pos < scripts_pos,
            "description must come before scripts"
        );
    }
}

impl VersionFile for JsonVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::JSON_READ)?;
        let v: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| format!("Invalid JSON in {}", file_path.display()))
            .error_code(error_code::JSON_PARSE)?;
        v["version"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No 'version' field in {}", file_path.display()))
            .error_code(error_code::JSON_VERSION_NOT_FOUND)
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::JSON_READ)?;

        // Validate the file is well-formed JSON before we touch it; this
        // catches the case where someone hand-edited it into garbage and
        // we'd otherwise surgically write into the middle of a broken doc.
        serde_json::from_str::<serde_json::Value>(&content)
            .with_context(|| format!("Invalid JSON in {}", file_path.display()))
            .error_code(error_code::JSON_PARSE)?;

        let new_content = match find_top_level_string_value_span(&content, "version") {
            Some((start, end)) => {
                let mut s = String::with_capacity(content.len() + version.len());
                s.push_str(&content[..start]);
                s.push_str(version);
                s.push_str(&content[end..]);
                s
            }
            None => {
                // No top-level string `version` field — fall back to the
                // serde round-trip so we still bump (acquiring a `version`
                // field on a file that didn't have one). This loses
                // formatting, but the alternative is silently failing.
                let mut v: serde_json::Value = serde_json::from_str(&content)
                    .with_context(|| format!("Invalid JSON in {}", file_path.display()))
                    .error_code(error_code::JSON_PARSE)?;
                v["version"] = serde_json::Value::String(version.to_string());
                serde_json::to_string_pretty(&v)
                    .with_context(|| format!("Cannot serialize JSON for {}", file_path.display()))
                    .error_code(error_code::JSON_WRITE)?
                    + "\n"
            }
        };

        std::fs::write(file_path, new_content)
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::JSON_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::JSON_INVALID_UTF8)?;
        let v: serde_json::Value = serde_json::from_str(text)
            .with_context(|| format!("Invalid JSON in {filename}"))
            .error_code(error_code::JSON_PARSE)?;
        v["version"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No 'version' field in {filename}"))
            .error_code(error_code::JSON_VERSION_NOT_FOUND)
    }
}
