use super::*;

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn pkg(name: &str) -> String {
    format!("{{\"name\": \"{name}\", \"version\": \"1.0.0\"}}")
}

fn cargo_package(name: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n")
}

fn names_and_paths(found: &[DiscoveredPackage]) -> Vec<(&str, &str)> {
    found
        .iter()
        .map(|p| (p.name.as_str(), p.path.as_str()))
        .collect()
}

#[test]
fn npm_workspaces_array_is_expanded() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "package.json", r#"{"workspaces": ["packages/*"]}"#);
    write(root, "packages/a/package.json", &pkg("@acme/a"));
    write(root, "packages/b/package.json", &pkg("@acme/b"));

    let found = discover(root);
    assert_eq!(
        found,
        vec![
            DiscoveredPackage {
                name: "@acme/a".into(),
                path: "packages/a".into(),
                manifest: "package.json",
                format: FileFormat::Json,
            },
            DiscoveredPackage {
                name: "@acme/b".into(),
                path: "packages/b".into(),
                manifest: "package.json",
                format: FileFormat::Json,
            },
        ]
    );
}

#[test]
fn yarn_berry_object_form_is_expanded() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "package.json",
        r#"{"workspaces": {"packages": ["libs/*"]}}"#,
    );
    write(root, "libs/one/package.json", &pkg("one"));

    assert_eq!(names_and_paths(&discover(root)), [("one", "libs/one")]);
}

#[test]
fn pnpm_workspace_globs_are_read() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "package.json", r#"{"name": "root"}"#);
    write(root, "pnpm-workspace.yaml", "packages:\n  - 'apps/*'\n");
    write(root, "apps/web/package.json", &pkg("@acme/web"));

    assert_eq!(
        names_and_paths(&discover(root)),
        [("@acme/web", "apps/web")]
    );
}

#[test]
fn node_modules_is_never_walked() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "package.json", r#"{"workspaces": ["packages/*"]}"#);
    write(root, "packages/a/package.json", &pkg("@acme/a"));
    write(
        root,
        "packages/a/node_modules/left-pad/package.json",
        &pkg("left-pad"),
    );
    write(root, "node_modules/react/package.json", &pkg("react"));

    assert_eq!(
        names_and_paths(&discover(root)),
        [("@acme/a", "packages/a")]
    );
}

#[test]
fn nested_globs_are_matched() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "package.json", r#"{"workspaces": ["apps/*/pkg/*"]}"#);
    write(root, "apps/web/pkg/ui/package.json", &pkg("ui"));

    assert_eq!(
        names_and_paths(&discover(root)),
        [("ui", "apps/web/pkg/ui")]
    );
}

#[test]
fn a_directory_without_a_manifest_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "package.json", r#"{"workspaces": ["packages/*"]}"#);
    std::fs::create_dir_all(root.join("packages/empty")).unwrap();
    write(root, "packages/real/package.json", &pkg("real"));

    assert_eq!(
        names_and_paths(&discover(root)),
        [("real", "packages/real")]
    );
}

#[test]
fn a_nameless_manifest_falls_back_to_its_directory() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "package.json", r#"{"workspaces": ["packages/*"]}"#);
    write(root, "packages/tools/package.json", r#"{"private": true}"#);

    assert_eq!(
        names_and_paths(&discover(root)),
        [("tools", "packages/tools")]
    );
}

#[test]
fn no_workspace_declaration_discovers_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "package.json", r#"{"name": "solo"}"#);
    write(root, "packages/a/package.json", &pkg("@acme/a"));

    assert!(discover(root).is_empty());
}

#[test]
fn negated_globs_exclude_their_matches() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "package.json",
        r#"{"workspaces": ["packages/*", "!packages/internal"]}"#,
    );
    write(root, "packages/keep/package.json", &pkg("keep"));
    write(root, "packages/internal/package.json", &pkg("internal"));

    assert_eq!(
        names_and_paths(&discover(root)),
        [("keep", "packages/keep")]
    );
}

#[test]
fn cargo_workspace_members_are_expanded_with_their_package_names() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\n",
    );
    write(root, "crates/api/Cargo.toml", &cargo_package("acme-api"));
    write(root, "crates/cli/Cargo.toml", &cargo_package("acme-cli"));

    let found = discover(root);
    assert_eq!(
        names_and_paths(&found),
        [("acme-api", "crates/api"), ("acme-cli", "crates/cli")]
    );
    assert!(
        found
            .iter()
            .all(|p| p.manifest == "Cargo.toml" && p.format == FileFormat::Toml)
    );
}

#[test]
fn cargo_workspace_exclude_drops_members() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/scratch\"]\n",
    );
    write(root, "crates/keep/Cargo.toml", &cargo_package("keep"));
    write(root, "crates/scratch/Cargo.toml", &cargo_package("scratch"));

    assert_eq!(names_and_paths(&discover(root)), [("keep", "crates/keep")]);
}

