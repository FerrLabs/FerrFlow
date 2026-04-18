use crate::config::{FloatingTagLevel, VersioningStrategy};
use crate::conventional_commits::BumpType;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::Result;
use chrono::Utc;
use semver::Version;

/// Return the baseline version to use when a package has no prior tag *and*
/// no readable version on disk. Each strategy gets the canonical zero-value
/// its [`compute_next_version`] bump function expects as input:
///
/// | Strategy            | Bootstrap value  |
/// |---------------------|------------------|
/// | `Semver`, `Zerover` | `0.0.0`          |
/// | `Sequential`        | `0`              |
/// | `CalverSeq`         | `0.0`            |
/// | `Calver`, `CalverShort` | `0.0.0` (ignored — calver ignores input) |
///
/// The release flow then runs the strategy-specific bump on top, so a first
/// `feat:` commit lands at `0.1.0` / `1` / today's date, and so on.
pub fn bootstrap_version(strategy: VersioningStrategy) -> String {
    match strategy {
        VersioningStrategy::Semver | VersioningStrategy::Zerover => "0.0.0".to_string(),
        VersioningStrategy::Sequential => "0".to_string(),
        VersioningStrategy::CalverSeq => "0.0".to_string(),
        // Calver variants are date-driven and ignore the input entirely, but
        // we still return a parseable semver string so any intermediate code
        // that inspects it doesn't crash.
        VersioningStrategy::Calver | VersioningStrategy::CalverShort => "0.0.0".to_string(),
    }
}

pub fn compute_next_version(
    current: &str,
    bump: BumpType,
    strategy: VersioningStrategy,
) -> Result<String> {
    match strategy {
        VersioningStrategy::Semver => bump_semver(current, bump),
        VersioningStrategy::Calver => calver_version("%Y.%m.%d"),
        VersioningStrategy::CalverShort => calver_version("short"),
        VersioningStrategy::CalverSeq => calver_seq_version(current),
        VersioningStrategy::Sequential => bump_sequential(current),
        VersioningStrategy::Zerover => bump_zerover(current, bump),
    }
}

fn bump_semver(current: &str, bump: BumpType) -> Result<String> {
    let mut v = Version::parse(current.trim_start_matches('v'))
        .map_err(|e| anyhow::anyhow!("Invalid semver '{}': {}", current, e))
        .error_code(error_code::VERSIONING_INVALID_SEMVER)?;

    // Strip any existing pre-release/build metadata so the base version is clean.
    // Pre-release suffixes are re-applied later by compute_identifier if needed.
    v.pre = semver::Prerelease::EMPTY;
    v.build = semver::BuildMetadata::EMPTY;

    match bump {
        BumpType::Major => {
            v.major += 1;
            v.minor = 0;
            v.patch = 0;
        }
        BumpType::Minor => {
            v.minor += 1;
            v.patch = 0;
        }
        BumpType::Patch => {
            v.patch += 1;
        }
        BumpType::None => {}
    }

    Ok(v.to_string())
}

fn calver_version(format: &str) -> Result<String> {
    let now = Utc::now();
    if format == "short" {
        Ok(format!(
            "{}.{}.{}",
            now.format("%y"),
            now.format("%-m"),
            now.format("%-d")
        ))
    } else {
        Ok(now.format("%Y.%-m.%-d").to_string())
    }
}

fn calver_seq_version(current: &str) -> Result<String> {
    let now = Utc::now();
    let year_month = format!("{}.{}", now.format("%Y"), now.format("%-m"));

    // Parse current version to check if same year.month prefix
    let seq = if current.starts_with(&year_month) {
        // Same month — increment the sequence number
        let parts: Vec<&str> = current.splitn(3, '.').collect();
        if parts.len() == 3 {
            parts[2].parse::<u32>().unwrap_or(0) + 1
        } else {
            1
        }
    } else {
        1
    };

    Ok(format!("{year_month}.{seq}"))
}

fn bump_sequential(current: &str) -> Result<String> {
    let n: u64 = current.trim_start_matches('v').parse().unwrap_or_else(|_| {
        // Try parsing as semver and use patch as sequence
        Version::parse(current.trim_start_matches('v'))
            .map(|v| v.patch)
            .unwrap_or(0)
    });
    Ok((n + 1).to_string())
}

