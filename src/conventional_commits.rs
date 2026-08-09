use regex::Regex;
use std::sync::OnceLock;

use crate::config::CommitFormats;

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

fn breaking_header_re() -> &'static Regex {
    BREAKING_RE.get_or_init(|| {
        Regex::new(r"^(feat|fix|refactor|perf|build|chore|docs|style|test|ci)(\(.+\))?!:").unwrap()
    })
}

static BREAKING_FOOTER_RE: OnceLock<Regex> = OnceLock::new();

// Case-insensitive so `breaking-change:` / `Breaking Change:` are caught
// alongside the spec's uppercase form, but the structural rules stay
// strict: the token must be at a line start, use a single space or hyphen,
// and be followed by a colon-space. Prose mentions ("a breaking change in
// the API") and the plural ("BREAKING CHANGES:") therefore never match.
fn breaking_footer_re() -> &'static Regex {
    BREAKING_FOOTER_RE.get_or_init(|| Regex::new(r"(?mi)^BREAKING[ -]CHANGE: ").unwrap())
}

static BREAKING_SCOPE_BANG_RE: OnceLock<Regex> = OnceLock::new();

// A `!` placed inside the scope (`feat(api!):`) instead of after it
// (`feat(api)!:`) is a common typo; treat it as breaking too. The bang
// must sit immediately before the closing paren, so `feat(a!b):` is not
// a breaking marker.
fn breaking_scope_bang_re() -> &'static Regex {
    BREAKING_SCOPE_BANG_RE.get_or_init(|| {
        Regex::new(r"^(feat|fix|refactor|perf|build|chore|docs|style|test|ci)\([^()]*!\):").unwrap()
    })
}

pub fn determine_bump(message: &str, formats: &CommitFormats) -> BumpType {
    match classify_commit(message, formats) {
        CommitCategory::Breaking => BumpType::Major,
        CommitCategory::Feature => BumpType::Minor,
        CommitCategory::Fix | CommitCategory::Refactor => BumpType::Patch,
        CommitCategory::Other => BumpType::None,
    }
}

/// Author-facing categorization of a single commit. Drives both the
/// version bump (via [`determine_bump`]) and changelog section grouping,
/// so the two can't disagree the way they used to. See #525.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitCategory {
    /// `feat!:`, `fix(scope)!:`, etc. or a `BREAKING CHANGE:` footer.
    Breaking,
    /// `feat:` or `feat(scope):` (and not breaking).
    Feature,
    /// `fix:` / `perf:` — patch-bumping bug or performance changes.
    Fix,
    /// `refactor:` — patch-bumping internal restructuring. Distinct
    /// from `Fix` so the changelog can render it as its own section
    /// (the prior code dropped refactor commits entirely from the
    /// changelog while still letting them trigger a release).
    Refactor,
    /// `chore:` / `docs:` / `ci:` / `style:` / `test:` / `build:` or any
    /// non-conventional message. Doesn't bump and shouldn't appear in
    /// the user-facing changelog.
    Other,
}

pub fn classify_commit(message: &str, formats: &CommitFormats) -> CommitCategory {
    let subject = parse_subject(message);
    let cs = formats.case_sensitive;

    // Structural breaking markers are recognised whatever the configured
    // patterns say. A `BREAKING CHANGE:` footer lives in the body, which
    // subject globs cannot see, and `feat(api!):` puts the bang where a
    // `*!:*` glob does not reach.
    if breaking_header_re().is_match(subject)
        || breaking_scope_bang_re().is_match(subject)
        || breaking_footer_re().is_match(message)
        || formats.major.matches(subject, cs)
    {
        return CommitCategory::Breaking;
    }
    if formats.minor.matches(subject, cs) {
        return CommitCategory::Feature;
    }
    if formats.patch.matches(subject, cs) {
        // Patch splits into two changelog sections. The configured patterns
        // only carry a bump level, so the section is resolved from the
        // conventional prefix when there is one, and falls back to Fix.
        return if refactor_header_re().is_match(subject) {
            CommitCategory::Refactor
        } else {
            CommitCategory::Fix
        };
    }
    CommitCategory::Other
}

