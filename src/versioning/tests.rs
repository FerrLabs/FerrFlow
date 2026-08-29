use super::strategies::{bump_semver, calver_seq_version, calver_version};
use super::*;

#[test]
fn bootstrap_semver_variants() {
    assert_eq!(bootstrap_version(VersioningStrategy::Semver), "0.0.0");
    assert_eq!(bootstrap_version(VersioningStrategy::Zerover), "0.0.0");
}

#[test]
fn bootstrap_sequential_is_zero_integer() {
    assert_eq!(bootstrap_version(VersioningStrategy::Sequential), "0");
}

#[test]
fn bootstrap_calverseq_is_dotted_zero() {
    assert_eq!(bootstrap_version(VersioningStrategy::CalverSeq), "0.0");
}

#[test]
fn bootstrap_calver_returns_placeholder() {
    assert_eq!(bootstrap_version(VersioningStrategy::Calver), "0.0.0");
    assert_eq!(bootstrap_version(VersioningStrategy::CalverShort), "0.0.0");
}

#[test]
fn bootstrap_values_survive_first_bump_for_every_strategy() {
    for strategy in [
        VersioningStrategy::Semver,
        VersioningStrategy::Zerover,
        VersioningStrategy::Sequential,
        VersioningStrategy::CalverSeq,
        VersioningStrategy::Calver,
        VersioningStrategy::CalverShort,
    ] {
        let baseline = bootstrap_version(strategy);
        for bump in [BumpType::Patch, BumpType::Minor, BumpType::Major] {
            let result = compute_next_version(&baseline, bump, strategy, None);
            assert!(
                result.is_ok(),
                "bootstrap {baseline:?} with {bump:?} on {strategy:?} failed: {result:?}"
            );
        }
    }
}

#[test]
fn test_bump_patch() {
    assert_eq!(bump_version("1.2.3", BumpType::Patch).unwrap(), "1.2.4");
}

#[test]
fn test_bump_minor() {
    assert_eq!(bump_version("1.2.3", BumpType::Minor).unwrap(), "1.3.0");
}

#[test]
fn test_bump_major() {
    assert_eq!(bump_version("1.2.3", BumpType::Major).unwrap(), "2.0.0");
}

#[test]
fn test_bump_none() {
    assert_eq!(bump_version("1.2.3", BumpType::None).unwrap(), "1.2.3");
}

#[test]
fn test_bump_with_v_prefix() {
    assert_eq!(bump_version("v1.2.3", BumpType::Patch).unwrap(), "1.2.4");
}

#[test]
fn test_zerover_major_becomes_minor() {
    assert_eq!(bump_zerover("0.5.2", BumpType::Major).unwrap(), "0.6.0");
}

#[test]
fn test_zerover_clamps_major() {
    assert_eq!(bump_zerover("0.9.0", BumpType::Major).unwrap(), "0.10.0");
}

#[test]
fn test_zerover_patch() {
    assert_eq!(bump_zerover("0.5.2", BumpType::Patch).unwrap(), "0.5.3");
}

#[test]
fn test_sequential() {
    assert_eq!(bump_sequential("41").unwrap(), "42");
}

#[test]
fn test_sequential_from_zero() {
    assert_eq!(bump_sequential("0").unwrap(), "1");
}

#[test]
fn test_calver_format() {
    let v = calver_version("%Y.%m.%d").unwrap();
    assert_eq!(v.split('.').count(), 3);
}

#[test]
fn test_calver_short_format() {
    let v = calver_version("short").unwrap();
    assert_eq!(v.split('.').count(), 3);
    let year: u32 = v.split('.').next().unwrap().parse().unwrap();
    assert!(year < 100);
}

#[test]
fn test_calver_seq_new_month() {
    let v = calver_seq_version("2024.1.5", "%Y").unwrap();
    let parts: Vec<&str> = v.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[2], "1");
}

#[test]
fn test_calver_seq_same_month() {
    let now = chrono::Utc::now();
    let current = format!("{}.{}.3", now.format("%Y"), now.format("%-m"));
    let v = calver_seq_version(&current, "%Y").unwrap();
    assert!(v.ends_with(".4"));
}

