use anyhow::Result;
use std::path::PathBuf;

use crate::error_code::{self, ErrorCodeExt};

use super::Config;
use super::format::{CONFIG_FORMATS, ConfigFileFormat, format_handler};
use super::loader_js::{JS_CONFIG_FILENAME, TS_CONFIG_FILENAME};
use super::package::{FileFormat, PackageConfig, VersionedFile};
use super::workspace::WorkspaceConfig;

fn prompt(question: &str, default: &str) -> String {
    use std::io::Write;
    if default.is_empty() {
        print!("{question}: ");
    } else {
        print!("{question} [{default}]: ");
    }
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed
    }
}

fn prompt_bool(question: &str, default: bool) -> bool {
    let hint = if default { "Y/n" } else { "y/N" };
    let answer = prompt(&format!("{question} [{hint}]"), "");
    match answer.to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default,
    }
}

const ALLOWED_FORMATS: &[&str] = &["toml", "json", "xml", "gradle", "gomod", "txt"];

fn prompt_format(indent: bool) -> String {
    let question = if indent {
        "  Version file format [toml/json/xml/gradle/gomod/txt]"
    } else {
        "Version file format [toml/json/xml/gradle/gomod/txt]"
    };
    loop {
        let input = prompt(question, "toml");
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

const ALLOWED_CONFIG_FORMATS: &[&str] = &["json", "json5", "toml", "dotfile"];

fn prompt_config_format() -> ConfigFileFormat {
    let question = "Config file format [json/json5/toml/dotfile]";
    loop {
        let input = prompt(question, "json");
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

fn collect_package(path_default: &str, monorepo: bool) -> PackageConfig {
    let dir_name = std::env::current_dir()
        .ok()
        .and_then(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "project".to_string());

    let name_default = if monorepo { "" } else { dir_name.as_str() };
    let name = prompt(&question("Package name", monorepo), name_default);

    let path = prompt(&question("Path", monorepo), path_default);

    let format_str = prompt_format(monorepo);

    let version_file_path = prompt(
        &question("Version file path", monorepo),
        &under(&path, default_version_file(&format_str)),
    );

    let changelog = prompt(
        &question("Changelog path", monorepo),
        &under(&path, "CHANGELOG.md"),
    );

    PackageConfig {
        name,
        path,
        versioned_files: vec![VersionedFile {
            path: version_file_path,
            format: parse_file_format(&format_str),
            selector: None,
        }],
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

fn collect_packages(monorepo: bool) -> Vec<PackageConfig> {
    if !monorepo {
        return vec![collect_package(".", false)];
    }
    println!("Add packages (leave name empty to finish):");
    let mut pkgs = Vec::new();
    loop {
        let pkg = collect_package("", true);
        if !pkg.name.is_empty() {
            pkgs.push(pkg);
        } else if pkgs.is_empty() {
            eprintln!("At least one package is required.");
        } else {
            return pkgs;
        }
    }
}

pub fn init(format: Option<ConfigFileFormat>, manifest: bool) -> Result<()> {
    ensure_no_config_exists()?;

    let fmt = format.unwrap_or_else(prompt_config_format);
    let handler = format_handler(fmt);

    let monorepo = prompt_bool("Is this a monorepo?", false);

    let packages = collect_packages(monorepo);

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
