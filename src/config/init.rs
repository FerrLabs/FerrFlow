use anyhow::Result;
use std::io::BufRead;
use std::path::PathBuf;

use crate::error_code::{self, ErrorCodeExt};

use super::Config;
use super::format::{CONFIG_FORMATS, ConfigFileFormat, format_handler};
use super::loader_js::{JS_CONFIG_FILENAME, TS_CONFIG_FILENAME};
use super::package::{FileFormat, PackageConfig, VersionedFile};
use super::workspace::WorkspaceConfig;
use super::workspace_discovery::{self, DiscoveredPackage};

struct Prompter<R> {
    input: R,
    exhausted: bool,
}

impl<R: BufRead> Prompter<R> {
    fn new(input: R) -> Self {
        Self {
            input,
            exhausted: false,
        }
    }

    fn ask(&mut self, question: &str, default: &str) -> String {
        use std::io::Write;
        if default.is_empty() {
            print!("{question}: ");
        } else {
            print!("{question} [{default}]: ");
        }
        std::io::stdout().flush().ok();
        let mut input = String::new();
        if matches!(self.input.read_line(&mut input), Ok(0) | Err(_)) {
            self.exhausted = true;
        }
        let trimmed = input.trim().to_string();
        if trimmed.is_empty() {
            default.to_string()
        } else {
            trimmed
        }
    }

    fn ask_bool(&mut self, question: &str, default: bool) -> bool {
        let hint = if default { "Y/n" } else { "y/N" };
        let answer = self.ask(&format!("{question} [{hint}]"), "");
        match answer.to_lowercase().as_str() {
            "y" | "yes" => true,
            "n" | "no" => false,
            _ => default,
        }
    }

    fn ask_format(&mut self, indent: bool) -> String {
        let question = if indent {
            "  Version file format [toml/json/xml/gradle/gomod/txt]"
        } else {
            "Version file format [toml/json/xml/gradle/gomod/txt]"
        };
        loop {
            let input = self.ask(question, "toml");
            let normalized = input.trim().to_lowercase();
            if ALLOWED_FORMATS.contains(&normalized.as_str()) {
                return normalized;
            }
            eprintln!(
                "Invalid format '{}'. Allowed values: toml, json, xml, gradle, gomod, txt.",
                input
            );
        }
    }

    fn ask_config_format(&mut self) -> ConfigFileFormat {
        let question = "Config file format [json/json5/toml/dotfile]";
        loop {
            let input = self.ask(question, "json");
            let normalized = input.trim().to_lowercase();
            if ALLOWED_CONFIG_FORMATS.contains(&normalized.as_str()) {
                return match normalized.as_str() {
                    "json5" => ConfigFileFormat::Json5,
                    "toml" => ConfigFileFormat::Toml,
                    "dotfile" | ".ferrflow" => ConfigFileFormat::Dotfile,
                    _ => ConfigFileFormat::Json,
                };
            }
            eprintln!(
                "Invalid format '{}'. Allowed values: json, json5, toml, dotfile.",
                input
            );
        }
    }

    fn package(&mut self, path_default: &str, monorepo: bool) -> PackageConfig {
        let dir_name = std::env::current_dir()
            .ok()
            .and_then(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "project".to_string());

        let name_default = if monorepo { "" } else { dir_name.as_str() };
        let name = self.ask(&question("Package name", monorepo), name_default);

        let path = self.ask(&question("Path", monorepo), path_default);

        let format_str = self.ask_format(monorepo);

        let version_file_path = self.ask(
            &question("Version file path", monorepo),
            &under(&path, default_version_file(&format_str)),
        );

        let changelog = self.ask(
            &question("Changelog path", monorepo),
            &under(&path, "CHANGELOG.md"),
        );

        package_config(
            name,
            path,
            vec![VersionedFile {
                path: version_file_path,
                format: parse_file_format(&format_str),
                selector: None,
            }],
            changelog,
        )
    }

    fn discovered(&mut self, found: &[DiscoveredPackage]) -> Option<Vec<PackageConfig>> {
        if found.is_empty() {
            return None;
        }
        println!("Found {} workspace package(s):", found.len());
        for package in found {
            println!("  {} ({})", package.path, package.name);
        }
        if !self.ask_bool("Use these packages?", true) {
            return None;
        }
        Some(found.iter().map(discovered_package).collect())
    }