#[test]
fn cargo_members_inheriting_the_workspace_version_collapse_to_the_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\n[workspace.package]\nversion = \"2.0.0\"\n",
    );
    write(
        root,
        "crates/a/Cargo.toml",
        "[package]\nname = \"a\"\nversion.workspace = true\n",
    );
    write(
        root,
        "crates/b/Cargo.toml",
        "[package]\nname = \"b\"\nversion.workspace = true\n",
    );

    let found = discover(root);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, ".");
    assert_eq!(found[0].manifest, "Cargo.toml");
}

#[test]
fn cargo_root_package_listed_as_a_member_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[package]\nname = \"tool\"\nversion = \"1.0.0\"\n[workspace]\nmembers = [\".\", \"helper\"]\n",
    );
    write(root, "helper/Cargo.toml", &cargo_package("helper"));

    assert_eq!(
        names_and_paths(&discover(root)),
        [("tool", "."), ("helper", "helper")]
    );
}

#[test]
fn a_cargo_manifest_without_a_workspace_table_discovers_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "Cargo.toml", &cargo_package("solo"));
    write(root, "crates/a/Cargo.toml", &cargo_package("a"));

    assert!(discover(root).is_empty());
}

#[test]
fn go_work_use_lines_and_blocks_are_expanded() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "go.work",
        "go 1.22\n\nuse ./api\n\nuse (\n\t./worker // background jobs\n\t./libs/shared\n)\n",
    );
    write(
        root,
        "api/go.mod",
        "module example.com/acme/api\n\ngo 1.22\n",
    );
    write(root, "worker/go.mod", "module example.com/acme/worker\n");
    write(
        root,
        "libs/shared/go.mod",
        "module example.com/acme/shared\n",
    );

    let found = discover(root);
    assert_eq!(
        names_and_paths(&found),
        [
            ("example.com/acme/api", "api"),
            ("example.com/acme/shared", "libs/shared"),
            ("example.com/acme/worker", "worker"),
        ]
    );
    assert!(
        found
            .iter()
            .all(|p| p.manifest == "go.mod" && p.format == FileFormat::GoMod)
    );
}

#[test]
fn gradle_includes_map_colons_to_directories() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "settings.gradle.kts",
        "rootProject.name = \"acme\"\ninclude(\":app\", \":libs:core\")\nincludeBuild(\"../other\")\n",
    );
    write(root, "app/build.gradle.kts", "");
    write(root, "libs/core/build.gradle", "");

    let found = discover(root);
    assert_eq!(
        names_and_paths(&found),
        [("app", "app"), ("core", "libs/core")]
    );
    assert_eq!(found[0].manifest, "build.gradle.kts");
    assert_eq!(found[1].manifest, "build.gradle");
    assert!(found.iter().all(|p| p.format == FileFormat::Gradle));
}

#[test]
fn gradle_groovy_includes_without_parentheses_are_read() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "settings.gradle", "include 'server', ':client'\n");
    write(root, "server/build.gradle", "");
    write(root, "client/build.gradle", "");

    assert_eq!(
        names_and_paths(&discover(root)),
        [("client", "client"), ("server", "server")]
    );
}

#[test]
fn a_gradle_include_without_a_build_file_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "settings.gradle", "include 'ghost', 'real'\n");
    std::fs::create_dir_all(root.join("ghost")).unwrap();
    write(root, "real/build.gradle", "");

    assert_eq!(names_and_paths(&discover(root)), [("real", "real")]);
}

#[test]
fn a_path_declared_by_two_ecosystems_is_listed_once() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "package.json", r#"{"workspaces": ["crates/*"]}"#);
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\n",
    );
    write(root, "crates/bridge/package.json", &pkg("bridge-js"));
    write(root, "crates/bridge/Cargo.toml", &cargo_package("bridge"));

    let found = discover(root);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "bridge-js");
}

#[test]
fn gradle_includes_spanning_several_lines_are_read() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "settings.gradle.kts",
        "include(\n    \":a\",\n    \":b:c\"\n)\n",
    );
    write(root, "a/build.gradle.kts", "");
    write(root, "b/c/build.gradle.kts", "");

    assert_eq!(names_and_paths(&discover(root)), [("a", "a"), ("c", "b/c")]);
}

#[test]
fn discover_node_ignores_other_ecosystems() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\n",
    );
    write(root, "crates/api/Cargo.toml", &cargo_package("api"));
    write(root, "package.json", r#"{"workspaces": ["packages/*"]}"#);
    write(root, "packages/web/package.json", &pkg("web"));

    assert_eq!(
        names_and_paths(&discover_node(root)),
        [("web", "packages/web")]
    );
    assert_eq!(discover(root).len(), 2);
}