static REFACTOR_RE: OnceLock<Regex> = OnceLock::new();

fn refactor_header_re() -> &'static Regex {
    REFACTOR_RE.get_or_init(|| Regex::new(r"^refactor(\(.+\))?:").unwrap())
}

pub fn parse_subject(message: &str) -> &str {
    message.lines().next().unwrap_or("").trim()
}

static HEADER_RE: OnceLock<Regex> = OnceLock::new();

fn header_re() -> &'static Regex {
    HEADER_RE.get_or_init(|| {
        Regex::new(r"^(?P<type>[a-z]+)(?:\((?P<scope>[^()]+)\))?(?P<bang>!)?:\s*(?P<desc>.*)$")
            .unwrap()
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHeader<'a> {
    pub commit_type: &'a str,
    pub scope: Option<&'a str>,
    pub breaking_bang: bool,
    pub description: &'a str,
}

pub fn parse_header(message: &str) -> Option<ParsedHeader<'_>> {
    let subject = parse_subject(message);
    let caps = header_re().captures(subject)?;
    // A `!` at the end of the scope (`feat(api!):`) is a breaking marker,
    // not part of the scope name — surface it as `breaking_bang` and hand
    // callers the clean scope so the changelog groups under `api`, not `api!`.
    let raw_scope = caps.name("scope").map(|m| m.as_str());
    let scope_bang = raw_scope.is_some_and(|s| s.ends_with('!'));
    let scope = raw_scope
        .map(|s| s.strip_suffix('!').unwrap_or(s))
        .filter(|s| !s.is_empty());
    Some(ParsedHeader {
        commit_type: caps.name("type")?.as_str(),
        scope,
        breaking_bang: caps.name("bang").is_some() || scope_bang,
        description: caps.name("desc").map(|m| m.as_str()).unwrap_or(""),
    })
}

pub fn is_breaking(message: &str, formats: &CommitFormats) -> bool {
    matches!(classify_commit(message, formats), CommitCategory::Breaking)
}

