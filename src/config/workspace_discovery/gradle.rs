use std::path::Path;

use regex::Regex;

use super::{DiscoveredPackage, Source};
use crate::config::package::FileFormat;

const SETTINGS: [&str; 2] = ["settings.gradle.kts", "settings.gradle"];
const BUILD_FILES: [&str; 2] = ["build.gradle.kts", "build.gradle"];

pub(super) fn source(root: &Path) -> Source {
    Source {
        globs: includes(root),
        read: read_project,
    }
}

fn includes(root: &Path) -> Vec<String> {
    let Some(raw) = SETTINGS
        .iter()
        .find_map(|name| std::fs::read_to_string(root.join(name)).ok())
    else {
        return Vec::new();
    };
    let (Ok(statement), Ok(quoted)) = (
        Regex::new(r"(?m)^\s*include\s*\(([^)]*)\)|^\s*include\s+([^\n(][^\n]*)"),
        Regex::new(r#"["']([^"']+)["']"#),
    ) else {
        return Vec::new();
    };
    statement
        .captures_iter(&raw)
        .flat_map(|c| {
            quoted
                .captures_iter(c.get(1).or_else(|| c.get(2)).map_or("", |m| m.as_str()))
                .map(|q| q[1].trim_matches(':').replace(':', "/"))
                .collect::<Vec<_>>()
        })
        .filter(|path| !path.is_empty())
        .collect()
}

fn read_project(_root: &Path, dir: &Path, rel: &str) -> Option<DiscoveredPackage> {
    let manifest = BUILD_FILES.iter().find(|name| dir.join(name).is_file())?;
    Some(DiscoveredPackage {
        name: rel.rsplit('/').next()?.to_string(),
        path: rel.to_string(),
        manifest,
        format: FileFormat::Gradle,
    })
}
