use anyhow::Result;
use chrono::Utc;
use semver::Version;

use crate::conventional_commits::BumpType;
use crate::error_code::{self, ErrorCodeExt};

pub(super) fn bump_semver(current: &str, bump: BumpType) -> Result<String> {
    let mut v = Version::parse(current.trim_start_matches('v'))
        .map_err(|e| anyhow::anyhow!("Invalid semver '{}': {}", current, e))
        .error_code(error_code::VERSIONING_INVALID_SEMVER)?;

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

pub(super) fn calver_version(format: &str) -> Result<String> {
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

pub(super) fn calver_seq_version(current: &str, year_format: &str) -> Result<String> {
    let now = Utc::now();
    let year_month = format!("{}.{}", now.format(year_format), now.format("%-m"));

    let seq = if current.starts_with(&year_month) {
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

pub(super) fn bump_sequential(current: &str) -> Result<String> {
    let n: u64 = current.trim_start_matches('v').parse().unwrap_or_else(|_| {
        Version::parse(current.trim_start_matches('v'))
            .map(|v| v.patch)
            .unwrap_or(0)
    });
    Ok((n + 1).to_string())
}

pub(super) fn bump_zerover(current: &str, bump: BumpType) -> Result<String> {
    let mut v = Version::parse(current.trim_start_matches('v'))
        .map_err(|e| anyhow::anyhow!("Invalid semver '{}': {}", current, e))
        .error_code(error_code::VERSIONING_INVALID_SEMVER)?;

    v.pre = semver::Prerelease::EMPTY;
    v.build = semver::BuildMetadata::EMPTY;

    match bump {
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

#[cfg(test)]
mod zerover_tests {
    use super::*;

    #[test]
    fn zerover_clears_prerelease_and_build_metadata() {
        assert_eq!(
            bump_zerover("0.4.0-beta.1+build", BumpType::Minor).unwrap(),
            "0.5.0"
        );
        assert_eq!(
            bump_zerover("0.4.0-rc.2", BumpType::Patch).unwrap(),
            "0.4.1"
        );
    }

    #[test]
    fn zerover_keeps_major_at_zero() {
        assert_eq!(bump_zerover("0.3.5", BumpType::Major).unwrap(), "0.4.0");
        assert_eq!(bump_zerover("1.2.3", BumpType::Minor).unwrap(), "0.3.0");
    }
}
