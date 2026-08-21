use std::hint::black_box;
use std::io::Write;

use criterion::{Criterion, criterion_group, criterion_main};
use ferrflow::changelog::{ChangelogRender, build_section_with, update_changelog};
use ferrflow::config::{Config, FileFormat, OrphanedTagStrategy};
use ferrflow::conventional_commits::{BumpType, determine_bump};
use ferrflow::formats::get_handler;
use ferrflow::git::{
    GitLog, collect_all_tags, find_last_tag_name, get_changed_files, get_changed_files_since_tag,
    get_commits_since_last_tag,
};
use ferrflow::versioning::compute_next_version;
use tempfile::{NamedTempFile, TempDir};

fn generate_commit_messages(count: usize) -> Vec<String> {
    let types = [
        "feat", "fix", "refactor", "perf", "chore", "docs", "ci", "test",
    ];
    let scopes = ["api", "auth", "db", "cache", "config"];
    let mut messages = Vec::with_capacity(count);
    for i in 0..count {
        let t = types[i % types.len()];
        let s = scopes[i % scopes.len()];
        let breaking = if i % 20 == 0 { "!" } else { "" };
        messages.push(format!("{t}({s}){breaking}: change number {i}"));
    }
    messages
}

fn bench_commit_parsing(c: &mut Criterion) {
    for size in [100, 1_000, 10_000] {
        let messages = generate_commit_messages(size);
        c.bench_function(&format!("commit_parsing/{size}"), |b| {
            b.iter(|| {
                for msg in &messages {
                    black_box(determine_bump(msg, &Default::default()));
                }
            });
        });
    }
}

fn generate_commits(count: usize) -> Vec<GitLog> {
    let types = [
        "feat", "fix", "refactor", "perf", "chore", "docs", "ci", "test",
    ];
    let scopes = ["api", "auth", "db", "cache", "config"];
    let mut commits = Vec::with_capacity(count);
    for i in 0..count {
        let t = types[i % types.len()];
        let s = scopes[i % scopes.len()];
        let breaking = if i % 20 == 0 { "!" } else { "" };
        commits.push(GitLog {
            hash: format!("{i:08x}"),
            message: format!("{t}({s}){breaking}: change number {i}"),
        });
    }
    commits
}

fn bench_changelog(c: &mut Criterion) {
    for size in [50, 500] {
        let commits = generate_commits(size);

        c.bench_function(&format!("changelog/build_{size}"), |b| {
            b.iter(|| {
                black_box(build_section_with(
                    "1.0.0",
                    &commits,
                    &ChangelogRender::default(),
                ));
            });
        });

        c.bench_function(&format!("changelog/update_{size}"), |b| {
            b.iter(|| {
                let mut f = NamedTempFile::new().unwrap();
                f.write_all(b"# Changelog\n\n## v0.9.0\n\n- old entry\n")
                    .unwrap();
                let path = f.path().to_path_buf();
                update_changelog(
                    black_box(&path),
                    "myapp",
                    "1.0.0",
                    &commits,
                    BumpType::Minor,
                    false,
                )
                .unwrap();
            });
        });
    }
}

fn bench_version_files(c: &mut Criterion) {
    let cases: Vec<(&str, FileFormat, &str)> = vec![
        (
            "toml",
            FileFormat::Toml,
            "[package]\nname = \"foo\"\nversion = \"1.2.3\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
        ),
        (
            "json",
            FileFormat::Json,
            r#"{"name":"foo","version":"1.2.3","description":"a package","main":"index.js"}"#,
        ),
        (
            "xml",
            FileFormat::Xml,
            "<project>\n  <modelVersion>4.0.0</modelVersion>\n  <groupId>com.example</groupId>\n  <artifactId>foo</artifactId>\n  <version>1.2.3</version>\n</project>\n",
        ),
        (
            "gradle",
            FileFormat::Gradle,
            "plugins { id 'java' }\nversion = \"1.2.3\"\ngroup = 'com.example'\n",
        ),
    ];

    for (name, format, content) in &cases {
        let handler = get_handler(format);

        c.bench_function(&format!("version_files/{name}_read"), |b| {
            let mut f = NamedTempFile::new().unwrap();
            f.write_all(content.as_bytes()).unwrap();
            let path = f.path().to_path_buf();
            b.iter(|| {
                black_box(handler.read_version(&path).unwrap());
            });
        });

        c.bench_function(&format!("version_files/{name}_write"), |b| {
            let mut f = NamedTempFile::new().unwrap();
            f.write_all(content.as_bytes()).unwrap();
            let path = f.path().to_path_buf();
            b.iter(|| {
                handler.write_version(black_box(&path), "2.0.0").unwrap();
            });
        });
    }
}

