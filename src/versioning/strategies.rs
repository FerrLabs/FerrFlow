use anyhow::{Result, anyhow};
use chrono::Utc;
use semver::Version;

use crate::conventional_commits::BumpType;
use crate::error_code::{self, ErrorCodeExt};

fn overflow() -> anyhow::Error {
    anyhow!("Version component overflow: cannot bump past the maximum value")
}

pub(super) fn bump_core(
    major: u64,
    minor: u64,
    patch: u64,
    bump: BumpType,
) -> Option<(u64, u64, u64)> {
    match bump {
        BumpType::Major => Some((major.checked_add(1)?, 0, 0)),
        BumpType::Minor => Some((major, minor.checked_add(1)?, 0)),
        BumpType::Patch => Some((major, minor, patch.checked_add(1)?)),
        BumpType::None => Some((major, minor, patch)),
    }
}

pub(super) fn bump_core_zero(minor: u64, patch: u64, bump: BumpType) -> Option<(u64, u64, u64)> {
    match bump {
        BumpType::Major | BumpType::Minor => Some((0, minor.checked_add(1)?, 0)),
        BumpType::Patch => Some((0, minor, patch.checked_add(1)?)),
        BumpType::None => Some((0, minor, patch)),
    }
}

pub(super) fn bump_semver(current: &str, bump: BumpType) -> Result<String> {
    let mut v = Version::parse(current.trim_start_matches('v'))
        .map_err(|e| anyhow::anyhow!("Invalid semver '{}': {}", current, e))
        .error_code(error_code::VERSIONING_INVALID_SEMVER)?;

    v.pre = semver::Prerelease::EMPTY;
    v.build = semver::BuildMetadata::EMPTY;

    (v.major, v.minor, v.patch) =
        bump_core(v.major, v.minor, v.patch, bump).ok_or_else(overflow)?;

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
            parts[2]
                .parse::<u32>()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(overflow)?
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
    n.checked_add(1)
        .map(|next| next.to_string())
        .ok_or_else(overflow)
}

pub(super) fn bump_zerover(current: &str, bump: BumpType) -> Result<String> {
    let mut v = Version::parse(current.trim_start_matches('v'))
        .map_err(|e| anyhow::anyhow!("Invalid semver '{}': {}", current, e))
        .error_code(error_code::VERSIONING_INVALID_SEMVER)?;

    v.pre = semver::Prerelease::EMPTY;
    v.build = semver::BuildMetadata::EMPTY;

    (v.major, v.minor, v.patch) = bump_core_zero(v.minor, v.patch, bump).ok_or_else(overflow)?;
    v.major = 0;
    Ok(v.to_string())
}

#[cfg(kani)]
mod proofs {
    use super::*;

    fn any_bump() -> BumpType {
        match kani::any::<u8>() % 4 {
            0 => BumpType::Major,
            1 => BumpType::Minor,
            2 => BumpType::Patch,
            _ => BumpType::None,
        }
    }

    #[kani::proof]
    fn semver_bump_never_goes_backwards() {
        let (major, minor, patch): (u64, u64, u64) = (kani::any(), kani::any(), kani::any());
        let bump = any_bump();
        match bump_core(major, minor, patch, bump) {
            Some(next) => {
                let before = (major, minor, patch);
                if bump == BumpType::None {
                    assert!(next == before);
                } else {
                    assert!(next > before);
                }
            }
            None => {
                let at_max = match bump {
                    BumpType::Major => major == u64::MAX,
                    BumpType::Minor => minor == u64::MAX,
                    BumpType::Patch => patch == u64::MAX,
                    BumpType::None => false,
                };
                assert!(at_max);
            }
        }
    }

    #[kani::proof]
    fn zerover_bump_stays_below_one_and_never_goes_backwards() {
        let (minor, patch): (u64, u64) = (kani::any(), kani::any());
        let bump = any_bump();
        if let Some((major, next_minor, next_patch)) = bump_core_zero(minor, patch, bump) {
            assert!(major == 0);
            if bump == BumpType::None {
                assert!((next_minor, next_patch) == (minor, patch));
            } else {
                assert!((next_minor, next_patch) > (minor, patch));
            }
        }
    }

    #[kani::proof]
    fn zerover_overflows_only_at_the_maximum_component() {
        let (minor, patch): (u64, u64) = (kani::any(), kani::any());
        let bump = any_bump();
        if bump_core_zero(minor, patch, bump).is_none() {
            let at_max = match bump {
                BumpType::Major | BumpType::Minor => minor == u64::MAX,
                BumpType::Patch => patch == u64::MAX,
                BumpType::None => false,
            };
            assert!(at_max);
        }
    }
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
