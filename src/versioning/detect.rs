use crate::config::VersioningStrategy;

pub(super) fn strip_tag_prefix(tag: &str) -> &str {
    let after_at = tag.rsplit_once('@').map(|(_, rest)| rest).unwrap_or(tag);
    let after_slash = after_at
        .rsplit_once('/')
        .map(|(_, r)| r)
        .unwrap_or(after_at);
    after_slash
        .strip_prefix('v')
        .or_else(|| after_slash.strip_prefix('V'))
        .unwrap_or(after_slash)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum TagClass {
    CalverSeq,
    Calver,
    CalverShort,
    Semver,
    Sequential,
}

fn is_digits(s: &str, len: std::ops::RangeInclusive<usize>) -> bool {
    len.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

fn split_dotted3(s: &str) -> Option<[&str; 3]> {
    let mut parts = s.split('.');
    let triple = [parts.next()?, parts.next()?, parts.next()?];
    parts.next().is_none().then_some(triple)
}

pub(super) fn classify_tag(tag: &str) -> Option<TagClass> {
    let stripped = strip_tag_prefix(tag);
    let dotted = split_dotted3(stripped);

    if let Some([y, m, t]) = dotted
        && is_digits(y, 4..=4)
        && is_digits(m, 1..=2)
        && is_digits(t, 1..=usize::MAX)
    {
        let year: u32 = y.parse().ok()?;
        let month: u32 = m.parse().ok()?;
        let third: u32 = t.parse().ok()?;
        if (1970..=9999).contains(&year) && (1..=12).contains(&month) {
            if third > 31 {
                return Some(TagClass::CalverSeq);
            }
            return Some(TagClass::Calver);
        }
    }

    if let Some([y, m, d]) = dotted
        && is_digits(y, 2..=2)
        && is_digits(m, 1..=2)
        && is_digits(d, 1..=2)
    {
        let year: u32 = y.parse().ok()?;
        let month: u32 = m.parse().ok()?;
        let day: u32 = d.parse().ok()?;
        if (20..=99).contains(&year) && (1..=12).contains(&month) && (1..=31).contains(&day) {
            return Some(TagClass::CalverShort);
        }
    }

    if dotted.is_some_and(|parts| parts.iter().all(|p| is_digits(p, 1..=usize::MAX))) {
        return Some(TagClass::Semver);
    }

    if is_digits(stripped, 1..=usize::MAX) {
        return Some(TagClass::Sequential);
    }

    None
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_each_shape() {
        assert_eq!(classify_tag("v2024.3.45"), Some(TagClass::CalverSeq));
        assert_eq!(classify_tag("2024.3.12"), Some(TagClass::Calver));
        assert_eq!(classify_tag("24.3.12"), Some(TagClass::CalverShort));
        assert_eq!(classify_tag("pkg@1.2.3"), Some(TagClass::Semver));
        assert_eq!(classify_tag("7"), Some(TagClass::Sequential));
        assert_eq!(classify_tag("1.2.3-rc.1"), None);
        assert_eq!(classify_tag("1.2.3+build"), None);
        assert_eq!(classify_tag("1.2"), None);
        assert_eq!(classify_tag("1.2.3.4"), None);
        assert_eq!(classify_tag("1..3"), None);
        assert_eq!(classify_tag(""), None);
        assert_eq!(classify_tag("v"), None);
    }

    #[test]
    fn out_of_range_calver_falls_through_to_semver() {
        assert_eq!(classify_tag("2024.13.1"), Some(TagClass::Semver));
        assert_eq!(classify_tag("1969.1.1"), Some(TagClass::Semver));
        assert_eq!(classify_tag("19.1.1"), Some(TagClass::Semver));
        assert_eq!(classify_tag("24.1.32"), Some(TagClass::Semver));
        assert_eq!(classify_tag("2024.0.1"), Some(TagClass::Semver));
    }

    #[test]
    fn overflowing_calver_component_is_unclassified() {
        assert_eq!(classify_tag("2024.1.99999999999"), None);
        assert_eq!(classify_tag("99999999999.1.1"), Some(TagClass::Semver));
    }

    #[test]
    fn non_ascii_digits_are_not_versions() {
        assert_eq!(classify_tag("٢٠٢٤.١.١"), None);
        assert_eq!(classify_tag("١.٢.٣"), None);
        assert_eq!(classify_tag("٣"), None);
        assert_eq!(classify_tag("1.2.3\n"), None);
    }
}
