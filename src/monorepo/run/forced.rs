use anyhow::Result;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Forced<'a> {
    pub name: Option<&'a str>,
    pub version: &'a str,
}

pub(super) fn parse_forced_version<'a>(
    force_version: Option<&'a str>,
    is_monorepo: bool,
) -> Result<Option<Forced<'a>>> {
    let Some(fv) = force_version else {
        return Ok(None);
    };
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
    Ok(Some(parsed))
}

pub(super) fn forced_version_for<'a>(
    forced: &Option<Forced<'a>>,
    pkg_name: &str,
) -> Option<&'a str> {
    let forced = forced.as_ref()?;
    match forced.name {
        Some(target) if target != pkg_name => None,
        _ => Some(forced.version),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_none_returns_none() {
        let r = parse_forced_version(None, false).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn parse_bare_version_in_single_package_mode() {
        let r = parse_forced_version(Some("1.2.3"), false).unwrap().unwrap();
        assert_eq!(r.name, None);
        assert_eq!(r.version, "1.2.3");
    }

    #[test]
    fn parse_bare_version_with_v_prefix_in_single_package_mode() {
        let r = parse_forced_version(Some("v1.2.3"), false)
            .unwrap()
            .unwrap();
        assert_eq!(r.version, "v1.2.3");
    }

    #[test]
    fn parse_bare_version_in_monorepo_fails() {
        let err = parse_forced_version(Some("1.2.3"), true).unwrap_err();
        assert!(err.to_string().contains("NAME@VERSION"));
    }

    #[test]
    fn parse_name_at_version() {
        let r = parse_forced_version(Some("api@1.2.3"), true)
            .unwrap()
            .unwrap();
        assert_eq!(r.name, Some("api"));
        assert_eq!(r.version, "1.2.3");
    }

    #[test]
    fn parse_empty_name_fails() {
        let err = parse_forced_version(Some("@1.2.3"), true).unwrap_err();
        assert!(err.to_string().contains("expected NAME@VERSION"));
    }

    #[test]
    fn parse_empty_version_fails() {
        let err = parse_forced_version(Some("api@"), true).unwrap_err();
        assert!(err.to_string().contains("expected NAME@VERSION"));
    }

    #[test]
    fn parse_invalid_semver_fails() {
        let err = parse_forced_version(Some("api@not-a-version"), true).unwrap_err();
        assert!(err.to_string().contains("not valid semver"));
    }

    #[test]
    fn forced_version_for_targets_named_package() {
        let f = Some(Forced {
            name: Some("api"),
            version: "1.2.3",
        });
        assert_eq!(forced_version_for(&f, "api"), Some("1.2.3"));
        assert_eq!(forced_version_for(&f, "site"), None);
    }

    #[test]
    fn forced_version_for_applies_to_all_packages_when_unnamed() {
        let f = Some(Forced {
            name: None,
            version: "1.2.3",
        });
        assert_eq!(forced_version_for(&f, "api"), Some("1.2.3"));
        assert_eq!(forced_version_for(&f, "anything"), Some("1.2.3"));
    }

    #[test]
    fn forced_version_for_returns_none_when_no_force() {
        let f: Option<Forced<'_>> = None;
        assert_eq!(forced_version_for(&f, "api"), None);
    }
}
