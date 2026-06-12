use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum BumpType {
    None,
    Patch,
    Minor,
    Major,
}

impl std::fmt::Display for BumpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BumpType::None => write!(f, "none"),
            BumpType::Patch => write!(f, "patch"),
            BumpType::Minor => write!(f, "minor"),
            BumpType::Major => write!(f, "major"),
        }
    }
}

static BREAKING_RE: OnceLock<Regex> = OnceLock::new();
static FEAT_RE: OnceLock<Regex> = OnceLock::new();
static PATCH_RE: OnceLock<Regex> = OnceLock::new();

fn breaking_header_re() -> &'static Regex {
    BREAKING_RE.get_or_init(|| {
        Regex::new(r"^(feat|fix|refactor|perf|build|chore|docs|style|test|ci)(\(.+\))?!:").unwrap()
    })
}

fn feat_header_re() -> &'static Regex {
    FEAT_RE.get_or_init(|| Regex::new(r"^feat(\(.+\))?:").unwrap())
}

fn patch_header_re() -> &'static Regex {
    PATCH_RE.get_or_init(|| Regex::new(r"^(fix|perf|refactor)(\(.+\))?:").unwrap())
}

static BREAKING_FOOTER_RE: OnceLock<Regex> = OnceLock::new();

fn breaking_footer_re() -> &'static Regex {
    BREAKING_FOOTER_RE.get_or_init(|| Regex::new(r"(?m)^BREAKING[ -]CHANGE:").unwrap())
}

pub fn determine_bump(message: &str) -> BumpType {
    let header = parse_subject(message);

    if breaking_header_re().is_match(header) || breaking_footer_re().is_match(message) {
        return BumpType::Major;
    }
    if feat_header_re().is_match(header) {
        return BumpType::Minor;
    }
    if patch_header_re().is_match(header) {
        return BumpType::Patch;
    }
    BumpType::None
}

