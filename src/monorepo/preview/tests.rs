use super::*;

#[test]
fn escape_md_cell_passthrough_for_normal_text() {
    assert_eq!(escape_md_cell("api"), "api");
    assert_eq!(escape_md_cell("1.2.3"), "1.2.3");
    assert_eq!(escape_md_cell("minor"), "minor");
}

#[test]
fn escape_md_cell_escapes_pipe_breaking_table() {
    assert_eq!(escape_md_cell("a|b"), r"a\|b");
}

#[test]
fn escape_md_cell_squashes_newlines_to_keep_row() {
    assert_eq!(escape_md_cell("line1\nline2"), "line1 line2");
    assert_eq!(escape_md_cell("a\rb"), "a b");
}

#[test]
fn escape_md_cell_neutralizes_html_tags() {
    assert_eq!(
        escape_md_cell("<script>alert(1)</script>"),
        "&lt;script&gt;alert(1)&lt;/script&gt;"
    );
}

#[test]
fn escape_md_cell_escapes_backticks_and_backslash() {
    assert_eq!(escape_md_cell(r"foo`code`bar"), r"foo\`code\`bar");
    assert_eq!(escape_md_cell(r"path\to\thing"), r"path\\to\\thing");
}

#[test]
fn escape_md_cell_blocks_link_injection() {
    assert_eq!(
        escape_md_cell("foo](javascript:alert(1))"),
        "foo](javascript:alert(1))"
    );
    assert_eq!(escape_md_cell("|<img src=x>"), r"\|&lt;img src=x&gt;");
}

#[test]
fn missing_token_names_every_variable_that_was_read() {
    assert_eq!(
        ForgeUnavailable::NoToken(ForgeKind::Gitlab).to_string(),
        "no Gitlab token found in FERRFLOW_TOKEN or GITLAB_TOKEN"
    );
    assert_eq!(
        ForgeUnavailable::NoToken(ForgeKind::Gitea).to_string(),
        "no Gitea token found in FERRFLOW_TOKEN or GITEA_TOKEN or FORGEJO_TOKEN"
    );
}

#[test]
fn forge_target_detects_known_hosts_without_a_token() {
    let target = forge_target("git@gitlab.com:group/app.git", ForgeKind::Auto).unwrap();
    assert_eq!(target.kind, ForgeKind::Gitlab);
    assert_eq!(target.slug, "group/app");
    assert_eq!(target.host, "gitlab.com");
}

#[test]
fn forge_target_honours_an_explicit_forge() {
    let target = forge_target("https://git.example.com/team/app.git", ForgeKind::Gitea).unwrap();
    assert_eq!(target.kind, ForgeKind::Gitea);
    assert_eq!(target.host, "git.example.com");
}

#[test]
fn forge_target_reports_an_unparseable_remote() {
    assert_eq!(
        forge_target("not a remote", ForgeKind::Github).err(),
        Some(ForgeUnavailable::UnknownForge { host: None })
    );
}

#[test]
fn unknown_forge_message_never_carries_remote_credentials() {
    let reason = forge_target(
        "https://oauth2:glpat-secret@git.example.com",
        ForgeKind::Gitea,
    )
    .err()
    .expect("a remote without a path has no slug");
    let rendered = format!("{reason} {reason:?}");
    assert!(rendered.contains("git.example.com"));
    assert!(!rendered.contains("glpat-secret"));
    assert!(!rendered.contains("oauth2"));
}

fn package(json: serde_json::Value) -> CheckPackage {
    serde_json::from_value(json).expect("check --json package shape")
}

fn api() -> CheckPackage {
    package(serde_json::json!({
        "name": "api",
        "current_version": "1.2.0",
        "next_version": "1.3.0",
        "bump_type": "minor",
        "tag": "api@v1.3.0",
        "prerelease": false,
        "commits": [
            { "hash": "a1", "message": "feat: one" },
            { "hash": "a2", "message": "fix: two" }
        ]
    }))
}

#[test]
fn the_comment_starts_with_the_marker_used_to_find_it_again() {
    for packages in [Vec::new(), vec![api()]] {
        let body = format_preview_comment(&packages);
        assert!(
            body.starts_with("<!-- ferrflow-preview -->\n"),
            "the marker must lead so find_comment can match it: {body}"
        );
    }
}

#[test]
fn no_packages_means_no_table() {
    let body = format_preview_comment(&[]);
    assert!(body.ends_with("No releasable changes detected."), "{body}");
    assert!(!body.contains("| Package |"), "{body}");
}

#[test]
fn each_package_gets_a_row_and_commits_are_summed_across_packages() {
    let web = package(serde_json::json!({
        "name": "web",
        "current_version": "0.1.0",
        "next_version": "0.1.1",
        "bump_type": "patch",
        "tag": "web@v0.1.1",
        "prerelease": false,
        "commits": [{ "hash": "b1", "message": "fix: web" }]
    }));
    let body = format_preview_comment(&[api(), web]);

    let rows: Vec<&str> = body.lines().filter(|l| l.starts_with("| ")).collect();
    assert_eq!(
        rows,
        vec![
            "| Package | Current | Next | Bump |",
            "| api | `1.2.0` | `1.3.0` | minor |",
            "| web | `0.1.0` | `0.1.1` | patch |",
        ]
    );
    assert!(body.ends_with("Based on 3 commit(s)."), "{body}");
}

#[test]
fn hostile_package_fields_cannot_break_the_table() {
    let mut evil = api();
    evil.name = "a|b\n<img>".to_string();
    evil.next_version = "1.3.0`".to_string();
    let body = format_preview_comment(&[evil]);
    let row = body
        .lines()
        .find(|l| l.starts_with("| a"))
        .unwrap_or_else(|| panic!("{body}"));
    assert_eq!(row, r"| a\|b &lt;img&gt; | `1.2.0` | `1.3.0\`` | minor |");
}

mod forge_from_repo {
    use super::*;
    use crate::config::Config;
    use crate::test_utils::{git, init_repo};

    fn config() -> Config {
        Config {
            include: Vec::new(),
            workspace: crate::config::WorkspaceConfig::default(),
            packages: Vec::new(),
        }
    }

    #[test]
    fn a_missing_remote_is_named_in_the_reason() {
        let (_dir, repo) = init_repo();
        let config = config();
        let reason = try_build_forge_instance(&repo, &config).err().unwrap();
        assert_eq!(
            reason,
            ForgeUnavailable::NoRemote(config.workspace.remote.clone())
        );
        assert_eq!(
            reason.to_string(),
            format!("remote `{}` not found", config.workspace.remote)
        );
        assert!(build_forge_instance(&repo, &config).is_none());
    }

    #[test]
    fn an_unparseable_remote_url_is_reported_as_an_unknown_forge() {
        let (dir, _repo) = init_repo();
        let config = config();
        git(
            dir.path(),
            &["remote", "add", &config.workspace.remote, "not a remote"],
        );
        let repo = crate::git::open_repo(dir.path()).unwrap();
        let reason = try_build_forge_instance(&repo, &config).err().unwrap();
        assert_eq!(reason, ForgeUnavailable::UnknownForge { host: None });
        assert_eq!(reason.to_string(), "could not parse the remote URL");
    }

    #[test]
    fn an_unrecognised_host_asks_for_the_forge_setting() {
        assert_eq!(
            ForgeUnavailable::UnknownForge {
                host: Some("git.example.com".to_string())
            }
            .to_string(),
            "could not tell which forge hosts git.example.com, set `forge` in the config"
        );
    }
}