#[test]
fn test_compute_next_version_semver() {
    assert_eq!(
        compute_next_version("1.2.3", BumpType::Minor, VersioningStrategy::Semver, None).unwrap(),
        "1.3.0"
    );
}

#[test]
fn test_compute_next_version_zerover() {
    assert_eq!(
        compute_next_version("0.5.2", BumpType::Major, VersioningStrategy::Zerover, None).unwrap(),
        "0.6.0"
    );
}

#[test]
fn test_compute_next_version_sequential() {
    assert_eq!(
        compute_next_version("10", BumpType::Patch, VersioningStrategy::Sequential, None).unwrap(),
        "11"
    );
}

#[test]
fn test_bump_invalid_version() {
    assert!(bump_version("not_a_version", BumpType::Patch).is_err());
}

#[test]
fn test_bump_empty_version() {
    assert!(bump_version("", BumpType::Patch).is_err());
}

#[test]
fn test_bump_pre_release_version() {
    let result = bump_version("1.0.0-alpha.1", BumpType::Patch).unwrap();
    assert!(result.starts_with("1.0.1"));
}

#[test]
fn test_zerover_none_keeps_version() {
    assert_eq!(bump_zerover("0.5.2", BumpType::None).unwrap(), "0.5.2");
}

#[test]
fn test_zerover_minor_same_as_major() {
    let from_major = bump_zerover("0.5.0", BumpType::Major).unwrap();
    let from_minor = bump_zerover("0.5.0", BumpType::Minor).unwrap();
    assert_eq!(from_major, from_minor);
}

#[test]
fn test_zerover_clamps_non_zero_major() {
    assert_eq!(bump_zerover("2.5.0", BumpType::Patch).unwrap(), "0.5.1");
}

#[test]
fn test_zerover_clears_prerelease_and_build_metadata() {
    assert_eq!(
        bump_zerover("0.4.0-beta.1", BumpType::Minor).unwrap(),
        "0.5.0"
    );
    assert_eq!(
        bump_zerover("0.4.0+build.7", BumpType::Patch).unwrap(),
        "0.4.1"
    );
    assert_eq!(
        bump_zerover("0.4.0-rc.1+build.7", BumpType::Major).unwrap(),
        "0.5.0"
    );
}

#[test]
fn test_zerover_invalid_version() {
    assert!(bump_zerover("garbage", BumpType::Patch).is_err());
}

#[test]
fn test_sequential_from_semver_fallback() {
    assert_eq!(bump_sequential("1.2.42").unwrap(), "43");
}

#[test]
fn test_sequential_from_garbage() {
    assert_eq!(bump_sequential("abc").unwrap(), "1");
}

#[test]
fn test_sequential_large_number() {
    assert_eq!(bump_sequential("999999").unwrap(), "1000000");
}

#[test]
fn test_sequential_with_v_prefix() {
    assert_eq!(bump_sequential("v42").unwrap(), "43");
}

#[test]
fn test_compute_next_version_calver() {
    let v =
        compute_next_version("0.0.0", BumpType::Minor, VersioningStrategy::Calver, None).unwrap();
    assert_eq!(v.split('.').count(), 3);
    let year: u32 = v.split('.').next().unwrap().parse().unwrap();
    assert!(year >= 2026);
}

#[test]
fn test_compute_next_version_calver_short() {
    let v = compute_next_version(
        "0.0.0",
        BumpType::Minor,
        VersioningStrategy::CalverShort,
        None,
    )
    .unwrap();
    let year: u32 = v.split('.').next().unwrap().parse().unwrap();
    assert!(year < 100);
}

#[test]
fn test_compute_next_version_calver_seq() {
    let v = compute_next_version(
        "2020.1.5",
        BumpType::Minor,
        VersioningStrategy::CalverSeq,
        None,
    )
    .unwrap();
    let parts: Vec<&str> = v.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[2], "1");
}

#[test]
fn truncate_semver_major() {
    assert_eq!(
        super::truncate_version("1.2.3", super::FloatingTagLevel::Major),
        Some("1".to_string())
    );
}