pub fn parse_subject(message: &str) -> &str {
    message.lines().next().unwrap_or("").trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch() {
        assert_eq!(determine_bump("fix: correct typo"), BumpType::Patch);
        assert_eq!(determine_bump("perf: faster query"), BumpType::Patch);
        assert_eq!(determine_bump("refactor: clean up"), BumpType::Patch);
    }

    #[test]
    fn test_minor() {
        assert_eq!(determine_bump("feat: add login"), BumpType::Minor);
        assert_eq!(determine_bump("feat(auth): add JWT"), BumpType::Minor);
    }

    #[test]
    fn test_major() {
        assert_eq!(determine_bump("feat!: breaking change"), BumpType::Major);
        assert_eq!(
            determine_bump("fix(api)!: remove endpoint"),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("BREAKING CHANGE: removed X"),
            BumpType::Major
        );
    }

    #[test]
    fn test_none() {
        assert_eq!(determine_bump("chore: update deps"), BumpType::None);
        assert_eq!(determine_bump("docs: update readme"), BumpType::None);
        assert_eq!(determine_bump("ci: fix pipeline"), BumpType::None);
    }

    #[test]
    fn test_parse_subject() {
        assert_eq!(parse_subject("feat: add login"), "feat: add login");
        assert_eq!(
            parse_subject("feat: add login\n\nbody text"),
            "feat: add login"
        );
        assert_eq!(parse_subject("  spaced  "), "spaced");
        assert_eq!(parse_subject(""), "");
    }

    #[test]
    fn test_scoped_commits() {
        assert_eq!(determine_bump("fix(api): null check"), BumpType::Patch);
        assert_eq!(determine_bump("feat(ui): new button"), BumpType::Minor);
        assert_eq!(determine_bump("refactor(db): simplify"), BumpType::Patch);
    }

    #[test]
    fn test_breaking_change_in_body() {
        let msg = "feat: something\n\nBREAKING CHANGE: removed old API";
        assert_eq!(determine_bump(msg), BumpType::Major);
    }

    #[test]
    fn test_breaking_change_hyphen_footer() {
        let msg = "feat: something\n\nBREAKING-CHANGE: removed old API";
        assert_eq!(determine_bump(msg), BumpType::Major);
    }

    #[test]
    fn test_breaking_change_prose_is_not_major() {
        assert_eq!(
            determine_bump("docs: note that BREAKING CHANGES are coming in v2"),
            BumpType::None
        );
        let body = "feat: add flag\n\nBREAKING CHANGE will be handled later, not yet";
        assert_eq!(determine_bump(body), BumpType::Minor);
        let plural = "chore: cleanup\n\nBREAKING CHANGES: none in this one";
        assert_eq!(determine_bump(plural), BumpType::None);
    }

    #[test]
    fn test_bump_ordering() {
        assert!(BumpType::Major > BumpType::Minor);
        assert!(BumpType::Minor > BumpType::Patch);
        assert!(BumpType::Patch > BumpType::None);
    }

    #[test]
    fn test_empty_message() {
        assert_eq!(determine_bump(""), BumpType::None);
    }

    #[test]
    fn test_whitespace_only_message() {
        assert_eq!(determine_bump("   \n\n  "), BumpType::None);
    }

    #[test]
    fn test_non_conventional_message() {
        assert_eq!(determine_bump("update readme"), BumpType::None);
        assert_eq!(determine_bump("fixed the thing"), BumpType::None);
        assert_eq!(determine_bump("WIP"), BumpType::None);
    }

    #[test]
    fn test_all_patch_types() {
        assert_eq!(determine_bump("fix: something"), BumpType::Patch);
        assert_eq!(determine_bump("perf: something"), BumpType::Patch);
        assert_eq!(determine_bump("refactor: something"), BumpType::Patch);
    }

    #[test]
    fn test_all_none_types() {
        assert_eq!(determine_bump("chore: something"), BumpType::None);
        assert_eq!(determine_bump("docs: something"), BumpType::None);
        assert_eq!(determine_bump("ci: something"), BumpType::None);
        assert_eq!(determine_bump("style: something"), BumpType::None);
        assert_eq!(determine_bump("test: something"), BumpType::None);
        assert_eq!(determine_bump("build: something"), BumpType::None);
    }

    #[test]
    fn test_breaking_all_types() {
        assert_eq!(determine_bump("fix!: breaking fix"), BumpType::Major);
        assert_eq!(determine_bump("refactor!: breaking"), BumpType::Major);
        assert_eq!(determine_bump("perf!: breaking"), BumpType::Major);
        assert_eq!(determine_bump("chore!: breaking"), BumpType::Major);
        assert_eq!(determine_bump("docs!: breaking"), BumpType::Major);
        assert_eq!(determine_bump("style!: breaking"), BumpType::Major);
        assert_eq!(determine_bump("test!: breaking"), BumpType::Major);
        assert_eq!(determine_bump("build!: breaking"), BumpType::Major);
        assert_eq!(determine_bump("ci!: breaking"), BumpType::Major);
    }

    #[test]
    fn test_breaking_with_scope() {
        assert_eq!(determine_bump("chore(deps)!: breaking"), BumpType::Major);
        assert_eq!(determine_bump("build(npm)!: breaking"), BumpType::Major);
    }

    #[test]
    fn test_breaking_change_in_body_multiline() {
        let msg = "feat: add feature\n\nSome description.\n\nBREAKING CHANGE: removed old API";
        assert_eq!(determine_bump(msg), BumpType::Major);
    }

    #[test]
    fn test_parse_subject_multiline() {
        assert_eq!(
            parse_subject("first line\nsecond line\nthird line"),
            "first line"
        );
    }

    #[test]
    fn test_parse_subject_empty() {
        assert_eq!(parse_subject(""), "");
    }

    #[test]
    fn test_bump_type_display() {
        assert_eq!(format!("{}", BumpType::None), "none");
        assert_eq!(format!("{}", BumpType::Patch), "patch");
        assert_eq!(format!("{}", BumpType::Minor), "minor");
        assert_eq!(format!("{}", BumpType::Major), "major");
    }

    #[test]
    fn test_feat_not_in_middle_of_word() {
        assert_eq!(determine_bump("featured something"), BumpType::None);
    }

    #[test]
    fn test_deep_nested_scope() {
        assert_eq!(
            determine_bump("feat(api/auth/jwt): add token"),
            BumpType::Minor
        );
        assert_eq!(
            determine_bump("fix(ui/modal): close on escape"),
            BumpType::Patch
        );
    }

    #[test]
    fn test_uppercase_types_not_matched() {
        assert_eq!(determine_bump("FEAT: add login"), BumpType::None);
        assert_eq!(determine_bump("FIX: bug"), BumpType::None);
        assert_eq!(determine_bump("Feat: add login"), BumpType::None);
    }

    #[test]
    fn test_missing_colon() {
        assert_eq!(determine_bump("feat add login"), BumpType::None);
        assert_eq!(determine_bump("fix something"), BumpType::None);
    }

    #[test]
    fn test_extra_space_after_type() {
        assert_eq!(determine_bump("feat : add login"), BumpType::None);
    }

    #[test]
    fn test_empty_scope() {
        assert_eq!(determine_bump("feat(): add login"), BumpType::None);
        assert_eq!(determine_bump("fix(): bug"), BumpType::None);
    }

    #[test]
    fn test_breaking_change_not_at_line_start() {
        let msg = "feat: something\n\nnot a BREAKING CHANGE here";
        assert_eq!(determine_bump(msg), BumpType::Minor);
    }

    #[test]
    fn test_parse_subject_crlf() {
        assert_eq!(parse_subject("feat: add\r\nbody text"), "feat: add");
    }

    #[test]
    fn test_parse_subject_only_newlines() {
        assert_eq!(parse_subject("\n\n\n"), "");
    }

    #[test]
    fn test_multiline_body_feat_in_body_does_not_match() {
        let msg = "chore: update deps\n\nfeat: this is in the body";
        assert_eq!(determine_bump(msg), BumpType::None);
    }

    #[test]
    fn test_multiline_body_fix_in_body_does_not_match() {
        let msg = "chore: update deps\n\nfix: this is in the body";
        assert_eq!(determine_bump(msg), BumpType::None);
    }

    #[test]
    fn test_multiline_body_breaking_marker_in_body_does_not_match() {
        let msg = "chore: update deps\n\nfeat!: this is in the body";
        assert_eq!(determine_bump(msg), BumpType::None);
    }
}
