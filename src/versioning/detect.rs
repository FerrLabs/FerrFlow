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

pub(super) fn classify_tag(tag: &str) -> Option<TagClass> {
    use regex::Regex;
    use std::sync::OnceLock;

    static CALVER_RE: OnceLock<Regex> = OnceLock::new();
    static CALVER_SHORT_RE: OnceLock<Regex> = OnceLock::new();
    static SEMVER_RE: OnceLock<Regex> = OnceLock::new();
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