pub fn breaking_footer_body(message: &str) -> Option<String> {
    let re = breaking_footer_re();
    let mat = re.find(message)?;
    let body = message[mat.end()..].trim();
    if body.is_empty() {
        None
    } else {
        Some(body.lines().next().unwrap_or("").trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CATCH_ALL, PatternSet};

    /// The permissive defaults are a deliberate behaviour change (#247), so
    /// the documented escape hatch has to actually work: with strict
    /// patterns every branch-style and capitalised variant goes back to
    /// being ignored, and plain conventional commits are unaffected.
    #[test]
    fn strict_patterns_restore_the_pre_247_behaviour() {
        let strict = CommitFormats {
            major: PatternSet::Many(Vec::new()),
            minor: vec!["feat:*", "feat(?*):*"].into(),
            patch: vec![
                "fix:*",
                "fix(?*):*",
                "perf:*",
                "perf(?*):*",
                "refactor:*",
                "refactor(?*):*",
            ]
            .into(),
            case_sensitive: true,
        };
        for ignored in [
            "Feat/add-login",
            "Fix/resolve-crash",
            "Refactor/cleanup",
            "Feat: add login",
            "Fix: resolve crash",
            "feature: add login",
        ] {
            assert_eq!(
                determine_bump(ignored, &strict),
                BumpType::None,
                "{ignored:?} must not bump under the strict preset"
            );
        }
        assert_eq!(determine_bump("feat: x", &strict), BumpType::Minor);
        assert_eq!(determine_bump("fix(db): x", &strict), BumpType::Patch);
        assert_eq!(determine_bump("refactor: x", &strict), BumpType::Patch);
        assert_eq!(determine_bump("chore: x", &strict), BumpType::None);
        assert_eq!(determine_bump("feat!: x", &strict), BumpType::Major);
        assert_eq!(
            determine_bump("fix: x\n\nBREAKING CHANGE: gone", &strict),
            BumpType::Major
        );
    }

    /// The structural breaking markers must survive whatever patterns are
    /// configured — a `BREAKING CHANGE:` footer lives in the body, which a
    /// subject glob cannot see, and `feat(api!):` puts the bang where
    /// `*!:*` does not reach.
    #[test]
    fn structural_breaking_markers_survive_a_config_that_omits_them() {
        let no_major = CommitFormats {
            major: PatternSet::Many(Vec::new()),
            minor: vec!["feat:*"].into(),
            patch: vec!["fix:*"].into(),
            case_sensitive: true,
        };
        assert_eq!(
            determine_bump("fix: x\n\nBREAKING CHANGE: gone", &no_major),
            BumpType::Major,
            "footer must still win"
        );
        assert_eq!(
            determine_bump("feat(api!): typo bang", &no_major),
            BumpType::Major,
            "scope-bang typo must still win"
        );
        assert_eq!(determine_bump("feat!: x", &no_major), BumpType::Major);
    }

    #[test]
    fn permissive_defaults_pick_up_branch_style_prefixes() {
        let d = CommitFormats::default();
        assert_eq!(determine_bump("Feat/add-login", &d), BumpType::Minor);
        assert_eq!(determine_bump("Fix/resolve-crash", &d), BumpType::Patch);
        assert_eq!(determine_bump("Refactor/cleanup", &d), BumpType::Patch);
        assert_eq!(determine_bump("Feat: add login", &d), BumpType::Minor);
        assert_eq!(determine_bump("feature: add login", &d), BumpType::Minor);
        assert_eq!(determine_bump("chore: deps", &d), BumpType::None);
    }

    #[test]
    fn catch_all_patch_makes_every_commit_release() {
        let f = CommitFormats {
            major: PatternSet::Many(Vec::new()),
            minor: vec!["feat:*"].into(),
            patch: PatternSet::One(CATCH_ALL.to_string()),
            case_sensitive: true,
        };
        assert_eq!(determine_bump("anything at all", &f), BumpType::Patch);
        assert_eq!(determine_bump("chore: deps", &f), BumpType::Patch);
        assert_eq!(determine_bump("feat: x", &f), BumpType::Minor);
        assert_eq!(determine_bump("feat!: x", &f), BumpType::Major);
    }

    #[test]
    fn priority_is_major_over_minor_over_patch() {
        let f = CommitFormats {
            major: PatternSet::One(CATCH_ALL.to_string()),
            minor: PatternSet::One(CATCH_ALL.to_string()),
            patch: PatternSet::One(CATCH_ALL.to_string()),
            case_sensitive: true,
        };
        assert_eq!(determine_bump("whatever", &f), BumpType::Major);
    }

    #[test]
    fn case_insensitive_config_accepts_any_casing() {
        let f = CommitFormats {
            major: PatternSet::Many(Vec::new()),
            minor: vec!["feat:*", "feat(?*):*"].into(),
            patch: vec!["fix:*", "fix(?*):*"].into(),
            case_sensitive: false,
        };
        assert_eq!(determine_bump("FEAT: shouting", &f), BumpType::Minor);
        assert_eq!(determine_bump("Fix(db): leak", &f), BumpType::Patch);
    }

    /// Configured patterns carry a bump level, not a changelog section, so
    /// a patch-level match still has to land in the right section.
    #[test]
    fn patch_level_splits_into_fix_and_refactor_sections() {
        let d = CommitFormats::default();
        assert_eq!(classify_commit("refactor: x", &d), CommitCategory::Refactor);
        assert_eq!(classify_commit("fix: x", &d), CommitCategory::Fix);
        assert_eq!(classify_commit("perf: x", &d), CommitCategory::Fix);
        assert_eq!(
            classify_commit("Fix/branch-style", &d),
            CommitCategory::Fix,
            "non-conventional patch matches fall back to Fix"
        );
    }

    #[test]
    fn test_patch() {
        assert_eq!(
            determine_bump("fix: correct typo", &Default::default()),
            BumpType::Patch
        );
        assert_eq!(
            determine_bump("perf: faster query", &Default::default()),
            BumpType::Patch
        );
        assert_eq!(
            determine_bump("refactor: clean up", &Default::default()),
            BumpType::Patch
        );
    }

    #[test]
    fn test_minor() {
        assert_eq!(
            determine_bump("feat: add login", &Default::default()),
            BumpType::Minor
        );
        assert_eq!(
            determine_bump("feat(auth): add JWT", &Default::default()),
            BumpType::Minor
        );
    }

    #[test]
    fn test_major() {
        assert_eq!(
            determine_bump("feat!: breaking change", &Default::default()),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("fix(api)!: remove endpoint", &Default::default()),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("BREAKING CHANGE: removed X", &Default::default()),
            BumpType::Major
        );
    }

    #[test]
    fn test_none() {
        assert_eq!(
            determine_bump("chore: update deps", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump("docs: update readme", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump("ci: fix pipeline", &Default::default()),
            BumpType::None
        );
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
        assert_eq!(
            determine_bump("fix(api): null check", &Default::default()),
            BumpType::Patch
        );
        assert_eq!(
            determine_bump("feat(ui): new button", &Default::default()),
            BumpType::Minor
        );
        assert_eq!(
            determine_bump("refactor(db): simplify", &Default::default()),
            BumpType::Patch
        );
    }

    #[test]
    fn test_breaking_change_in_body() {
        let msg = "feat: something\n\nBREAKING CHANGE: removed old API";
        assert_eq!(determine_bump(msg, &Default::default()), BumpType::Major);
    }

    #[test]
    fn test_breaking_change_hyphen_footer() {
        let msg = "feat: something\n\nBREAKING-CHANGE: removed old API";
        assert_eq!(determine_bump(msg, &Default::default()), BumpType::Major);
    }

    #[test]
    fn test_breaking_change_prose_is_not_major() {
        assert_eq!(
            determine_bump(
                "docs: note that BREAKING CHANGES are coming in v2",
                &Default::default()
            ),
            BumpType::None
        );
        let body = "feat: add flag\n\nBREAKING CHANGE will be handled later, not yet";
        assert_eq!(determine_bump(body, &Default::default()), BumpType::Minor);
        let plural = "chore: cleanup\n\nBREAKING CHANGES: none in this one";
        assert_eq!(determine_bump(plural, &Default::default()), BumpType::None);
    }

    #[test]
    fn test_breaking_change_footer_missing_space_after_colon() {
        let msg = "feat: x\n\nBREAKING CHANGE:no-space-description";
        assert_eq!(determine_bump(msg, &Default::default()), BumpType::Minor);
    }

    #[test]
    fn test_bump_ordering() {
        assert!(BumpType::Major > BumpType::Minor);
        assert!(BumpType::Minor > BumpType::Patch);
        assert!(BumpType::Patch > BumpType::None);
    }

    #[test]
    fn test_empty_message() {
        assert_eq!(determine_bump("", &Default::default()), BumpType::None);
    }

    #[test]
    fn test_whitespace_only_message() {
        assert_eq!(
            determine_bump("   \n\n  ", &Default::default()),
            BumpType::None
        );
    }

    #[test]
    fn test_non_conventional_message() {
        assert_eq!(
            determine_bump("update readme", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump("fixed the thing", &Default::default()),
            BumpType::None
        );
        assert_eq!(determine_bump("WIP", &Default::default()), BumpType::None);
    }

    #[test]
    fn test_all_patch_types() {
        assert_eq!(
            determine_bump("fix: something", &Default::default()),
            BumpType::Patch
        );
        assert_eq!(
            determine_bump("perf: something", &Default::default()),
            BumpType::Patch
        );
        assert_eq!(
            determine_bump("refactor: something", &Default::default()),
            BumpType::Patch
        );
    }

    #[test]
    fn test_all_none_types() {
        assert_eq!(
            determine_bump("chore: something", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump("docs: something", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump("ci: something", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump("style: something", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump("test: something", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump("build: something", &Default::default()),
            BumpType::None
        );
    }

    #[test]
    fn test_breaking_all_types() {
        assert_eq!(
            determine_bump("fix!: breaking fix", &Default::default()),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("refactor!: breaking", &Default::default()),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("perf!: breaking", &Default::default()),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("chore!: breaking", &Default::default()),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("docs!: breaking", &Default::default()),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("style!: breaking", &Default::default()),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("test!: breaking", &Default::default()),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("build!: breaking", &Default::default()),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("ci!: breaking", &Default::default()),
            BumpType::Major
        );
    }

    #[test]
    fn test_breaking_with_scope() {
        assert_eq!(
            determine_bump("chore(deps)!: breaking", &Default::default()),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("build(npm)!: breaking", &Default::default()),
            BumpType::Major
        );
    }

    #[test]
    fn test_breaking_change_in_body_multiline() {
        let msg = "feat: add feature\n\nSome description.\n\nBREAKING CHANGE: removed old API";
        assert_eq!(determine_bump(msg, &Default::default()), BumpType::Major);
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
        assert_eq!(
            determine_bump("featured something", &Default::default()),
            BumpType::None
        );
    }

    #[test]
    fn test_deep_nested_scope() {
        assert_eq!(
            determine_bump("feat(api/auth/jwt): add token", &Default::default()),
            BumpType::Minor
        );
        assert_eq!(
            determine_bump("fix(ui/modal): close on escape", &Default::default()),
            BumpType::Patch
        );
    }

    /// All-caps stays unmatched under the default patterns; only the
    /// Title-case variants were added by #247.
    #[test]
    fn test_uppercase_types_not_matched() {
        assert_eq!(
            determine_bump("FEAT: add login", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump("FIX: bug", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump("Feat: add login", &Default::default()),
            BumpType::Minor
        );
    }

    #[test]
    fn test_missing_colon() {
        assert_eq!(
            determine_bump("feat add login", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump("fix something", &Default::default()),
            BumpType::None
        );
    }

    #[test]
    fn test_extra_space_after_type() {
        assert_eq!(
            determine_bump("feat : add login", &Default::default()),
            BumpType::None
        );
    }

    #[test]
    fn test_empty_scope() {
        assert_eq!(
            determine_bump("feat(): add login", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump("fix(): bug", &Default::default()),
            BumpType::None
        );
    }

    #[test]
    fn test_breaking_change_not_at_line_start() {
        let msg = "feat: something\n\nnot a BREAKING CHANGE here";
        assert_eq!(determine_bump(msg, &Default::default()), BumpType::Minor);
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
        assert_eq!(determine_bump(msg, &Default::default()), BumpType::None);
    }

    #[test]
    fn test_multiline_body_fix_in_body_does_not_match() {
        let msg = "chore: update deps\n\nfix: this is in the body";
        assert_eq!(determine_bump(msg, &Default::default()), BumpType::None);
    }

    #[test]
    fn test_multiline_body_breaking_marker_in_body_does_not_match() {
        let msg = "chore: update deps\n\nfeat!: this is in the body";
        assert_eq!(determine_bump(msg, &Default::default()), BumpType::None);
    }

    #[test]
    fn test_parse_header_type_scope_desc() {
        let h = parse_header("feat(api): add events endpoint").unwrap();
        assert_eq!(h.commit_type, "feat");
        assert_eq!(h.scope, Some("api"));
        assert!(!h.breaking_bang);
        assert_eq!(h.description, "add events endpoint");
    }

    #[test]
    fn test_parse_header_breaking_bang() {
        let h = parse_header("feat!: drop flag").unwrap();
        assert_eq!(h.commit_type, "feat");
        assert_eq!(h.scope, None);
        assert!(h.breaking_bang);
        assert_eq!(h.description, "drop flag");
    }

    #[test]
    fn test_parse_header_scoped_bang() {
        let h = parse_header("fix(security)!: patch").unwrap();
        assert_eq!(h.commit_type, "fix");
        assert_eq!(h.scope, Some("security"));
        assert!(h.breaking_bang);
    }

    #[test]
    fn test_parse_header_non_conventional() {
        assert!(parse_header("just a message").is_none());
        assert!(parse_header("feat add no colon").is_none());
    }

    #[test]
    fn test_breaking_footer_body_extracts_description() {
        let msg = "feat: add x\n\nBREAKING CHANGE: the old endpoint is gone";
        assert_eq!(
            breaking_footer_body(msg).as_deref(),
            Some("the old endpoint is gone")
        );
    }

    #[test]
    fn test_breaking_footer_body_none_when_absent() {
        assert_eq!(breaking_footer_body("feat: add x"), None);
        assert_eq!(breaking_footer_body("feat!: add x"), None);
    }

    #[test]
    fn test_is_breaking() {
        assert!(is_breaking("feat!: x", &Default::default()));
        assert!(is_breaking(
            "feat: x\n\nBREAKING CHANGE: y",
            &Default::default()
        ));
        assert!(!is_breaking("feat: x", &Default::default()));
        assert!(!is_breaking("fix: y", &Default::default()));
    }

    #[test]
    fn breaking_footer_case_and_hyphen_variants() {
        for footer in [
            "BREAKING CHANGE: gone",
            "BREAKING-CHANGE: gone",
            "breaking-change: gone",
            "breaking change: gone",
            "Breaking Change: gone",
        ] {
            let msg = format!("feat: x\n\n{footer}");
            assert_eq!(
                determine_bump(&msg, &Default::default()),
                BumpType::Major,
                "footer: {footer:?}"
            );
        }
    }

    #[test]
    fn breaking_footer_stays_strict_on_malformed_shapes() {
        // Missing the colon-space, plural, prose, and mid-line placement must
        // not trip the detector — we accept case variants, not any shape.
        assert_eq!(
            determine_bump("feat: x\n\nBreaking change:nospace", &Default::default()),
            BumpType::Minor
        );
        assert_eq!(
            determine_bump("chore: x\n\nbreaking changes: none", &Default::default()),
            BumpType::None
        );
        assert_eq!(
            determine_bump(
                "feat: x\n\nthis is a breaking change: really",
                &Default::default()
            ),
            BumpType::Minor
        );
    }

    #[test]
    fn bang_inside_scope_is_breaking() {
        assert_eq!(
            determine_bump("feat(api!): remove endpoint", &Default::default()),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("fix(db!): drop table", &Default::default()),
            BumpType::Major
        );
        // Bang not immediately before the closing paren is not a marker.
        assert_eq!(
            determine_bump("feat(a!b): middle", &Default::default()),
            BumpType::Minor
        );
    }

    #[test]
    fn parse_header_normalizes_scope_internal_bang() {
        let h = parse_header("feat(api!): remove endpoint").unwrap();
        assert_eq!(h.commit_type, "feat");
        assert_eq!(
            h.scope,
            Some("api"),
            "the trailing ! is stripped from scope"
        );
        assert!(h.breaking_bang);
        let empty = parse_header("feat(!): drop flag").unwrap();
        assert_eq!(empty.scope, None);
        assert!(empty.breaking_bang);
    }

    #[test]
    fn fixtures_classify_by_directory() {
        use std::path::Path;
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/conventional_commits");
        let mut checked = 0;
        for (dir, expect_breaking) in [("breaking", true), ("not_breaking", false)] {
            let subdir = root.join(dir);
            let entries =
                std::fs::read_dir(&subdir).unwrap_or_else(|e| panic!("read {subdir:?}: {e}"));
            for entry in entries {
                let path = entry.unwrap().path();
                if path.extension().and_then(|e| e.to_str()) != Some("txt") {
                    continue;
                }
                let message = std::fs::read_to_string(&path).unwrap();
                assert_eq!(
                    is_breaking(&message, &Default::default()),
                    expect_breaking,
                    "fixture {:?} should classify breaking={expect_breaking}",
                    path.file_name().unwrap()
                );
                checked += 1;
            }
        }
        assert!(
            checked >= 14,
            "fixture corpus looks unloaded — only {checked} messages checked"
        );
    }
}
