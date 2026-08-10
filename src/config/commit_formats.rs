use serde::{Deserialize, Serialize};

pub const CATCH_ALL: &str = "all";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PatternSet {
    One(String),
    Many(Vec<String>),
}

impl PatternSet {
    fn patterns(&self) -> &[String] {
        match self {
            PatternSet::One(p) => std::slice::from_ref(p),
            PatternSet::Many(p) => p,
        }
    }

    // `all` is a keyword, not a pattern, so it is recognised whatever the
    // casing — `case_sensitive` governs how subjects are matched, and letting
    // it turn `"All"` into a literal that matches only the subject "all"
    // would be a trap.
    fn is_catch_all(&self) -> bool {
        self.patterns()
            .iter()
            .any(|p| p.eq_ignore_ascii_case(CATCH_ALL))
    }

    pub fn matches(&self, subject: &str, case_sensitive: bool) -> bool {
        if self.is_catch_all() {
            return true;
        }
        let haystack = if case_sensitive {
            subject.to_string()
        } else {
            subject.to_lowercase()
        };
        self.patterns().iter().any(|p| {
            let pattern = if case_sensitive {
                p.clone()
            } else {
                p.to_lowercase()
            };
            wildcard_match(&pattern, &haystack)
        })
    }
}

/// `*` (any run, possibly empty) and `?` (exactly one char) over a commit
/// subject.
///
/// Deliberately not `glob_match`, which the rest of the codebase uses for
/// branch names: there `/` is a real separator and `*` must not cross it.
/// A commit subject is prose — `fix: update src/foo.rs` and
/// `feat(api/auth/jwt): x` both contain slashes — so path semantics would
/// make `fix:*` fail on the majority of real messages (#247).
fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut backtrack) = (None, 0usize);

    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            backtrack = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            backtrack += 1;
            ti = backtrack;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

