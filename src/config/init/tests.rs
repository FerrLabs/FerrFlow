use super::*;
use crate::test_utils::with_cwd;
use std::path::Path;

fn run_init(
    dir: &Path,
    answers: &str,
    format: Option<ConfigFileFormat>,
    manifest: bool,
) -> Result<()> {
    let mut outcome = None;
    with_cwd(dir, || {
        outcome = Some(init_from(answers.as_bytes(), format, manifest));
        Ok(())
    })
    .unwrap();
    outcome.unwrap()
}

fn load(dir: &Path, filename: &str) -> Config {
    Config::load(dir, Some(&dir.join(filename))).expect("init must write a loadable config")
}

fn lines(answers: &[&str]) -> String {
    answers.iter().map(|a| format!("{a}\n")).collect()
}

#[test]
fn accepting_every_default_scaffolds_one_cargo_package_named_after_the_directory() {
    let dir = tempfile::tempdir().unwrap();
    let dir_name = dir
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    run_init(dir.path(), "", Some(ConfigFileFormat::Json), false).unwrap();

    let config = load(dir.path(), "ferrflow.json");
    assert_eq!(config.packages.len(), 1);
    let pkg = &config.packages[0];
    assert_eq!(pkg.name, dir_name);
    assert_eq!(pkg.path, ".");
    assert_eq!(pkg.versioned_files[0].path, "Cargo.toml");
    assert_eq!(pkg.versioned_files[0].format, FileFormat::Toml);
    assert_eq!(pkg.changelog.as_deref(), Some("CHANGELOG.md"));
    assert!(config.workspace.manifest_file.is_none());
    assert!(!dir.path().join(DEFAULT_MANIFEST_FILE).exists());
}

#[test]
fn the_config_format_answer_picks_the_file_that_gets_written() {
    for (answer, filename) in [
        ("toml", "ferrflow.toml"),
        ("JSON5", "ferrflow.json5"),
        ("dotfile", ".ferrflow"),
        ("", "ferrflow.json"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        run_init(dir.path(), &lines(&[answer]), None, false).unwrap();
        assert!(
            dir.path().join(filename).is_file(),
            "answering {answer:?} should write {filename}"
        );
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            1,
            "only {filename} should be written for {answer:?}"
        );
    }
}

#[test]
fn an_unknown_config_format_is_asked_again() {
    let dir = tempfile::tempdir().unwrap();
    run_init(dir.path(), &lines(&["yaml", "toml"]), None, false).unwrap();
    assert!(dir.path().join("ferrflow.toml").is_file());
    assert!(!dir.path().join("ferrflow.json").exists());
}

#[test]
fn the_version_format_drives_the_default_version_file_and_its_parser() {
    for (format, file, parsed) in [
        ("json", "package.json", FileFormat::Json),
        ("xml", "pom.xml", FileFormat::Xml),
        ("gradle", "build.gradle", FileFormat::Gradle),
        ("gomod", "go.mod", FileFormat::GoMod),
        ("txt", "VERSION.txt", FileFormat::Txt),
        ("TOML", "Cargo.toml", FileFormat::Toml),
    ] {
        let dir = tempfile::tempdir().unwrap();
        run_init(
            dir.path(),
            &lines(&["n", "app", ".", format]),
            Some(ConfigFileFormat::Json),
            false,
        )
        .unwrap();
        let vf = &load(dir.path(), "ferrflow.json").packages[0].versioned_files[0];
        assert_eq!(vf.path, file, "format {format}");
        assert_eq!(vf.format, parsed, "format {format}");
    }
}

#[test]
fn an_invalid_version_format_is_asked_again_rather_than_defaulted() {
    let dir = tempfile::tempdir().unwrap();
    run_init(
        dir.path(),
        &lines(&["n", "app", ".", "yaml", "json"]),
        Some(ConfigFileFormat::Json),
        false,
    )
    .unwrap();
    let vf = &load(dir.path(), "ferrflow.json").packages[0].versioned_files[0];
    assert_eq!(vf.format, FileFormat::Json);
    assert_eq!(vf.path, "package.json");
}