#[test]
fn truncate_semver_minor() {
    assert_eq!(
        super::truncate_version("1.2.3", super::FloatingTagLevel::Minor),
        Some("1.2".to_string())
    );
}

#[test]
fn truncate_calver_major() {
    assert_eq!(
        super::truncate_version("2026.03.31", super::FloatingTagLevel::Major),
        Some("2026".to_string())
    );
}

#[test]
fn truncate_calver_minor() {
    assert_eq!(
        super::truncate_version("2026.03.31", super::FloatingTagLevel::Minor),
        Some("2026.03".to_string())
    );
}

#[test]
fn truncate_sequential_major() {
    assert_eq!(
        super::truncate_version("42", super::FloatingTagLevel::Major),
        Some("42".to_string())
    );
}

#[test]
fn truncate_sequential_minor_returns_none() {
    assert_eq!(
        super::truncate_version("42", super::FloatingTagLevel::Minor),
        None
    );
}

#[test]
fn truncate_with_v_prefix() {
    assert_eq!(
        super::truncate_version("v1.2.3", super::FloatingTagLevel::Major),
        Some("1".to_string())
    );
}

#[test]
fn bump_from_zero() {
    assert_eq!(bump_version("0.0.0", BumpType::Patch).unwrap(), "0.0.1");
    assert_eq!(bump_version("0.0.0", BumpType::Minor).unwrap(), "0.1.0");
    assert_eq!(bump_version("0.0.0", BumpType::Major).unwrap(), "1.0.0");
}

#[test]
fn bump_large_versions() {
    assert_eq!(
        bump_version("99.99.99", BumpType::Patch).unwrap(),
        "99.99.100"
    );
    assert_eq!(
        bump_version("99.99.99", BumpType::Minor).unwrap(),
        "99.100.0"
    );
    assert_eq!(
        bump_version("99.99.99", BumpType::Major).unwrap(),
        "100.0.0"
    );
}

#[test]
fn zerover_from_zero() {
    assert_eq!(bump_zerover("0.0.0", BumpType::Major).unwrap(), "0.1.0");
    assert_eq!(bump_zerover("0.0.0", BumpType::Minor).unwrap(), "0.1.0");
    assert_eq!(bump_zerover("0.0.0", BumpType::Patch).unwrap(), "0.0.1");
}

#[test]
fn zerover_with_v_prefix() {
    assert_eq!(bump_zerover("v0.3.0", BumpType::Patch).unwrap(), "0.3.1");
}

#[test]
fn sequential_with_v_prefix_semver() {
    assert_eq!(bump_sequential("v1.2.5").unwrap(), "6");
}

#[test]
fn calver_seq_empty_string() {
    let v = calver_seq_version("", "%Y").unwrap();
    let parts: Vec<&str> = v.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[2], "1");
}

#[test]
fn calver_seq_malformed_input() {
    let v = calver_seq_version("garbage", "%Y").unwrap();
    assert!(v.ends_with(".1"));
}

#[test]
fn calver_seq_two_parts_only() {
    let now = chrono::Utc::now();
    let current = format!("{}.{}", now.format("%Y"), now.format("%-m"));
    let v = calver_seq_version(&current, "%Y").unwrap();
    assert!(v.ends_with(".1"));
}

#[test]
fn calver_seq_non_numeric_seq() {
    let now = chrono::Utc::now();
    let current = format!("{}.{}.abc", now.format("%Y"), now.format("%-m"));
    let v = calver_seq_version(&current, "%Y").unwrap();
    assert!(v.ends_with(".1"));
}

#[test]
fn truncate_single_component() {
    assert_eq!(
        truncate_version("42", FloatingTagLevel::Major),
        Some("42".to_string())
    );
    assert_eq!(truncate_version("42", FloatingTagLevel::Minor), None);
}

#[test]
fn truncate_v_prefix_minor() {
    assert_eq!(
        truncate_version("v2.5.9", FloatingTagLevel::Minor),
        Some("2.5".to_string())
    );
}

