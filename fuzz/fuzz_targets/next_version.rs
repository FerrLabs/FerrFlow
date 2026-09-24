#![no_main]

use libfuzzer_sys::fuzz_target;

use ferrflow::config::VersioningStrategy;
use ferrflow::conventional_commits::BumpType;
use ferrflow::versioning::compute_next_version;

const BUMPS: &[BumpType] = &[
    BumpType::Major,
    BumpType::Minor,
    BumpType::Patch,
    BumpType::None,
];

const STRATEGIES: &[VersioningStrategy] = &[
    VersioningStrategy::Semver,
    VersioningStrategy::Calver,
    VersioningStrategy::CalverShort,
    VersioningStrategy::CalverSeq,
    VersioningStrategy::CalverShortSeq,
    VersioningStrategy::Sequential,
    VersioningStrategy::Zerover,
];

#[derive(arbitrary::Arbitrary, Debug)]
struct Input<'a> {
    bump: u8,
    strategy: u8,
    current: &'a str,
    template: Option<&'a str>,
}

fuzz_target!(|input: Input| {
    let bump = BUMPS[input.bump as usize % BUMPS.len()];
    let strategy = STRATEGIES[input.strategy as usize % STRATEGIES.len()];

    let Ok(next) = compute_next_version(input.current, bump, strategy, input.template) else {
        return;
    };

    assert!(!next.is_empty(), "a computed version is never empty");
    assert_eq!(
        next.trim(),
        next,
        "a computed version carries no whitespace"
    );

    if input.template.is_none() && strategy == VersioningStrategy::Semver {
        let parsed = semver_ok(&next);
        assert!(parsed, "semver produced {next:?}, which is not a version");
    }
});

fn semver_ok(version: &str) -> bool {
    let core = version.split(['-', '+']).next().unwrap_or_default();
    let mut parts = core.split('.');
    let ok =
        |p: Option<&str>| p.is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()));
    ok(parts.next()) && ok(parts.next()) && ok(parts.next()) && parts.next().is_none()
}
