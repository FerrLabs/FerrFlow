use std::path::Path;

use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct HookCommit {
    pub hash: String,
    pub message: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub commit_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub breaking: bool,
}

impl HookCommit {
    pub fn from_commit(hash: &str, message: &str) -> Self {
        let header = crate::conventional_commits::parse_header(message);
        HookCommit {
            hash: hash.to_string(),
            message: crate::conventional_commits::parse_subject(message).to_string(),
            commit_type: header.as_ref().map(|h| h.commit_type.to_string()),
            scope: header.as_ref().and_then(|h| h.scope.map(str::to_string)),
            breaking: crate::conventional_commits::is_breaking(message),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct HookFile {
    pub path: String,
    pub format: String,
}

#[derive(Clone)]
pub struct HookContext {
    pub package: String,
    pub old_version: String,
    pub new_version: String,
    pub bump_type: String,
    pub tag: String,
    pub dry_run: bool,
    pub package_path: String,
    pub channel: Option<String>,
    pub error_code: Option<String>,
    pub monorepo: bool,
    pub is_prerelease: bool,
    pub changelog: String,
    pub commits: Vec<HookCommit>,
    pub bumped_files: Vec<HookFile>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_commit_extracts_type_scope_and_breaking() {
        let c = HookCommit::from_commit("abc1234", "feat(api): add endpoint");
        assert_eq!(c.hash, "abc1234");
        assert_eq!(c.message, "feat(api): add endpoint");
        assert_eq!(c.commit_type.as_deref(), Some("feat"));
        assert_eq!(c.scope.as_deref(), Some("api"));
        assert!(!c.breaking);

        let breaking = HookCommit::from_commit("d", "feat!: drop the old flag");
        assert!(breaking.breaking);
        assert_eq!(breaking.scope, None);

        let footer = HookCommit::from_commit("e", "fix: x\n\nBREAKING CHANGE: gone");
        assert!(footer.breaking);
        // The serialized message is the subject only, not the footer.
        assert_eq!(footer.message, "fix: x");

        let plain = HookCommit::from_commit("f", "just a plain message");
        assert_eq!(plain.commit_type, None);
        assert_eq!(plain.scope, None);
        assert!(!plain.breaking);
    }

    #[test]
    fn hook_commit_json_omits_absent_type_and_scope() {
        let json = serde_json::to_string(&HookCommit::from_commit("h", "chore: bump")).unwrap();
        // `type` present (chore), `scope` absent, `breaking` always present.
        assert!(json.contains(r#""type":"chore""#));
        assert!(!json.contains("scope"));
        assert!(json.contains(r#""breaking":false"#));
    }
}

impl HookContext {
    pub fn release_summary(root: &Path, tags: &[String], dry_run: bool, monorepo: bool) -> Self {
        HookContext {
            package: String::new(),
            old_version: String::new(),
            new_version: String::new(),
            bump_type: String::new(),
            tag: tags.join(","),
            dry_run,
            package_path: root.to_string_lossy().into_owned(),
            channel: None,
            error_code: None,
            monorepo,
            is_prerelease: false,
            changelog: String::new(),
            commits: Vec::new(),
            bumped_files: Vec::new(),
        }
    }
}
