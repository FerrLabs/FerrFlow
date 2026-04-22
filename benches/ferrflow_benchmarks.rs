use std::io::Write;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use ferrflow::changelog::{build_section, update_changelog};
use ferrflow::config::{Config, FileFormat, OrphanedTagStrategy};
use ferrflow::conventional_commits::{BumpType, determine_bump};
use ferrflow::formats::get_handler;
use ferrflow::git::{
    GitLog, collect_all_tags, find_last_tag_name, get_changed_files, get_changed_files_since_tag,
    get_commits_since_last_tag,
};
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
                    black_box(determine_bump(msg));
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
                black_box(build_section("1.0.0", &commits));
            });
        });

        c.bench_function(&format!("changelog/update_{size}"), |b| {
            b.iter(|| {
                let mut f = NamedTempFile::new().unwrap();
                f.write_all(b"# Changelog\n\n## v0.9.0\n\n- old entry\n")
                    .unwrap();
                let path = f.path().to_path_buf();
                black_box(
                    update_changelog(&path, "myapp", "1.0.0", &commits, BumpType::Minor, false)
                        .unwrap(),
                );
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
                black_box(handler.write_version(&path, "2.0.0").unwrap());
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

/// Create a git repo with `num_commits` commits and a tag at `tag_at` position.
/// Returns the TempDir (must be kept alive) and the opened Repository.
fn create_bench_repo(num_commits: usize, tag_at: usize) -> (TempDir, git2::Repository) {
    let dir = TempDir::new().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();

    // Configure committer identity
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "bench").unwrap();
    config.set_str("user.email", "bench@test.com").unwrap();

    let sig = git2::Signature::now("bench", "bench@test.com").unwrap();
    let types = ["feat", "fix", "refactor", "perf", "chore"];
    let scopes = ["api", "auth", "db"];

    let mut parent_oid: Option<git2::Oid> = None;

    for i in 0..num_commits {
        let t = types[i % types.len()];
        let s = scopes[i % scopes.len()];
        let breaking = if i % 20 == 0 && i > 0 { "!" } else { "" };
        let msg = format!("{t}({s}){breaking}: change {i}");

        // Create a file change per commit
        let file_name = format!("src/file_{i}.rs");
        let file_path = dir.path().join(&file_name);
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, format!("// commit {i}\n")).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(&file_name)).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();

        let oid = if let Some(parent) = parent_oid {
            let parent_commit = repo.find_commit(parent).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, &msg, &tree, &[&parent_commit])
                .unwrap()
        } else {
            repo.commit(Some("HEAD"), &sig, &sig, &msg, &tree, &[])
                .unwrap()
        };

        if i == tag_at {
            let obj = repo.find_object(oid, None).unwrap();
            repo.tag_lightweight("v1.0.0", &obj, false).unwrap();
        }

        parent_oid = Some(oid);
    }

    (dir, repo)
}

fn bench_git_operations(c: &mut Criterion) {
    // Benchmark get_commits_since_last_tag with varying history sizes
    for (label, total_commits, tag_position) in [
        ("git_commits/100", 100, 0),
        ("git_commits/1000", 1_000, 0),
        ("git_commits/5000", 5_000, 0),
    ] {
        let (_dir, repo) = create_bench_repo(total_commits, tag_position);
        c.bench_function(label, |b| {
            b.iter(|| {
                black_box(
                    get_commits_since_last_tag(&repo, "v", OrphanedTagStrategy::Warn).unwrap(),
                );
            });
        });
    }

    // Benchmark find_last_tag_name
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

    // Benchmark collect_all_tags
    {
        let (_dir, repo) = create_bench_repo(100, 50);
        c.bench_function("git_collect_tags/single_tag", |b| {
            b.iter(|| {
                black_box(collect_all_tags(&repo));
            });
        });
    }

    // Benchmark get_changed_files
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

    // Benchmark get_changed_files_since_tag
    for (label, total_commits, tag_position) in [
        ("git_changed_since_tag/100_commits_50_since", 100, 50),
        ("git_changed_since_tag/1000_commits_500_since", 1_000, 500),
    ] {
        let (_dir, repo) = create_bench_repo(total_commits, tag_position);
        c.bench_function(label, |b| {
            b.iter(|| {
                black_box(
                    get_changed_files_since_tag(&repo, "v", OrphanedTagStrategy::Warn).unwrap(),
                );
            });
        });
    }
}

fn bench_validate(c: &mut Criterion) {
    // Benchmark config loading + validation (the local part of validate)
    for (label, num_pkgs) in [
        ("validate/single", 1),
        ("validate/mono_50", 50),
        ("validate/mono_100", 100),
    ] {
        c.bench_function(label, |b| {
            let dir = TempDir::new().unwrap();
            let config_path = dir.path().join(".ferrflow");
            std::fs::write(&config_path, generate_config_json(num_pkgs)).unwrap();

            // Create version files so validation passes
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
    // Benchmark the complete check flow: config load + git log + commit parsing + bump determination
    for (label, num_commits) in [
        ("full_check_flow/100_commits", 100),
        ("full_check_flow/1000_commits", 1_000),
    ] {
        let (_dir, repo) = create_bench_repo(num_commits, 0);

        // Write a config into the repo dir
        let config_content = generate_config_json(1);
        std::fs::write(_dir.path().join(".ferrflow"), &config_content).unwrap();

        // Create version file (directory MUST exist before write)
        std::fs::create_dir_all(_dir.path().join("packages/pkg-001")).unwrap();
        std::fs::write(
            _dir.path().join("packages/pkg-001/package.json"),
            r#"{"name":"pkg-001","version":"1.0.0"}"#,
        )
        .unwrap();

        c.bench_function(label, |b| {
            b.iter(|| {
                let config = Config::load(_dir.path(), None).unwrap();
                let commits =
                    get_commits_since_last_tag(&repo, "v", OrphanedTagStrategy::Warn).unwrap();
                for commit in &commits {
                    black_box(determine_bump(&commit.message));
                }
                black_box((&config, commits.len()));
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
    bench_full_check_flow
);
criterion_main!(benches);
