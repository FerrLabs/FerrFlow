use std::io::Write;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use ferrflow::changelog::{build_section, update_changelog};
use ferrflow::conventional_commits::{BumpType, determine_bump};
use ferrflow::git::GitLog;
use tempfile::NamedTempFile;

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

criterion_group!(benches, bench_commit_parsing, bench_changelog);
criterion_main!(benches);
