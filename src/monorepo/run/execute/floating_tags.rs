use anyhow::Result;
use colored::Colorize;

use crate::config::{Config, PackageConfig};
use crate::error_code::{self, ErrorCodeExt};
use crate::git::{Repository, create_or_move_tag, get_tag_message, tag_exists};
use crate::versioning::truncate_version;

use super::super::summary::PlannedTag;
use super::ReleasePlan;

pub(super) fn create_and_move_floating_tags(
    plan: &mut ReleasePlan<'_>,
    floating_tag_names: &mut Vec<String>,
) -> Result<()> {
    for t in plan.tags_to_create.iter().filter(|t| !t.is_prerelease) {
        let pkg = package_for_tag(plan.config, t)?;
        if let Some(alias) = pkg.latest_tag_name(&plan.config.workspace) {
            move_floating_tag(plan, t, alias, floating_tag_names)?;
        }
        let levels = pkg.effective_floating_tags(&plan.config.workspace);
        for truncated in levels
            .iter()
            .filter_map(|level| truncate_version(&t.version, *level))
        {
            let float_tag = pkg.tag_for_version(
                &plan.config.workspace,
                plan.config.is_monorepo(),
                &truncated,
            );
            refuse_backward_move(plan, &float_tag, &t.version)?;
            move_floating_tag(plan, t, float_tag, floating_tag_names)?;
        }
    }
    Ok(())
}

fn package_for_tag<'a>(config: &'a Config, t: &PlannedTag) -> Result<&'a PackageConfig> {
    config
        .packages
        .iter()
        .find(|p| p.name == t.package)
        .ok_or_else(|| anyhow::anyhow!("package '{}' not found in config", t.package))
        .error_code(error_code::MONOREPO_PACKAGE_NOT_FOUND)
}

fn move_floating_tag(
    plan: &mut ReleasePlan<'_>,
    t: &PlannedTag,
    name: String,
    floating_tag_names: &mut Vec<String>,
) -> Result<()> {
    let msg = format!("Release {}", t.version);
    let moved = create_or_move_tag(plan.repo, &name, &msg)?;
    let verb = if moved { "Moved" } else { "Created" };
    if let Some((_, lines)) = plan
        .pkg_outputs
        .iter_mut()
        .rev()
        .find(|(n, _)| n == &t.package)
    {
        lines.push(format!("  ✓ {} floating tag {}", verb, name.cyan()));
    }
    floating_tag_names.push(name);
    Ok(())
}

fn refuse_backward_move(plan: &ReleasePlan<'_>, float_tag: &str, version: &str) -> Result<()> {
    let Some(old_ver) = backward_from(plan.repo, float_tag, version) else {
        return Ok(());
    };
    if !plan.force {
        Err(anyhow::anyhow!(
            "Floating tag {} would move backward ({} → {}). Use --force to override.",
            float_tag,
            old_ver,
            version,
        ))
        .error_code(error_code::MONOREPO_PUSH_FAILED)?;
    }
    tracing::warn!(
        "{}",
        format!(
            "  ⚠ Floating tag {} moves backward ({} → {})",
            float_tag, old_ver, version,
        )
        .yellow()
    );
    Ok(())
}

fn backward_from(repo: &Repository, float_tag: &str, version: &str) -> Option<String> {
    if !tag_exists(repo, float_tag) {
        return None;
    }
    release_ahead_of(&get_tag_message(repo, float_tag)?, version)
}

fn release_ahead_of(tag_message: &str, version: &str) -> Option<String> {
    let old_ver = tag_message.trim().strip_prefix("Release ")?;
    let old = semver::Version::parse(old_ver.trim_start_matches('v')).ok()?;
    let new = semver::Version::parse(version.trim_start_matches('v')).ok()?;
    (new < old).then(|| old_ver.to_string())
}

#[cfg(test)]
mod tests {
    use super::release_ahead_of;

    #[test]
    fn a_tag_message_with_a_trailing_newline_still_blocks_a_backward_move() {
        assert_eq!(
            release_ahead_of("Release 1.5.0\n", "1.4.2").as_deref(),
            Some("1.5.0")
        );
    }

    #[test]
    fn a_forward_or_equal_move_is_not_backward() {
        assert_eq!(release_ahead_of("Release 1.5.0\n", "1.6.0"), None);
        assert_eq!(release_ahead_of("Release 1.5.0", "1.5.0"), None);
    }

    #[test]
    fn a_v_prefixed_release_is_compared_by_version() {
        assert_eq!(
            release_ahead_of("Release v2.0.0\n", "v1.9.9").as_deref(),
            Some("v2.0.0")
        );
    }

    #[test]
    fn a_message_ferrflow_did_not_write_never_blocks() {
        assert_eq!(release_ahead_of("hand-made alias", "0.1.0"), None);
    }
}
