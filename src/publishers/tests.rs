use std::collections::BTreeMap;
use std::path::Path;

use super::{PublishContext, PublishOutcome, run, run_all};
use crate::config::{PublisherConfig, RegistryConfig};

fn publishers(json: &str) -> Vec<PublisherConfig> {
    serde_json::from_str(json).unwrap()
}

fn ctx<'a>(
    registries: &'a BTreeMap<String, RegistryConfig>,
    package_path: &'a Path,
    dry_run: bool,
) -> PublishContext<'a> {
    PublishContext {
        package_name: "sdk",
        package_path,
        new_version: "1.2.3",
        tag: "sdk@v1.2.3",
        registries,
        dry_run,
        verbose: false,
    }
}

fn chart_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Chart.yaml"),
        "apiVersion: v2\nname: sdk\nversion: 1.2.3\n",
    )
    .unwrap();
    dir
}

#[test]
fn a_dry_run_reports_nothing_as_published() {
    let dir = chart_dir();
    let registries = BTreeMap::new();
    let list = publishers(
        r#"[
            {"kind":"cargo"},
            {"kind":"pypi"},
            {"kind":"docker","image":"ghcr.io/x/sdk"},
            {"kind":"helm","registry":"oci://ghcr.io/x/charts"}
        ]"#,
    );
    let mut published = Vec::new();

    run_all(&list, &ctx(&registries, dir.path(), true), &mut published)
        .expect("every publisher dry-runs");

    assert!(
        published.is_empty(),
        "rollback must never be told a dry run published anything: {published:?}"
    );
}

#[test]
fn the_first_failing_publisher_stops_the_rest() {
    let dir = chart_dir();
    let registries = BTreeMap::new();
    let list = publishers(
        r#"[
            {"kind":"cargo"},
            {"kind":"pypi","registry":"missing-pypi"},
            {"kind":"cargo","registry":"missing-cargo"}
        ]"#,
    );
    let mut published = Vec::new();

    let err = run_all(&list, &ctx(&registries, dir.path(), true), &mut published)
        .expect_err("an undeclared registry fails the batch");

    let rendered = format!("{err:#}");
    assert!(rendered.contains("missing-pypi"), "{rendered}");
    assert!(
        !rendered.contains("missing-cargo"),
        "the publisher after the failure must not run: {rendered}"
    );
    assert!(published.is_empty());
}

#[test]
fn no_publishers_is_a_no_op() {
    let registries = BTreeMap::new();
    let mut published = Vec::new();

    run_all(
        &[],
        &ctx(&registries, Path::new("."), false),
        &mut published,
    )
    .unwrap();

    assert!(published.is_empty());
}

#[test]
fn dispatch_forwards_the_trusted_publishing_flag() {
    let mut registries = BTreeMap::new();
    registries.insert(
        "internal".to_string(),
        RegistryConfig {
            url: None,
            token_env: Some("PATH".to_string()),
        },
    );
    let [trusted, token] = <[PublisherConfig; 2]>::try_from(publishers(
        r#"[
            {"kind":"pypi","registry":"internal","trustedPublishing":true},
            {"kind":"pypi","registry":"internal"}
        ]"#,
    ))
    .unwrap();
    let c = ctx(&registries, Path::new("."), true);

    let err = run(&trusted, &c).expect_err("both auth sources configured");
    assert!(
        format!("{err:#}").contains("both configure authentication"),
        "{err:#}"
    );
    assert!(matches!(run(&token, &c).unwrap(), PublishOutcome::DryRun));
}

#[test]
fn dispatch_resolves_the_helm_chart_under_the_package_path() {
    let dir = tempfile::tempdir().unwrap();
    let registries = BTreeMap::new();
    let [helm] = <[PublisherConfig; 1]>::try_from(publishers(
        r#"[{"kind":"helm","chart":"deploy/chart","registry":"oci://ghcr.io/x"}]"#,
    ))
    .unwrap();

    let err = run(&helm, &ctx(&registries, dir.path(), true))
        .expect_err("the chart directory is missing");

    let rendered = format!("{err:#}");
    assert!(rendered.contains("does not exist"), "{rendered}");
    assert!(rendered.contains("deploy"), "{rendered}");
}