fn generate_config_json(num_packages: usize) -> String {
    let mut packages = Vec::new();
    for i in 1..=num_packages {
        packages.push(format!(
            r#"    {{
      "name": "pkg-{i:03}",
      "path": "packages/pkg-{i:03}",
      "changelog": "packages/pkg-{i:03}/CHANGELOG.md",
      "versioned_files": [
        {{ "path": "packages/pkg-{i:03}/package.json", "format": "json" }}
      ]
    }}"#
        ));
    }
    format!("{{\n  \"package\": [\n{}\n  ]\n}}", packages.join(",\n"))
}

fn bench_config_loading(c: &mut Criterion) {
    for (label, num_pkgs) in [
        ("single", 1),
        ("mono_10", 10),
        ("mono_50", 50),
        ("mono_100", 100),
    ] {
        c.bench_function(&format!("config_loading/{label}"), |b| {
            let dir = TempDir::new().unwrap();
            let config_path = dir.path().join(".ferrflow");
            std::fs::write(&config_path, generate_config_json(num_pkgs)).unwrap();
            std::process::Command::new("git")
                .args(["init", "-q"])
                .current_dir(dir.path())
                .output()
                .unwrap();
            b.iter(|| {
                black_box(Config::load(dir.path(), None).unwrap());
            });
        });
    }
}

fn run_git(dir: &std::path::Path, args: &[&str]) -> String {
    run_git_with_stdin(dir, args, None)
}

fn run_git_with_stdin(dir: &std::path::Path, args: &[&str], stdin: Option<&[u8]>) -> String {
    use std::io::Write;
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(dir).args(args);
    cmd.env("GIT_AUTHOR_NAME", "bench");
    cmd.env("GIT_AUTHOR_EMAIL", "bench@test.com");
    cmd.env("GIT_AUTHOR_DATE", "1700000000 +0000");
    cmd.env("GIT_COMMITTER_NAME", "bench");
    cmd.env("GIT_COMMITTER_EMAIL", "bench@test.com");
    cmd.env("GIT_COMMITTER_DATE", "1700000000 +0000");
    if stdin.is_some() {
        cmd.stdin(std::process::Stdio::piped());
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("git {} failed to spawn: {}", args.join(" "), e));
    if let Some(data) = stdin {
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(data)
            .expect("write stdin");
    }
    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("git {} wait failed: {}", args.join(" "), e));
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        panic!("git {} failed: {}{}", args.join(" "), stdout, stderr);
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn create_bench_repo(num_commits: usize, tag_at: usize) -> (TempDir, gix::Repository) {
    let dir = TempDir::new().unwrap();
    let path = dir.path();
    run_git(path, &["init", "-b", "main"]);
    run_git(path, &["config", "user.name", "bench"]);
    run_git(path, &["config", "user.email", "bench@test.com"]);
    run_git(path, &["config", "commit.gpgsign", "false"]);

    let types = ["feat", "fix", "refactor", "perf", "chore"];
    let scopes = ["api", "auth", "db"];

    let mut parent: Option<String> = None;
    for i in 0..num_commits {
        let t = types[i % types.len()];
        let s = scopes[i % scopes.len()];
        let breaking = if i % 20 == 0 && i > 0 { "!" } else { "" };
        let msg = format!("{t}({s}){breaking}: change {i}");

        let file_name = format!("src/file_{i}.rs");
        let content = format!("// commit {i}\n");
        let blob_sha = run_git_with_stdin(
            path,
            &["hash-object", "-w", "--stdin"],
            Some(content.as_bytes()),
        )
        .trim()
        .to_string();
        run_git(
            path,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("100644,{blob_sha},{file_name}"),
            ],
        );
        let tree_sha = run_git(path, &["write-tree"]).trim().to_string();
        let mut commit_args: Vec<String> = vec!["commit-tree".into(), tree_sha, "-m".into(), msg];
        if let Some(p) = &parent {
            commit_args.push("-p".into());
            commit_args.push(p.clone());
        }
        let arg_refs: Vec<&str> = commit_args.iter().map(String::as_str).collect();
        let commit_sha = run_git(path, &arg_refs).trim().to_string();
        run_git(path, &["update-ref", "refs/heads/main", &commit_sha]);
        parent = Some(commit_sha.clone());

        if i == tag_at {
            run_git(path, &["tag", "v1.0.0", &commit_sha]);
        }
    }

    let repo = gix::discover(path).unwrap();
    (dir, repo)
}

