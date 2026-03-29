pub mod gomod;
pub mod gradle;
pub mod json;
pub mod toml_format;
pub mod txt;
pub mod xml;

use anyhow::Result;
use std::path::Path;

use crate::config::{FileFormat, VersionedFile};

pub trait VersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String>;
    fn write_version(&self, file_path: &Path, version: &str) -> Result<()>;
    /// Returns false if this format tracks versions via git tags only and
    /// does not write any file. Callers should skip committing the path.
    fn modifies_file(&self) -> bool {
        true
    }
}

pub fn get_handler(format: &FileFormat) -> Box<dyn VersionFile> {
    match format {
        FileFormat::GoMod => Box::new(gomod::GoModVersionFile),
        FileFormat::Gradle => Box::new(gradle::GradleVersionFile),
        FileFormat::Json => Box::new(json::JsonVersionFile),
        FileFormat::Toml => Box::new(toml_format::TomlVersionFile),
        FileFormat::Txt => Box::new(txt::TxtVersionFile),
        FileFormat::Xml => Box::new(xml::XmlVersionFile),
    }
}

pub fn read_version(vf: &VersionedFile, repo_root: &Path) -> Result<String> {
    let path = repo_root.join(&vf.path);
    let handler = get_handler(&vf.format);
    handler.read_version(&path)
}

pub fn write_version(vf: &VersionedFile, repo_root: &Path, version: &str) -> Result<()> {
    let path = repo_root.join(&vf.path);
    let handler = get_handler(&vf.format);
    handler.write_version(&path, version)
}
