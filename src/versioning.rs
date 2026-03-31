use crate::config::{FloatingTagLevel, VersioningStrategy};
use crate::conventional_commits::BumpType;
use anyhow::Result;
use chrono::Local;
use semver::Version;

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
        .map_err(|e| anyhow::anyhow!("Invalid semver '{}': {}", current, e))?;

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
    let now = Local::now();
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
    let now = Local::now();
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
        .map_err(|e| anyhow::anyhow!("Invalid semver '{}': {}", current, e))?;

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
        let now = chrono::Local::now();
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
}