fn bench_git_operations(c: &mut Criterion) {
    for (label, total_commits, tag_position) in [
        ("git_commits/100", 100, 0),
        ("git_commits/1000", 1_000, 0),
        ("git_commits/5000", 5_000, 0),
    ] {
        let (_dir, repo) = create_bench_repo(total_commits, tag_position);
        c.bench_function(label, |b| {
            b.iter(|| {
                black_box(
                    get_commits_since_last_tag(
                        &repo,
                        "v",
                        OrphanedTagStrategy::Warn,
                        &ferrflow::config::default_commit_skip_markers(),
                        None,
                    )
                    .unwrap(),
                );
            });
        });
    }

    for (label, total_commits, tag_position) in [
        ("git_find_tag/100", 100, 50),
        ("git_find_tag/1000", 1_000, 500),
    ] {
        let (_dir, repo) = create_bench_repo(total_commits, tag_position);
        c.bench_function(label, |b| {
            b.iter(|| {
                black_box(find_last_tag_name(&repo, "v", OrphanedTagStrategy::Warn).unwrap());
            });
        });
    }

    {
        let (_dir, repo) = create_bench_repo(100, 50);
        c.bench_function("git_collect_tags/single_tag", |b| {
            b.iter(|| {
                black_box(collect_all_tags(&repo));
            });
        });
    }

    for (label, total_commits) in [
        ("git_changed_files/100", 100),
        ("git_changed_files/1000", 1_000),
    ] {
        let (_dir, repo) = create_bench_repo(total_commits, 0);
        c.bench_function(label, |b| {
            b.iter(|| {
                black_box(get_changed_files(&repo).unwrap());
            });
        });
    }

    for (label, total_commits, tag_position) in [
        ("git_changed_since_tag/100_commits_50_since", 100, 50),
        ("git_changed_since_tag/1000_commits_500_since", 1_000, 500),
    ] {
        let (_dir, repo) = create_bench_repo(total_commits, tag_position);
        c.bench_function(label, |b| {
            b.iter(|| {
                black_box(
                    get_changed_files_since_tag(&repo, "v", OrphanedTagStrategy::Warn, None)
                        .unwrap(),
                );
            });
        });
    }
}

fn bench_validate(c: &mut Criterion) {
    for (label, num_pkgs) in [
        ("validate/single", 1),
        ("validate/mono_50", 50),
        ("validate/mono_100", 100),
    ] {
        c.bench_function(label, |b| {
            let dir = TempDir::new().unwrap();
            let config_path = dir.path().join(".ferrflow");
            std::fs::write(&config_path, generate_config_json(num_pkgs)).unwrap();

            for i in 1..=num_pkgs {
                let pkg_dir = dir.path().join(format!("packages/pkg-{i:03}"));
                std::fs::create_dir_all(&pkg_dir).unwrap();
                std::fs::write(
                    pkg_dir.join("package.json"),
                    r#"{"name":"pkg","version":"1.0.0"}"#,
                )
                .unwrap();
            }

            std::process::Command::new("git")
                .args(["init", "-q"])
                .current_dir(dir.path())
                .output()
                .unwrap();

            b.iter(|| {
                let config = Config::load(dir.path(), None).unwrap();
                for pkg in &config.packages {
                    for vf in &pkg.versioned_files {
                        let handler = get_handler(&vf.format);
                        black_box(handler.read_version(&dir.path().join(&vf.path)).unwrap());
                    }
                }
            });
        });
    }
}

fn bench_full_check_flow(c: &mut Criterion) {
    for (label, num_commits) in [
        ("full_check_flow/100_commits", 100),
        ("full_check_flow/1000_commits", 1_000),
    ] {
        let (_dir, repo) = create_bench_repo(num_commits, 0);

        let config_content = generate_config_json(1);
        std::fs::write(_dir.path().join(".ferrflow"), &config_content).unwrap();

        std::fs::create_dir_all(_dir.path().join("packages/pkg-001")).unwrap();
        std::fs::write(
            _dir.path().join("packages/pkg-001/package.json"),
            r#"{"name":"pkg-001","version":"1.0.0"}"#,
        )
        .unwrap();

        c.bench_function(label, |b| {
            b.iter(|| {
                let config = Config::load(_dir.path(), None).unwrap();
                let commits = get_commits_since_last_tag(
                    &repo,
                    "v",
                    OrphanedTagStrategy::Warn,
                    &ferrflow::config::default_commit_skip_markers(),
                    None,
                )
                .unwrap();
                for commit in &commits {
                    black_box(determine_bump(
                        &commit.message,
                        &config.workspace.commit_formats,
                    ));
                }
                black_box((&config, commits.len()));
            });
        });
    }
}

