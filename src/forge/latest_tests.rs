use super::LatestRelease::{self, Auto, Never};
use crate::config::{Config, PackageConfig, WorkspaceConfig};

fn make_pkg(name: &str) -> PackageConfig {
    PackageConfig {
        name: name.into(),
        path: ".".into(),
        ..Default::default()
    }
}

fn pkg_with_latest(name: &str, latest: Option<&str>) -> PackageConfig {
    PackageConfig {
        latest_tag: latest.map(String::from),
        ..make_pkg(name)
    }
}

fn config_of(workspace_latest: Option<&str>, packages: Vec<PackageConfig>) -> Config {
    Config {
        include: Vec::new(),
        workspace: WorkspaceConfig {
            latest_tag: workspace_latest.map(String::from),
            ..WorkspaceConfig::default()
        },
        packages,
    }
}

#[test]
fn only_the_package_owning_the_unnamespaced_latest_tag_may_be_the_latest_release() {
    let config = config_of(
        None,
        vec![
            pkg_with_latest("ferrflow", Some("latest")),
            pkg_with_latest("site", Some("{name}@latest")),
            pkg_with_latest("api", Some("{name}@latest")),
        ],
    );
    assert_eq!(LatestRelease::of(&config, "ferrflow"), Auto);
    assert_eq!(LatestRelease::of(&config, "site"), Never);
    assert_eq!(LatestRelease::of(&config, "api"), Never);
}

#[test]
fn a_package_without_a_latest_tag_never_takes_latest_from_the_owner() {
    let config = config_of(
        None,
        vec![
            pkg_with_latest("ferrflow", Some("latest")),
            pkg_with_latest("docs", None),
        ],
    );
    assert_eq!(LatestRelease::of(&config, "ferrflow"), Auto);
    assert_eq!(LatestRelease::of(&config, "docs"), Never);
}

#[test]
fn a_workspace_latest_tag_is_inherited_unless_the_package_namespaces_it() {
    let config = config_of(
        Some("latest"),
        vec![
            pkg_with_latest("ferrflow", None),
            pkg_with_latest("site", Some("{name}@latest")),
        ],
    );
    assert_eq!(LatestRelease::of(&config, "ferrflow"), Auto);
    assert_eq!(LatestRelease::of(&config, "site"), Never);
}

#[test]
fn a_monorepo_with_no_owner_of_the_latest_tag_keeps_the_forge_default() {
    let unconfigured = config_of(None, vec![make_pkg("a"), make_pkg("b")]);
    assert_eq!(LatestRelease::of(&unconfigured, "a"), Auto);
    assert_eq!(LatestRelease::of(&unconfigured, "b"), Auto);

    let all_namespaced = config_of(Some("{name}@latest"), vec![make_pkg("a"), make_pkg("b")]);
    assert_eq!(LatestRelease::of(&all_namespaced, "a"), Auto);
    assert_eq!(LatestRelease::of(&all_namespaced, "b"), Auto);
}

#[test]
fn a_single_package_repo_keeps_the_forge_default_whatever_its_latest_tag() {
    for latest in [None, Some("latest"), Some("{name}@latest")] {
        let config = config_of(None, vec![pkg_with_latest("app", latest)]);
        assert_eq!(
            LatestRelease::of(&config, "app"),
            Auto,
            "latest tag {latest:?}"
        );
    }
}

#[test]
fn an_unknown_package_keeps_the_forge_default() {
    let config = config_of(
        None,
        vec![
            pkg_with_latest("ferrflow", Some("latest")),
            pkg_with_latest("site", None),
        ],
    );
    assert_eq!(LatestRelease::of(&config, "ghost"), Auto);
}
