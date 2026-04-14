use crate::config::{BranchChannelConfig, ChannelValue, PrereleaseIdentifier};
use crate::error_code::{self, ErrorCodeExt};
use anyhow::Result;
use chrono::Utc;

pub struct PrereleaseContext {
    pub channel: Option<String>,
    pub identifier_strategy: PrereleaseIdentifier,
}

#[allow(dead_code)]
pub struct ResolvedPrerelease {
    pub channel: String,
    pub identifier: String,
    pub full_suffix: String,
}

impl PrereleaseContext {
    /// Resolve the pre-release channel.
    /// Priority: CLI flag > branch config match > None (stable).
    pub fn resolve(
        cli_channel: Option<&str>,
        current_branch: &str,
        branches_config: Option<&[BranchChannelConfig]>,
    ) -> Result<Self> {
        let ctx = if let Some(ch) = cli_channel {
            let strategy = branches_config
                .and_then(|branches| find_matching_branch(current_branch, branches))
                .map(|b| b.prerelease_identifier)
                .unwrap_or(PrereleaseIdentifier::Increment);
            PrereleaseContext {
                channel: Some(ch.to_string()),
                identifier_strategy: strategy,
            }
        } else if let Some(branch_config) =
            branches_config.and_then(|branches| find_matching_branch(current_branch, branches))
        {
            match &branch_config.channel {
                ChannelValue::Named(name) => PrereleaseContext {
                    channel: Some(name.clone()),
                    identifier_strategy: branch_config.prerelease_identifier,
                },
                ChannelValue::Stable(_) => PrereleaseContext {
                    channel: None,
                    identifier_strategy: PrereleaseIdentifier::Increment,
                },
            }
        } else {
            PrereleaseContext {
                channel: None,
                identifier_strategy: PrereleaseIdentifier::Increment,
            }
        };

        if let Some(ref ch) = ctx.channel {
            validate_channel_name(ch)?;
        }

        Ok(ctx)
    }

    pub fn is_prerelease(&self) -> bool {
        self.channel.is_some()
    }

    /// Compute the pre-release identifier. Returns None for stable releases.
    pub fn compute_identifier(
        &self,
        base_version: &str,
        tag_prefix: &str,
        existing_tags: &[String],
        short_hash: &str,
    ) -> Option<ResolvedPrerelease> {
        let channel = self.channel.as_ref()?;

        let identifier = match self.identifier_strategy {
            PrereleaseIdentifier::Increment => {
                let search_prefix = format!("{tag_prefix}{base_version}-{channel}.");
                let max_n = find_max_prerelease_number(&search_prefix, existing_tags);
                (max_n + 1).to_string()
            }
            PrereleaseIdentifier::Timestamp => Utc::now().format("%Y%m%dT%H%M").to_string(),
            PrereleaseIdentifier::ShortHash => short_hash.to_string(),
            PrereleaseIdentifier::TimestampHash => {
                let ts = Utc::now().format("%Y%m%dT%H%M").to_string();
                format!("{ts}-{short_hash}")
            }
        };

        let full_suffix = format!("-{channel}.{identifier}");

        Some(ResolvedPrerelease {
            channel: channel.clone(),
            identifier,
            full_suffix,
        })
    }
}

fn find_matching_branch<'a>(
    current_branch: &str,
    branches: &'a [BranchChannelConfig],
) -> Option<&'a BranchChannelConfig> {
    branches.iter().find(|b| {
        // Branch names use `/` as separators (e.g. fix/global, feature/auth),
        // but glob `*` doesn't cross `/`. Normalize lone `*` segments to `**`
        // so that `*` matches any branch including those with slashes.
        let pattern = b.name.replace("/*", "/**").replace("/*/", "/**/");
        let pattern = if pattern == "*" {
            "**".to_string()
        } else {
            pattern
        };
        glob_match::glob_match(&pattern, current_branch)
    })
}

fn find_max_prerelease_number(search_prefix: &str, tags: &[String]) -> u64 {
    tags.iter()
        .filter_map(|tag| {
            let suffix = tag.strip_prefix(search_prefix)?;
            suffix.parse::<u64>().ok()
        })
        .max()
        .unwrap_or(0)
}