fn create_monorepo_bench_repo(
    num_packages: usize,
    num_commits: usize,
) -> (TempDir, gix::Repository) {
    let dir = TempDir::new().unwrap();
    let path = dir.path();
    run_git(path, &["init", "-b", "main"]);
    run_git(path, &["config", "user.name", "bench"]);
    run_git(path, &["config", "user.email", "bench@test.com"]);
    run_git(path, &["config", "commit.gpgsign", "false"]);

    let types = ["feat", "fix", "refactor", "perf", "chore"];
    let mut parent: Option<String> = None;

    for i in 0..num_commits {
        let pkg = (i % num_packages) + 1;
        let t = types[i % types.len()];
        let breaking = if i % 50 == 0 && i > 0 { "!" } else { "" };
        let msg = format!("{t}(pkg-{pkg:03}){breaking}: change {i}");

        let file_name = format!("packages/pkg-{pkg:03}/src/file_{i}.rs");
        let content = format!(
            "// commit {i}
"
        );
        let blob_sha = run_git_with_stdin(
            path,
            &["hash-object", "-w", "--stdin"],
            Some(content.as_bytes()),
        )
        .trim()
        .to_string();
        run_git(
            path,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("100644,{blob_sha},{file_name}"),
            ],
        );
        let tree_sha = run_git(path, &["write-tree"]).trim().to_string();
        let mut commit_args: Vec<String> = vec!["commit-tree".into(), tree_sha, "-m".into(), msg];
        if let Some(p) = &parent {
            commit_args.push("-p".into());
            commit_args.push(p.clone());
        }
        let arg_refs: Vec<&str> = commit_args.iter().map(String::as_str).collect();
        let commit_sha = run_git(path, &arg_refs).trim().to_string();
        run_git(path, &["update-ref", "refs/heads/main", &commit_sha]);
        parent = Some(commit_sha.clone());

        if i == 0 {
            run_git(path, &["tag", "v1.0.0", &commit_sha]);
        }
    }

    std::fs::write(path.join(".ferrflow"), generate_config_json(num_packages)).unwrap();
    for i in 1..=num_packages {
        let pkg_dir = path.join(format!("packages/pkg-{i:03}"));
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("package.json"),
            format!(r#"{{"name":"pkg-{i:03}","version":"1.0.0"}}"#),
        )
        .unwrap();
    }

    let repo = gix::discover(path).unwrap();
    (dir, repo)
}

fn bench_full_monorepo_flow(c: &mut Criterion) {
    for (label, num_packages, num_commits) in [
        ("full_monorepo_flow/10_packages", 10, 500),
        ("full_monorepo_flow/50_packages", 50, 500),
    ] {
        let (dir, repo) = create_monorepo_bench_repo(num_packages, num_commits);

        c.bench_function(label, |b| {
            b.iter(|| {
                let config = Config::load(dir.path(), None).unwrap();
                let changed =
                    get_changed_files_since_tag(&repo, "v", OrphanedTagStrategy::Warn, None)
                        .unwrap();
                let commits = get_commits_since_last_tag(
                    &repo,
                    "v",
                    OrphanedTagStrategy::Warn,
                    &ferrflow::config::default_commit_skip_markers(),
                    None,
                )
                .unwrap();

                let bump = commits
                    .iter()
                    .map(|commit| determine_bump(&commit.message, &config.workspace.commit_formats))
                    .max()
                    .unwrap_or(BumpType::None);

                let mut planned = 0usize;
                for pkg in &config.packages {
                    if !pkg.is_touched_by(&changed, true) {
                        continue;
                    }
                    let strategy = pkg.effective_versioning(&config.workspace, Vec::new);
                    let next = compute_next_version("1.0.0", bump, strategy, None).unwrap();
                    black_box(&next);
                    planned += 1;
                }
                black_box(planned);
            });
        });
    }
}

criterion_group!(
    benches,
    bench_commit_parsing,
    bench_changelog,
    bench_version_files,
    bench_config_loading,
    bench_git_operations,
    bench_validate,
    bench_full_check_flow,
    bench_full_monorepo_flow
);
criterion_main!(benches);