    fn packages(&mut self, monorepo: bool) -> Result<Vec<PackageConfig>> {
        if !monorepo {
            return Ok(vec![self.package(".", false)]);
        }
        println!("Add packages (leave name empty to finish):");
        let mut pkgs = Vec::new();
        loop {
            let pkg = self.package("", true);
            if !pkg.name.is_empty() {
                pkgs.push(pkg);
            } else if !pkgs.is_empty() {
                return Ok(pkgs);
            } else if self.exhausted {
                anyhow::bail!("input ended before any package was entered");
            } else {
                eprintln!("At least one package is required.");
            }
        }
    }
}

fn package_config(
    name: String,
    path: String,
    versioned_files: Vec<VersionedFile>,
    changelog: String,
) -> PackageConfig {
    PackageConfig {
        name,
        path,
        versioned_files,
        changelog: Some(changelog),
        shared_paths: Vec::new(),
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        version_template: None,
        hooks: None,
        floating_tags: None,
        latest_tag: None,
        build_metadata: None,
        publishers: vec![],
        update_lockfiles: None,
        version_source: None,
    }
}

fn discovered_package(found: &DiscoveredPackage) -> PackageConfig {
    package_config(
        found.name.clone(),
        found.path.clone(),
        vec![VersionedFile {
            path: under(&found.path, found.manifest),
            format: found.format.clone(),
            selector: None,
        }],
        under(&found.path, "CHANGELOG.md"),
    )
}

const ALLOWED_FORMATS: &[&str] = &["toml", "json", "xml", "gradle", "gomod", "txt"];

const ALLOWED_CONFIG_FORMATS: &[&str] = &["json", "json5", "toml", "dotfile"];

fn default_version_file(format: &str) -> &'static str {
    match format {
        "json" => "package.json",
        "xml" => "pom.xml",
        "gradle" => "build.gradle",
        "gomod" => "go.mod",
        "txt" => "VERSION.txt",
        _ => "Cargo.toml",
    }
}

fn parse_file_format(s: &str) -> FileFormat {
    match s {
        "json" => FileFormat::Json,
        "xml" => FileFormat::Xml,
        "gradle" => FileFormat::Gradle,
        "gomod" => FileFormat::GoMod,
        "txt" => FileFormat::Txt,
        _ => FileFormat::Toml,
    }
}

fn question(text: &str, indented: bool) -> String {
    if indented {
        format!("  {text}")
    } else {
        text.to_string()
    }
}

fn under(dir: &str, file: &str) -> String {
    if dir == "." {
        file.to_string()
    } else {
        format!("{dir}/{file}")
    }
}

const DEFAULT_MANIFEST_FILE: &str = ".ferrflow.manifest.json";

fn ensure_no_config_exists() -> Result<()> {
    let existing = CONFIG_FORMATS
        .iter()
        .map(|handler| handler.filename())
        .chain([TS_CONFIG_FILENAME, JS_CONFIG_FILENAME])
        .find(|filename| PathBuf::from(filename).exists());
    if let Some(filename) = existing {
        Err(anyhow::anyhow!("{filename} already exists"))
            .error_code(error_code::CONFIG_ALREADY_EXISTS)?;
    }
    Ok(())
}

pub fn init(format: Option<ConfigFileFormat>, manifest: bool) -> Result<()> {
    init_from(std::io::stdin().lock(), format, manifest)
}

fn init_from(input: impl BufRead, format: Option<ConfigFileFormat>, manifest: bool) -> Result<()> {
    ensure_no_config_exists()?;

    let mut prompter = Prompter::new(input);

    let fmt = format.unwrap_or_else(|| prompter.ask_config_format());
    let handler = format_handler(fmt);

    let found = workspace_discovery::discover(&std::env::current_dir()?);
    let packages = match prompter.discovered(&found) {
        Some(packages) => packages,
        None => {
            let monorepo = prompter.ask_bool("Is this a monorepo?", false);
            prompter.packages(monorepo)?
        }
    };

    let mut workspace = WorkspaceConfig::default();
    if manifest {
        workspace.manifest_file = Some(DEFAULT_MANIFEST_FILE.to_string());
    }

    let config = Config {
        include: Vec::new(),
        workspace,
        packages,
    };

    let content = handler.serialize(&config)?;
    let filename = handler.filename();
    std::fs::write(filename, &content)?;
    println!("Created {filename}");

    if manifest {
        let root = std::env::current_dir()?;
        let packages = crate::manifest::initial_snapshot(&config, &root);
        let initial = crate::manifest::Manifest::new(
            packages,
            crate::manifest::now_utc_iso8601(),
            String::new(),
        );
        crate::manifest::write_atomic(&root.join(DEFAULT_MANIFEST_FILE), &initial)?;
        println!("Created {DEFAULT_MANIFEST_FILE}");
    }

    println!("Run: ferrflow check");

    Ok(())
}

#[cfg(test)]
mod tests;