#[test]
fn bump_semver_strips_prerelease() {
    let result = bump_semver("1.1.0-dev.1", BumpType::Minor).unwrap();
    assert_eq!(result, "1.2.0");
}

#[test]
fn bump_semver_strips_prerelease_on_patch() {
    let result = bump_semver("2.0.0-rc.3", BumpType::Patch).unwrap();
    assert_eq!(result, "2.0.1");
}

#[test]
fn bump_semver_strips_build_metadata() {
    let result = bump_semver("1.0.0+build.42", BumpType::Major).unwrap();
    assert_eq!(result, "2.0.0");
}

#[test]
fn bump_semver_none_strips_prerelease() {
    let result = bump_semver("1.1.0-dev.1", BumpType::None).unwrap();
    assert_eq!(result, "1.1.0");
}

#[test]
fn detect_returns_none_when_no_tags() {
    assert_eq!(detect_strategy_from_tags(&[]), None);
}

#[test]
fn detect_semver_from_plain_v_tags() {
    let tags = vec!["v1.2.3", "v1.3.0", "v2.0.0"];
    assert_eq!(
        detect_strategy_from_tags(&tags),
        Some(VersioningStrategy::Semver)
    );
}

#[test]
fn detect_calver() {
    let tags = vec!["v2024.04.18", "v2024.05.01"];
    assert_eq!(
        detect_strategy_from_tags(&tags),
        Some(VersioningStrategy::Calver)
    );
}

#[test]
fn detect_calver_short() {
    let tags = vec!["v24.4.18", "v25.1.1"];
    assert_eq!(
        detect_strategy_from_tags(&tags),
        Some(VersioningStrategy::CalverShort)
    );
}

#[test]
fn detect_calver_seq() {
    let tags = vec!["v2024.04.1", "v2024.04.42", "v2024.04.100"];
    assert_eq!(
        detect_strategy_from_tags(&tags),
        Some(VersioningStrategy::CalverSeq)
    );
}

#[test]
fn detect_sequential() {
    let tags = vec!["v1", "v2", "v3"];
    assert_eq!(
        detect_strategy_from_tags(&tags),
        Some(VersioningStrategy::Sequential)
    );
}

#[test]
fn detect_ignores_monorepo_prefix() {
    let tags = vec!["pkg@v1.2.3", "pkg@v1.3.0"];
    assert_eq!(
        detect_strategy_from_tags(&tags),
        Some(VersioningStrategy::Semver)
    );
}

#[test]
fn detect_none_for_gibberish() {
    let tags = vec!["release-foo", "rc-2024"];
    assert_eq!(detect_strategy_from_tags(&tags), None);
}

#[test]
fn detect_prefers_calver_over_semver() {
    let tags = vec!["v2024.01.01", "v1.2.3"];
    assert_eq!(
        detect_strategy_from_tags(&tags),
        Some(VersioningStrategy::Calver)
    );
}

#[test]
fn detect_strips_tag_prefixes_like_release_slash() {
    let tags = vec!["release/v1.2.3"];
    assert_eq!(
        detect_strategy_from_tags(&tags),
        Some(VersioningStrategy::Semver)
    );
}

#[test]
fn detect_sequential_without_v_prefix() {
    let tags = vec!["1", "2", "3"];
    assert_eq!(
        detect_strategy_from_tags(&tags),
        Some(VersioningStrategy::Sequential)
    );
}

#[test]
fn detect_ignores_non_matching_tags_but_picks_matching() {
    let tags = vec!["latest", "stable", "v1.2.3", "nightly"];
    assert_eq!(
        detect_strategy_from_tags(&tags),
        Some(VersioningStrategy::Semver)
    );
}

#[test]
fn detect_calver_seq_mixed_with_calver_shape_prefers_seq() {
    let tags = vec!["v2024.04.10", "v2024.04.100"];
    assert_eq!(
        detect_strategy_from_tags(&tags),
        Some(VersioningStrategy::CalverSeq)
    );
}

