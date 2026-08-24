use anyhow::Result;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Forced<'a> {
    pub name: Option<&'a str>,
    pub version: &'a str,
}

/// Parse every `--force-version` occurrence. Repeatable so a monorepo
/// release can pin several packages in one invocation, which is what
/// makes an interactively-chosen plan expressible as a command.
pub(super) fn parse_forced_versions<'a>(
    force_versions: &'a [String],
    is_monorepo: bool,
) -> Result<Vec<Forced<'a>>> {
    let mut parsed = Vec::with_capacity(force_versions.len());
    for fv in force_versions {
        let one = parse_one(fv, is_monorepo)?;
        if let Some(name) = one.name
            && parsed.iter().any(|p: &Forced<'_>| p.name == Some(name))
        {
            anyhow::bail!("--force-version given more than once for package {name:?}");
        }
        if one.name.is_none() && !parsed.is_empty() {
            anyhow::bail!(
                "--force-version without a package name cannot be combined with other overrides"
            );
        }
        parsed.push(one);
    }
    Ok(parsed)
}

fn parse_one<'a>(fv: &'a str, is_monorepo: bool) -> Result<Forced<'a>> {
    let parsed = if let Some(at_pos) = fv.find('@') {
        let name = &fv[..at_pos];
        let version = &fv[at_pos + 1..];
        if name.is_empty() || version.is_empty() {
            anyhow::bail!("Invalid --force-version format: expected NAME@VERSION, got {fv:?}");
        }
        Forced {
            name: Some(name),
            version,
        }
    } else {
        if is_monorepo {
            anyhow::bail!(
                "In a monorepo, --force-version requires NAME@VERSION format (e.g. api@1.2.3)"
            );
        }
        Forced {
            name: None,
            version: fv,
        }
    };
    let clean = parsed.version.strip_prefix('v').unwrap_or(parsed.version);
    if semver::Version::parse(clean).is_err() {
        anyhow::bail!(
            "Invalid version in --force-version: {:?} is not valid semver",
            parsed.version
        );
    }
    Ok(parsed)
}

pub(super) fn forced_version_for<'a>(forced: &[Forced<'a>], pkg_name: &str) -> Option<&'a str> {
    forced
        .iter()
        .find(|f| match f.name {
            Some(target) => target == pkg_name,
            None => true,
        })
        .map(|f| f.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_overrides_parses_to_an_empty_list() {
        assert!(parse_forced_versions(&[], false).unwrap().is_empty());
    }

    #[test]
    fn parse_bare_version_in_single_package_mode() {
        let raw = v(&["1.2.3"]);
        let r = parse_forced_versions(&raw, false).unwrap();
        assert_eq!(r[0].name, None);
        assert_eq!(r[0].version, "1.2.3");
    }

    #[test]
    fn parse_bare_version_with_v_prefix_in_single_package_mode() {
        let raw = v(&["v1.2.3"]);
        let r = parse_forced_versions(&raw, false).unwrap();
        assert_eq!(r[0].version, "v1.2.3");
    }

    #[test]
    fn parse_bare_version_in_monorepo_fails() {
        let raw = v(&["1.2.3"]);
        let err = parse_forced_versions(&raw, true).unwrap_err();
        assert!(err.to_string().contains("NAME@VERSION"));
    }

    #[test]
    fn parse_name_at_version() {
        let raw = v(&["api@1.2.3"]);
        let r = parse_forced_versions(&raw, true).unwrap();
        assert_eq!(r[0].name, Some("api"));
        assert_eq!(r[0].version, "1.2.3");
    }

    #[test]
    fn several_packages_can_be_pinned_in_one_invocation() {
        let raw = v(&["api@1.2.3", "core@2.0.0"]);
        let r = parse_forced_versions(&raw, true).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(forced_version_for(&r, "api"), Some("1.2.3"));
        assert_eq!(forced_version_for(&r, "core"), Some("2.0.0"));
        assert_eq!(forced_version_for(&r, "web"), None);
    }

    #[test]
    fn pinning_the_same_package_twice_is_rejected_rather_than_last_wins() {
        let raw = v(&["api@1.2.3", "api@2.0.0"]);
        let err = parse_forced_versions(&raw, true).unwrap_err();
        assert!(err.to_string().contains("more than once"), "{err}");
    }

    #[test]
    fn a_bare_version_cannot_be_mixed_with_named_overrides() {
        let raw = v(&["api@1.2.3", "2.0.0"]);
        assert!(parse_forced_versions(&raw, false).is_err());
    }

    #[test]
    fn an_invalid_version_is_rejected() {
        let raw = v(&["api@not-a-version"]);
        let err = parse_forced_versions(&raw, true).unwrap_err();
        assert!(err.to_string().contains("not valid semver"));
    }
}
