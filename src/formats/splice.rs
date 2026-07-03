use std::ops::Range;
use std::path::Path;

use anyhow::{Context, Result};

use crate::error_code::{ErrorCode, ErrorCodeExt};

pub(super) trait FormatPreservingEditor {
    fn locate_version(&self, content: &str, selector: Option<&str>) -> Result<Range<usize>>;
    fn read_error(&self) -> ErrorCode;
    fn write_error(&self) -> ErrorCode;
}

pub(super) fn write_via_splice<E: FormatPreservingEditor>(
    editor: &E,
    file_path: &Path,
    version: &str,
    selector: Option<&str>,
) -> Result<()> {
    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("Cannot read {}", file_path.display()))
        .error_code(editor.read_error())?;
    let range = editor.locate_version(&content, selector)?;
    let mut out = String::with_capacity(content.len() - range.len() + version.len());
    out.push_str(&content[..range.start]);
    out.push_str(version);
    out.push_str(&content[range.end..]);
    std::fs::write(file_path, out)
        .with_context(|| format!("Cannot write {}", file_path.display()))
        .error_code(editor.write_error())?;
    Ok(())
}