#[test]
fn compute_next_version_all_strategies() {
    assert!(
        compute_next_version("1.0.0", BumpType::Patch, VersioningStrategy::Semver, None).is_ok()
    );
    assert!(
        compute_next_version("0.1.0", BumpType::Patch, VersioningStrategy::Zerover, None).is_ok()
    );
    assert!(
        compute_next_version("5", BumpType::Patch, VersioningStrategy::Sequential, None).is_ok()
    );
    assert!(
        compute_next_version(
            "2020.1.1",
            BumpType::Patch,
            VersioningStrategy::Calver,
            None
        )
        .is_ok()
    );
    assert!(
        compute_next_version(
            "2020.1.1",
            BumpType::Patch,
            VersioningStrategy::CalverShort,
            None
        )
        .is_ok()
    );
    assert!(
        compute_next_version(
            "2020.1.1",
            BumpType::Patch,
            VersioningStrategy::CalverSeq,
            None
        )
        .is_ok()
    );
}

#[test]
fn calver_short_seq_bootstraps_at_one() {
    let v = calver_seq_version("0.0", "%y").unwrap();
    let now = chrono::Utc::now();
    assert_eq!(v, format!("{}.{}.1", now.format("%y"), now.format("%-m")));
}

#[test]
fn calver_short_seq_increments_within_the_same_month() {
    let now = chrono::Utc::now();
    let current = format!("{}.{}.7", now.format("%y"), now.format("%-m"));

    let v = calver_seq_version(&current, "%y").unwrap();

    assert_eq!(v, format!("{}.{}.8", now.format("%y"), now.format("%-m")));
}

#[test]
fn calver_short_seq_can_publish_more_than_once_a_day() {
    let now = chrono::Utc::now();
    let first = calver_seq_version("0.0", "%y").unwrap();
    let second = calver_seq_version(&first, "%y").unwrap();
    let third = calver_seq_version(&second, "%y").unwrap();

    assert_ne!(first, second);
    assert_ne!(second, third);
    assert_eq!(
        third,
        format!("{}.{}.3", now.format("%y"), now.format("%-m"))
    );
}

#[test]
fn migrating_from_calver_short_continues_from_the_day_number() {
    // The point of the short variant: a repo on YY.M.D whose last release was
    // the 28th gets 29 next, so the switch costs no version discontinuity.
    let now = chrono::Utc::now();
    let last_short = format!("{}.{}.28", now.format("%y"), now.format("%-m"));

    let v = calver_seq_version(&last_short, "%y").unwrap();

    assert_eq!(v, format!("{}.{}.29", now.format("%y"), now.format("%-m")));
}

#[test]
fn calver_short_seq_keeps_the_two_digit_year_where_calver_seq_widens_it() {
    let now = chrono::Utc::now();
    let short = calver_seq_version("0.0", "%y").unwrap();
    let long = calver_seq_version("0.0", "%Y").unwrap();

    assert!(short.starts_with(&now.format("%y").to_string()));
    assert!(long.starts_with(&now.format("%Y").to_string()));
    assert_ne!(
        short, long,
        "switching to calver-seq is what jumps the major from 26 to 2026"
    );
}

#[test]
fn calver_short_seq_restarts_when_the_month_rolls_over() {
    let v = calver_seq_version("01.1.9", "%y").unwrap();
    let now = chrono::Utc::now();

    assert_eq!(
        v,
        format!("{}.{}.1", now.format("%y"), now.format("%-m")),
        "a version from another month must not carry its counter forward"
    );
}

#[test]
fn calver_short_seq_bootstraps_like_calver_seq() {
    assert_eq!(
        bootstrap_version(VersioningStrategy::CalverShortSeq),
        bootstrap_version(VersioningStrategy::CalverSeq)
    );
}

#[test]
fn calver_short_seq_is_reachable_through_compute_next_version() {
    let now = chrono::Utc::now();
    let v = compute_next_version(
        "0.0",
        BumpType::Minor,
        VersioningStrategy::CalverShortSeq,
        None,
    )
    .unwrap();

    assert_eq!(v, format!("{}.{}.1", now.format("%y"), now.format("%-m")));
}