/// Validate that a channel name is a valid semver pre-release identifier segment.
/// Must be non-empty, alphanumeric + hyphens only.
pub fn validate_channel_name(name: &str) -> Result<()> {
    if name.is_empty() {
        Err(anyhow::anyhow!("Pre-release channel name cannot be empty"))
            .error_code(error_code::PRERELEASE_EMPTY_CHANNEL)?;
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        Err(anyhow::anyhow!(
            "Invalid channel name '{}': must contain only alphanumeric characters and hyphens",
            name
        ))
        .error_code(error_code::PRERELEASE_INVALID_CHANNEL)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn branch(
        name: &str,
        channel: ChannelValue,
        strategy: PrereleaseIdentifier,
    ) -> BranchChannelConfig {
        BranchChannelConfig {
            name: name.to_string(),
            channel,
            prerelease_identifier: strategy,
        }
    }

    // --- Channel resolution tests ---

    #[test]
    fn cli_flag_takes_priority() {
        let branches = vec![branch(
            "main",
            ChannelValue::Stable(false),
            PrereleaseIdentifier::Increment,
        )];
        let ctx = PrereleaseContext::resolve(Some("beta"), "main", Some(&branches)).unwrap();
        assert_eq!(ctx.channel.as_deref(), Some("beta"));
        assert!(ctx.is_prerelease());
    }

    #[test]
    fn branch_match_named_channel() {
        let branches = vec![
            branch(
                "main",
                ChannelValue::Stable(false),
                PrereleaseIdentifier::Increment,
            ),
            branch(
                "develop",
                ChannelValue::Named("dev".to_string()),
                PrereleaseIdentifier::Timestamp,
            ),
        ];
        let ctx = PrereleaseContext::resolve(None, "develop", Some(&branches)).unwrap();
        assert_eq!(ctx.channel.as_deref(), Some("dev"));
        assert_eq!(ctx.identifier_strategy, PrereleaseIdentifier::Timestamp);
    }

    #[test]
    fn branch_match_stable() {
        let branches = vec![branch(
            "main",
            ChannelValue::Stable(false),
            PrereleaseIdentifier::Increment,
        )];
        let ctx = PrereleaseContext::resolve(None, "main", Some(&branches)).unwrap();
        assert!(ctx.channel.is_none());
        assert!(!ctx.is_prerelease());
    }

    #[test]
    fn no_branches_config_is_stable() {
        let ctx = PrereleaseContext::resolve(None, "develop", None).unwrap();
        assert!(ctx.channel.is_none());
    }

    #[test]
    fn unmatched_branch_is_stable() {
        let branches = vec![branch(
            "main",
            ChannelValue::Stable(false),
            PrereleaseIdentifier::Increment,
        )];
        let ctx = PrereleaseContext::resolve(None, "feature/foo", Some(&branches)).unwrap();
        assert!(ctx.channel.is_none());
    }

    #[test]
    fn glob_pattern_match() {
        let branches = vec![branch(
            "release/*",
            ChannelValue::Named("rc".to_string()),
            PrereleaseIdentifier::Increment,
        )];
        let ctx = PrereleaseContext::resolve(None, "release/2.0", Some(&branches)).unwrap();
        assert_eq!(ctx.channel.as_deref(), Some("rc"));
    }

    #[test]
    fn first_match_wins() {
        let branches = vec![
            branch(
                "develop",
                ChannelValue::Named("dev".to_string()),
                PrereleaseIdentifier::Timestamp,
            ),
            branch(
                "*",
                ChannelValue::Named("nightly".to_string()),
                PrereleaseIdentifier::Increment,
            ),
        ];
        let ctx = PrereleaseContext::resolve(None, "develop", Some(&branches)).unwrap();
        assert_eq!(ctx.channel.as_deref(), Some("dev"));
    }

    #[test]
    fn wildcard_matches_branch_with_slash() {
        let branches = vec![
            branch(
                "main",
                ChannelValue::Stable(false),
                PrereleaseIdentifier::Increment,
            ),
            branch(
                "*",
                ChannelValue::Named("dev".to_string()),
                PrereleaseIdentifier::Increment,
            ),
        ];
        let ctx = PrereleaseContext::resolve(None, "fix/global", Some(&branches)).unwrap();
        assert_eq!(ctx.channel.as_deref(), Some("dev"));
    }

    #[test]
    fn wildcard_prefix_matches_nested_branch() {
        let branches = vec![branch(
            "feature/*",
            ChannelValue::Named("dev".to_string()),
            PrereleaseIdentifier::Increment,
        )];
        let ctx = PrereleaseContext::resolve(None, "feature/auth/oauth", Some(&branches)).unwrap();
        assert_eq!(ctx.channel.as_deref(), Some("dev"));
    }

    #[test]
    fn cli_flag_uses_branch_strategy() {
        let branches = vec![branch(
            "develop",
            ChannelValue::Named("dev".to_string()),
            PrereleaseIdentifier::TimestampHash,
        )];
        let ctx = PrereleaseContext::resolve(Some("beta"), "develop", Some(&branches)).unwrap();
        assert_eq!(ctx.channel.as_deref(), Some("beta"));
        assert_eq!(ctx.identifier_strategy, PrereleaseIdentifier::TimestampHash);
    }

    // --- Identifier computation tests ---

    #[test]
    fn increment_no_existing_tags() {
        let ctx = PrereleaseContext {
            channel: Some("beta".to_string()),
            identifier_strategy: PrereleaseIdentifier::Increment,
        };
        let result = ctx
            .compute_identifier("2.0.0", "v", &[], "abc1234")
            .unwrap();
        assert_eq!(result.channel, "beta");
        assert_eq!(result.identifier, "1");
        assert_eq!(result.full_suffix, "-beta.1");
    }

    #[test]
    fn increment_with_existing_tags() {
        let tags = vec![
            "v2.0.0-beta.1".to_string(),
            "v2.0.0-beta.2".to_string(),
            "v2.0.0-beta.5".to_string(),
            "v1.0.0-beta.10".to_string(),
        ];
        let ctx = PrereleaseContext {
            channel: Some("beta".to_string()),
            identifier_strategy: PrereleaseIdentifier::Increment,
        };
        let result = ctx
            .compute_identifier("2.0.0", "v", &tags, "abc1234")
            .unwrap();
        assert_eq!(result.identifier, "6");
        assert_eq!(result.full_suffix, "-beta.6");
    }

    #[test]
    fn timestamp_strategy() {
        let ctx = PrereleaseContext {
            channel: Some("dev".to_string()),
            identifier_strategy: PrereleaseIdentifier::Timestamp,
        };
        let result = ctx
            .compute_identifier("2.0.0", "v", &[], "abc1234")
            .unwrap();
        assert_eq!(result.channel, "dev");
        assert_eq!(result.identifier.len(), 13);
        assert!(result.identifier.contains('T'));
        assert!(result.full_suffix.starts_with("-dev."));
    }

    #[test]
    fn short_hash_strategy() {
        let ctx = PrereleaseContext {
            channel: Some("dev".to_string()),
            identifier_strategy: PrereleaseIdentifier::ShortHash,
        };
        let result = ctx
            .compute_identifier("2.0.0", "v", &[], "a1b2c3d")
            .unwrap();
        assert_eq!(result.identifier, "a1b2c3d");
        assert_eq!(result.full_suffix, "-dev.a1b2c3d");
    }

    #[test]
    fn timestamp_hash_strategy() {
        let ctx = PrereleaseContext {
            channel: Some("dev".to_string()),
            identifier_strategy: PrereleaseIdentifier::TimestampHash,
        };
        let result = ctx
            .compute_identifier("2.0.0", "v", &[], "a1b2c3d")
            .unwrap();
        assert!(result.identifier.contains("-a1b2c3d"));
        assert!(result.full_suffix.starts_with("-dev."));
    }

    #[test]
    fn stable_context_returns_none() {
        let ctx = PrereleaseContext {
            channel: None,
            identifier_strategy: PrereleaseIdentifier::Increment,
        };
        assert!(
            ctx.compute_identifier("2.0.0", "v", &[], "abc1234")
                .is_none()
        );
    }

    #[test]
    fn increment_monorepo_prefix() {
        let tags = vec![
            "sdk@v2.0.0-beta.3".to_string(),
            "api@v2.0.0-beta.7".to_string(),
        ];
        let ctx = PrereleaseContext {
            channel: Some("beta".to_string()),
            identifier_strategy: PrereleaseIdentifier::Increment,
        };
        let result = ctx
            .compute_identifier("2.0.0", "sdk@v", &tags, "abc1234")
            .unwrap();
        assert_eq!(result.identifier, "4");
        assert_eq!(result.full_suffix, "-beta.4");
    }

    // --- Validation tests ---

    #[test]
    fn validate_channel_name_valid() {
        assert!(validate_channel_name("beta").is_ok());
        assert!(validate_channel_name("rc").is_ok());
        assert!(validate_channel_name("dev").is_ok());
        assert!(validate_channel_name("alpha-1").is_ok());
    }

    #[test]
    fn validate_channel_name_invalid() {
        assert!(validate_channel_name("").is_err());
        assert!(validate_channel_name("beta.1").is_err());
        assert!(validate_channel_name("my channel").is_err());
        assert!(validate_channel_name("beta_1").is_err());
    }

    // --- find_max_prerelease_number tests ---

    #[test]
    fn find_max_prerelease_number_mixed_tags() {
        let tags = vec![
            "v2.0.0-beta.1".to_string(),
            "v2.0.0-beta.3".to_string(),
            "v2.0.0-rc.1".to_string(),
            "v1.0.0-beta.10".to_string(),
            "v2.0.0".to_string(),
            "something-else".to_string(),
        ];
        let max = find_max_prerelease_number("v2.0.0-beta.", &tags);
        assert_eq!(max, 3);
    }

    #[test]
    fn find_max_prerelease_number_no_matches() {
        let tags = vec!["v1.0.0".to_string()];
        let max = find_max_prerelease_number("v2.0.0-beta.", &tags);
        assert_eq!(max, 0);
    }
}