fn bump_zerover(current: &str, bump: BumpType) -> Result<String> {
    let mut v = Version::parse(current.trim_start_matches('v'))
        .map_err(|e| anyhow::anyhow!("Invalid semver '{}': {}", current, e))
        .error_code(error_code::VERSIONING_INVALID_SEMVER)?;

    match bump {
        // Major bump becomes minor in zerover
        BumpType::Major => {
            v.minor += 1;
            v.patch = 0;
        }
        BumpType::Minor => {
            v.minor += 1;
            v.patch = 0;
        }
        BumpType::Patch => {
            v.patch += 1;
        }
        BumpType::None => {}
    }

    v.major = 0;
    Ok(v.to_string())
}

// Keep backward-compatible alias used by tests and other modules
pub fn bump_version(current: &str, bump: BumpType) -> Result<String> {
    bump_semver(current, bump)
}

/// Strip any monorepo prefix like `my-pkg@` and a leading `v`, yielding the
/// raw version-like tail. Also handles `release/` style prefixes by dropping
/// any leading non-digit characters up to (but not including) the first
/// segment that looks like a version.
fn strip_tag_prefix(tag: &str) -> &str {
    // First, if there's a `@` in the tag, take the portion after the last `@`.
    // This covers `pkg@v1.2.3`, `@scope/pkg@v1.2.3`, etc.
    let after_at = tag.rsplit_once('@').map(|(_, rest)| rest).unwrap_or(tag);
    // Then drop a `release/`, `rel/`, `refs/tags/` or any other prefix segment
    // separated by `/` — take the last path component.
    let after_slash = after_at
        .rsplit_once('/')
        .map(|(_, r)| r)
        .unwrap_or(after_at);
    // Finally strip a leading `v` or `V`.
    after_slash
        .strip_prefix('v')
        .or_else(|| after_slash.strip_prefix('V'))
        .unwrap_or(after_slash)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TagClass {
    CalverSeq,
    Calver,
    CalverShort,
    Semver,
    Sequential,
}

/// Classify a single tag into its most-specific category, if any.
///
/// Returns `None` for tags that don't look like any known version shape.
///
/// For 4-digit-year `YYYY.M.D` shapes with a third segment ≤ 31 we return
/// `Calver`; larger third segments indicate an auto-increment counter and
/// return `CalverSeq`. See module docs for the ambiguity rules.
fn classify_tag(tag: &str) -> Option<TagClass> {
    use regex::Regex;
    use std::sync::OnceLock;

    // Three-segment calver/calver-seq (4-digit year)
    static CALVER_RE: OnceLock<Regex> = OnceLock::new();
    // Three-segment calver-short (2-digit year 20..=99)
    static CALVER_SHORT_RE: OnceLock<Regex> = OnceLock::new();
    // Three-segment semver-ish
    static SEMVER_RE: OnceLock<Regex> = OnceLock::new();
    // Single number sequential
    static SEQUENTIAL_RE: OnceLock<Regex> = OnceLock::new();

    let calver_re = CALVER_RE.get_or_init(|| Regex::new(r"^(\d{4})\.(\d{1,2})\.(\d+)$").unwrap());
    let calver_short_re =
        CALVER_SHORT_RE.get_or_init(|| Regex::new(r"^(\d{2})\.(\d{1,2})\.(\d{1,2})$").unwrap());
    let semver_re = SEMVER_RE.get_or_init(|| Regex::new(r"^\d+\.\d+\.\d+$").unwrap());
    let sequential_re = SEQUENTIAL_RE.get_or_init(|| Regex::new(r"^\d+$").unwrap());

    let stripped = strip_tag_prefix(tag);

    if let Some(caps) = calver_re.captures(stripped) {
        let year: u32 = caps[1].parse().ok()?;
        let month: u32 = caps[2].parse().ok()?;
        let third: u32 = caps[3].parse().ok()?;
        // Require a plausible year and month to avoid misclassifying oddball
        // semver tags like `1970.13.0` that happen to match the digit shape.
        if (1970..=9999).contains(&year) && (1..=12).contains(&month) {
            if third > 31 {
                return Some(TagClass::CalverSeq);
            }
            return Some(TagClass::Calver);
        }
    }

    if let Some(caps) = calver_short_re.captures(stripped) {
        let year: u32 = caps[1].parse().ok()?;
        let month: u32 = caps[2].parse().ok()?;
        let day: u32 = caps[3].parse().ok()?;
        // Plausible short-year range + valid month/day window. Lower bound 20
        // avoids matching semver tags like `1.2.3` / `10.11.12`.
        if (20..=99).contains(&year) && (1..=12).contains(&month) && (1..=31).contains(&day) {
            return Some(TagClass::CalverShort);
        }
    }

    if semver_re.is_match(stripped) {
        return Some(TagClass::Semver);
    }

    if sequential_re.is_match(stripped) {
        return Some(TagClass::Sequential);
    }

    None
}

/// Infer the versioning strategy from a list of existing git tag names.
///
/// Returns `None` when no tags match any known strategy. Callers should fall
/// back to the explicit default ([`VersioningStrategy::Semver`]) in that case.
///
/// When tags match multiple patterns, the most specific one wins:
/// `calver-seq` > `calver` > `calver-short` > `semver` > `sequential`. That
/// ordering matters because a `v2024.04.18` tag matches both calver and a
/// relaxed semver shape — we pick the date-aware one.
///
/// Zerover is intentionally excluded: it is ambiguous with semver (both use
/// `X.Y.Z`), so we fall through to semver and require an explicit opt-in for
/// zerover.
pub fn detect_strategy_from_tags(tags: &[&str]) -> Option<VersioningStrategy> {
    let mut has_calver_seq = false;
    let mut has_calver = false;
    let mut has_calver_short = false;
    let mut has_semver = false;
    let mut has_sequential = false;

    for tag in tags {
        match classify_tag(tag) {
            Some(TagClass::CalverSeq) => has_calver_seq = true,
            Some(TagClass::Calver) => has_calver = true,
            Some(TagClass::CalverShort) => has_calver_short = true,
            Some(TagClass::Semver) => has_semver = true,
            Some(TagClass::Sequential) => has_sequential = true,
            None => {}
        }
    }

    // Specificity tiers — most specific wins even if less specific has more hits.
    if has_calver_seq {
        return Some(VersioningStrategy::CalverSeq);
    }
    if has_calver {
        return Some(VersioningStrategy::Calver);
    }
    if has_calver_short {
        return Some(VersioningStrategy::CalverShort);
    }
    if has_semver {
        return Some(VersioningStrategy::Semver);
    }
    if has_sequential {
        return Some(VersioningStrategy::Sequential);
    }
    None
}

pub fn truncate_version(version: &str, level: FloatingTagLevel) -> Option<String> {
    let v = version.trim_start_matches('v');
    let parts: Vec<&str> = v.split('.').collect();

    match level {
        FloatingTagLevel::Major => Some(parts[0].to_string()),
        FloatingTagLevel::Minor => {
            if parts.len() < 2 {
                None
            } else {
                Some(format!("{}.{}", parts[0], parts[1]))
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
        // Calver ignores the baseline anyway — we just need a non-empty
        // parseable string for intermediate logging/diffing.
        assert_eq!(bootstrap_version(VersioningStrategy::Calver), "0.0.0");
        assert_eq!(bootstrap_version(VersioningStrategy::CalverShort), "0.0.0");
    }

    #[test]
    fn bootstrap_values_survive_first_bump_for_every_strategy() {
        // The whole point of `bootstrap_version` is that feeding it into
        // `compute_next_version` with any bump type doesn't error — the first
        // release cuts a valid tag.
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
                let result = compute_next_version(&baseline, bump, strategy);
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
        // Should have 3 dot-separated parts
        assert_eq!(v.split('.').count(), 3);
    }

    #[test]
    fn test_calver_short_format() {
        let v = calver_version("short").unwrap();
        assert_eq!(v.split('.').count(), 3);
        // Year part should be 2 digits
        let year: u32 = v.split('.').next().unwrap().parse().unwrap();
        assert!(year < 100);
    }

    #[test]
    fn test_calver_seq_new_month() {
        let v = calver_seq_version("2024.1.5").unwrap();
        let parts: Vec<&str> = v.split('.').collect();
        assert_eq!(parts.len(), 3);
        // Should be current year.month.1 (new month resets seq)
        assert_eq!(parts[2], "1");
    }

    #[test]
    fn test_calver_seq_same_month() {
        let now = chrono::Utc::now();
        let current = format!("{}.{}.3", now.format("%Y"), now.format("%-m"));
        let v = calver_seq_version(&current).unwrap();
        assert!(v.ends_with(".4"));
    }

    #[test]
    fn test_compute_next_version_semver() {
        assert_eq!(
            compute_next_version("1.2.3", BumpType::Minor, VersioningStrategy::Semver).unwrap(),
            "1.3.0"
        );
    }

    #[test]
    fn test_compute_next_version_zerover() {
        assert_eq!(
            compute_next_version("0.5.2", BumpType::Major, VersioningStrategy::Zerover).unwrap(),
            "0.6.0"
        );
    }

    #[test]
    fn test_compute_next_version_sequential() {
        assert_eq!(
            compute_next_version("10", BumpType::Patch, VersioningStrategy::Sequential).unwrap(),
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
        // semver crate parses pre-release; patch bump increments patch but keeps pre-release
        let result = bump_version("1.0.0-alpha.1", BumpType::Patch).unwrap();
        // Pre-release is preserved in the version string
        assert!(result.starts_with("1.0.1"));
    }

    #[test]
    fn test_zerover_none_keeps_version() {
        assert_eq!(bump_zerover("0.5.2", BumpType::None).unwrap(), "0.5.2");
    }

    #[test]
    fn test_zerover_minor_same_as_major() {
        // In zerover, both major and minor bump the minor
        let from_major = bump_zerover("0.5.0", BumpType::Major).unwrap();
        let from_minor = bump_zerover("0.5.0", BumpType::Minor).unwrap();
        assert_eq!(from_major, from_minor);
    }

    #[test]
    fn test_zerover_clamps_non_zero_major() {
        // Even if input has major > 0, zerover forces it to 0
        assert_eq!(bump_zerover("2.5.0", BumpType::Patch).unwrap(), "0.5.1");
    }

    #[test]
    fn test_zerover_invalid_version() {
        assert!(bump_zerover("garbage", BumpType::Patch).is_err());
    }

    #[test]
    fn test_sequential_from_semver_fallback() {
        // When given a semver string, sequential uses patch as sequence
        assert_eq!(bump_sequential("1.2.42").unwrap(), "43");
    }

    #[test]
    fn test_sequential_from_garbage() {
        // When given garbage, defaults to 0, then increments to 1
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
        let v = compute_next_version("0.0.0", BumpType::Minor, VersioningStrategy::Calver).unwrap();
        assert_eq!(v.split('.').count(), 3);
        let year: u32 = v.split('.').next().unwrap().parse().unwrap();
        assert!(year >= 2026);
    }

    #[test]
    fn test_compute_next_version_calver_short() {
        let v = compute_next_version("0.0.0", BumpType::Minor, VersioningStrategy::CalverShort)
            .unwrap();
        let year: u32 = v.split('.').next().unwrap().parse().unwrap();
        assert!(year < 100);
    }

    #[test]
    fn test_compute_next_version_calver_seq() {
        let v = compute_next_version("2020.1.5", BumpType::Minor, VersioningStrategy::CalverSeq)
            .unwrap();
        let parts: Vec<&str> = v.split('.').collect();
        assert_eq!(parts.len(), 3);
        // Different year/month, so seq resets to 1
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
        let v = calver_seq_version("").unwrap();
        let parts: Vec<&str> = v.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[2], "1");
    }

    #[test]
    fn calver_seq_malformed_input() {
        let v = calver_seq_version("garbage").unwrap();
        assert!(v.ends_with(".1"));
    }

    #[test]
    fn calver_seq_two_parts_only() {
        let now = chrono::Utc::now();
        let current = format!("{}.{}", now.format("%Y"), now.format("%-m"));
        let v = calver_seq_version(&current).unwrap();
        // Two parts means splitn(3, '.') yields 2 elements, so seq = 1
        assert!(v.ends_with(".1"));
    }

    #[test]
    fn calver_seq_non_numeric_seq() {
        let now = chrono::Utc::now();
        let current = format!("{}.{}.abc", now.format("%Y"), now.format("%-m"));
        let v = calver_seq_version(&current).unwrap();
        // "abc".parse::<u32>() fails -> unwrap_or(0) -> 0 + 1 = 1
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
        // v2024.04.100 triggers calver-seq (100 > 31), even though
        // v2024.04.10 would otherwise classify as calver.
        let tags = vec!["v2024.04.10", "v2024.04.100"];
        assert_eq!(
            detect_strategy_from_tags(&tags),
            Some(VersioningStrategy::CalverSeq)
        );
    }

    #[test]
    fn compute_next_version_all_strategies() {
        // Verify each strategy variant works through the dispatch
        assert!(compute_next_version("1.0.0", BumpType::Patch, VersioningStrategy::Semver).is_ok());
        assert!(
            compute_next_version("0.1.0", BumpType::Patch, VersioningStrategy::Zerover).is_ok()
        );
        assert!(compute_next_version("5", BumpType::Patch, VersioningStrategy::Sequential).is_ok());
        assert!(
            compute_next_version("2020.1.1", BumpType::Patch, VersioningStrategy::Calver).is_ok()
        );
        assert!(
            compute_next_version("2020.1.1", BumpType::Patch, VersioningStrategy::CalverShort)
                .is_ok()
        );
        assert!(
            compute_next_version("2020.1.1", BumpType::Patch, VersioningStrategy::CalverSeq)
                .is_ok()
        );
    }
}
