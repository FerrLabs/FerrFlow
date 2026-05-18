use anyhow::Result;

use crate::config::{FloatingTagLevel, VersioningStrategy};
use crate::conventional_commits::BumpType;

mod detect;
mod strategies;

pub use detect::detect_strategy_from_tags;

use strategies::{bump_semver, bump_sequential, bump_zerover, calver_seq_version, calver_version};

pub fn bootstrap_version(strategy: VersioningStrategy) -> String {
    match strategy {
        VersioningStrategy::Semver | VersioningStrategy::Zerover => "0.0.0".to_string(),
        VersioningStrategy::Sequential => "0".to_string(),
        VersioningStrategy::CalverSeq => "0.0".to_string(),
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

pub fn bump_version(current: &str, bump: BumpType) -> Result<String> {
    strategies::bump_semver(current, bump)
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
mod tests;