#[test]
fn a_monorepo_collects_packages_until_an_empty_name_with_paths_under_each_package() {
    let dir = tempfile::tempdir().unwrap();
    run_init(
        dir.path(),
        &lines(&[
            "yes",
            "api",
            "packages/api",
            "gomod",
            "",
            "",
            "web",
            "packages/web",
            "json",
            "packages/web/custom.json",
            "docs/WEB.md",
            "",
        ]),
        Some(ConfigFileFormat::Json),
        false,
    )
    .unwrap();

    let config = load(dir.path(), "ferrflow.json");
    let names: Vec<&str> = config.packages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["api", "web"]);

    let api = &config.packages[0];
    assert_eq!(api.path, "packages/api");
    assert_eq!(api.versioned_files[0].path, "packages/api/go.mod");
    assert_eq!(api.versioned_files[0].format, FileFormat::GoMod);
    assert_eq!(api.changelog.as_deref(), Some("packages/api/CHANGELOG.md"));

    let web = &config.packages[1];
    assert_eq!(web.versioned_files[0].path, "packages/web/custom.json");
    assert_eq!(web.changelog.as_deref(), Some("docs/WEB.md"));
}

#[test]
fn a_monorepo_keeps_asking_while_no_package_has_been_named() {
    let dir = tempfile::tempdir().unwrap();
    run_init(
        dir.path(),
        &lines(&["y", "", "", "", "", "", "api", "api", "", "", "", ""]),
        Some(ConfigFileFormat::Json),
        false,
    )
    .unwrap();

    let config = load(dir.path(), "ferrflow.json");
    assert_eq!(config.packages.len(), 1);
    assert_eq!(config.packages[0].name, "api");
}

#[test]
fn a_monorepo_whose_input_ends_before_any_package_fails_instead_of_looping() {
    let dir = tempfile::tempdir().unwrap();

    let err = run_init(dir.path(), "y\n", Some(ConfigFileFormat::Json), false).unwrap_err();

    assert!(format!("{err:#}").contains("input ended"), "{err:#}");
    assert!(
        !dir.path().join("ferrflow.json").exists(),
        "nothing must be written when init gives up"
    );
}

#[test]
fn an_unrecognised_monorepo_answer_falls_back_to_single_package() {
    let dir = tempfile::tempdir().unwrap();
    run_init(
        dir.path(),
        &lines(&["maybe", "solo"]),
        Some(ConfigFileFormat::Json),
        false,
    )
    .unwrap();

    let config = load(dir.path(), "ferrflow.json");
    assert_eq!(config.packages.len(), 1);
    assert_eq!(config.packages[0].name, "solo");
    assert_eq!(config.packages[0].path, ".");
}

#[test]
fn the_manifest_flag_writes_the_manifest_and_points_the_config_at_it() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"1.2.3\"\n",
    )
    .unwrap();

    run_init(
        dir.path(),
        &lines(&["n", "app"]),
        Some(ConfigFileFormat::Json),
        true,
    )
    .unwrap();

    let config = load(dir.path(), "ferrflow.json");
    assert_eq!(
        config.workspace.manifest_file.as_deref(),
        Some(DEFAULT_MANIFEST_FILE)
    );
    let manifest = std::fs::read_to_string(dir.path().join(DEFAULT_MANIFEST_FILE)).unwrap();
    assert!(
        manifest.contains("1.2.3"),
        "the manifest should snapshot the current version: {manifest}"
    );
}

#[test]
fn an_existing_config_of_any_kind_is_never_overwritten() {
    for existing in [
        "ferrflow.json",
        "ferrflow.json5",
        "ferrflow.toml",
        ".ferrflow",
        TS_CONFIG_FILENAME,
        JS_CONFIG_FILENAME,
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(existing), "original").unwrap();

        let err = run_init(dir.path(), "", Some(ConfigFileFormat::Json), false).unwrap_err();

        assert_eq!(
            crate::error_code::code_from_error(&err).as_deref(),
            Some("E1017"),
            "{existing}: {err:#}"
        );
        assert!(format!("{err:#}").contains(existing), "{err:#}");
        assert_eq!(
            std::fs::read_to_string(dir.path().join(existing)).unwrap(),
            "original"
        );
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            1,
            "{existing} must stop init before it writes anything"
        );
    }
}