impl From<Vec<&str>> for PatternSet {
    fn from(v: Vec<&str>) -> Self {
        PatternSet::Many(v.into_iter().map(String::from).collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CommitFormats {
    #[serde(default = "default_major")]
    pub major: PatternSet,
    #[serde(default = "default_minor")]
    pub minor: PatternSet,
    #[serde(default = "default_patch")]
    pub patch: PatternSet,
    #[serde(default = "default_case_sensitive", alias = "caseSensitive")]
    pub case_sensitive: bool,
}

fn default_case_sensitive() -> bool {
    true
}

/// Empty on purpose. Every real breaking marker — `feat!:`, `fix(api)!:`,
/// the `feat(api!):` typo, and a `BREAKING CHANGE:` footer — is detected
/// structurally in `classify_commit`, whatever is configured here. A glob
/// cannot express those precisely: `*!:*` matches any subject containing
/// `!:` anywhere, which turns `fix: handle the !: token in the parser`
/// into a major release. This level exists for teams whose breaking
/// convention is something else entirely, e.g. `"major": "Breaking/*"`.
fn default_major() -> PatternSet {
    PatternSet::Many(Vec::new())
}

fn default_minor() -> PatternSet {
    vec![
        "feat:*",
        "feat(?*):*",
        "Feat:*",
        "Feat(?*):*",
        "Feat/*",
        "feat/*",
        "feature:*",
        "feature(?*):*",
        "Feature/*",
        "feature/*",
    ]
    .into()
}

fn default_patch() -> PatternSet {
    vec![
        "fix:*",
        "fix(?*):*",
        "Fix:*",
        "Fix(?*):*",
        "Fix/*",
        "fix/*",
        "perf:*",
        "perf(?*):*",
        "Perf:*",
        "Perf(?*):*",
        "refactor:*",
        "refactor(?*):*",
        "Refactor:*",
        "Refactor(?*):*",
        "Refactor/*",
        "refactor/*",
    ]
    .into()
}

impl Default for CommitFormats {
    fn default() -> Self {
        Self {
            major: default_major(),
            minor: default_minor(),
            patch: default_patch(),
            case_sensitive: default_case_sensitive(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> CommitFormats {
        CommitFormats::default()
    }

    #[test]
    fn single_string_pattern_is_accepted() {
        let p: PatternSet = serde_json::from_str(r#""feat:*""#).unwrap();
        assert!(p.matches("feat: add login", true));
        assert!(!p.matches("fix: crash", true));
    }

    #[test]
    fn list_of_patterns_is_accepted() {
        let p: PatternSet = serde_json::from_str(r#"["feat:*", "Feat/*"]"#).unwrap();
        assert!(p.matches("feat: x", true));
        assert!(p.matches("Feat/add-login", true));
        assert!(!p.matches("chore: x", true));
    }

    #[test]
    fn catch_all_matches_anything() {
        let p: PatternSet = serde_json::from_str(r#""all""#).unwrap();
        assert!(p.matches("literally anything", true));
        assert!(p.matches("", true));
    }

    #[test]
    fn catch_all_is_a_keyword_not_a_pattern_so_casing_is_irrelevant() {
        for raw in [r#""All""#, r#""ALL""#, r#""all""#] {
            let p: PatternSet = serde_json::from_str(raw).unwrap();
            assert!(p.matches("chore: whatever", true), "{raw} should catch all");
            assert!(
                p.matches("chore: whatever", false),
                "{raw} should catch all"
            );
        }
    }

    #[test]
    fn catch_all_inside_a_list_still_catches_all() {
        let p: PatternSet = serde_json::from_str(r#"["feat:*", "all"]"#).unwrap();
        assert!(p.matches("chore: whatever", true));
    }

    #[test]
    fn case_sensitive_by_default_rejects_wrong_casing() {
        let p: PatternSet = serde_json::from_str(r#""feat:*""#).unwrap();
        assert!(!p.matches("FEAT: shouting", true));
        assert!(p.matches("FEAT: shouting", false));
    }

    #[test]
    fn case_insensitive_lowercases_both_sides() {
        let p: PatternSet = serde_json::from_str(r#""Feat/*""#).unwrap();
        assert!(p.matches("FEAT/add-login", false));
        assert!(p.matches("feat/add-login", false));
    }

    #[test]
    fn defaults_cover_the_branch_style_prefixes_the_issue_asked_for() {
        let f = defaults();
        for subject in [
            "Feat/add-login",
            "Fix/resolve-crash",
            "Refactor/cleanup",
            "Feat: add login",
            "Fix: resolve crash",
            "feature: add login",
        ] {
            let hit = f.major.matches(subject, f.case_sensitive)
                || f.minor.matches(subject, f.case_sensitive)
                || f.patch.matches(subject, f.case_sensitive);
            assert!(hit, "default patterns should match {subject:?}");
        }
    }

    #[test]
    fn defaults_still_ignore_non_release_conventional_types() {
        let f = defaults();
        for subject in ["chore: deps", "docs: typo", "ci: cache", "test: add case"] {
            let hit = f.major.matches(subject, f.case_sensitive)
                || f.minor.matches(subject, f.case_sensitive)
                || f.patch.matches(subject, f.case_sensitive);
            assert!(!hit, "{subject:?} must not trigger a release");
        }
    }

    #[test]
    fn partial_config_falls_back_to_defaults_per_level() {
        let f: CommitFormats = serde_json::from_str(r#"{"minor": "all"}"#).unwrap();
        assert!(f.minor.matches("anything", true));
        assert!(f.patch.matches("fix: x", true), "patch keeps its default");
        assert!(f.case_sensitive, "case_sensitive defaults to true");
    }

    #[test]
    fn case_sensitive_accepts_both_snake_and_camel_spelling() {
        let a: CommitFormats = serde_json::from_str(r#"{"case_sensitive": false}"#).unwrap();
        let b: CommitFormats = serde_json::from_str(r#"{"caseSensitive": false}"#).unwrap();
        assert!(!a.case_sensitive);
        assert!(!b.case_sensitive);
    }
}
