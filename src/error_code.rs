use std::fmt;

/// A stable error code that can be attached to any `anyhow::Error` as context.
///
/// Error codes follow the pattern `E{NNNN}` where the first digit(s) indicate
/// the domain and the remaining digits identify the specific error.
///
/// Ranges:
/// - E1000–E1099: Configuration
/// - E1100–E1199: Validation
/// - E2000–E2099: Git operations
/// - E3000–E3099: GitHub API
/// - E3100–E3199: GitLab API
/// - E4000–E4099: Version files (general)
/// - E4100–E4199: TOML version files
/// - E4200–E4299: JSON version files
/// - E4300–E4399: Helm/YAML version files
/// - E4400–E4499: XML/CSProj version files
/// - E4500–E4599: Gradle version files
/// - E4600–E4699: Go mod version files
/// - E4700–E4799: Text version files
/// - E5000–E5099: Pre-release / channels
/// - E5010–E5019: Versioning
/// - E6000–E6099: Hooks
/// - E7000–E7099: Query / package lookup
/// - E8000–E8099: Monorepo operations
#[derive(Debug, Clone, Copy)]
pub struct ErrorCode(pub u16);

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E{:04}", self.0)
    }
}

impl std::error::Error for ErrorCode {}

impl ErrorCode {
    /// Returns the documentation URL for this error code.
    pub fn doc_url(&self) -> String {
        format!(
            "https://ferrflow.com/docs/reference/errors#{}",
            self.to_string().to_lowercase()
        )
    }
}

/// Extension trait to attach an `ErrorCode` as context to any `anyhow::Result`.
#[allow(dead_code)]
pub trait ErrorCodeExt<T> {
    fn error_code(self, code: ErrorCode) -> anyhow::Result<T>;
}

impl<T> ErrorCodeExt<T> for anyhow::Result<T> {
    fn error_code(self, code: ErrorCode) -> anyhow::Result<T> {
        self.map_err(|e| e.context(code))
    }
}

// ── Configuration (E1000–E1099) ──────────────────────────────────────────────
#[allow(dead_code)]
pub const CONFIG_NOT_FOUND: ErrorCode = ErrorCode(1001);
#[allow(dead_code)]
pub const CONFIG_PARSE_JSON: ErrorCode = ErrorCode(1002);
#[allow(dead_code)]
pub const CONFIG_PARSE_JSON5: ErrorCode = ErrorCode(1003);
#[allow(dead_code)]
pub const CONFIG_PARSE_TOML: ErrorCode = ErrorCode(1004);
#[allow(dead_code)]
pub const CONFIG_SERIALIZE_TOML: ErrorCode = ErrorCode(1005);
#[allow(dead_code)]
pub const CONFIG_PARSE_DOTFILE: ErrorCode = ErrorCode(1006);
#[allow(dead_code)]
pub const CONFIG_SERIALIZE_DOTFILE: ErrorCode = ErrorCode(1007);
#[allow(dead_code)]
pub const CONFIG_RESOLVE_PATH: ErrorCode = ErrorCode(1008);
#[allow(dead_code)]
pub const CONFIG_WRITE_LOADER: ErrorCode = ErrorCode(1009);
#[allow(dead_code)]
pub const CONFIG_EVAL_TS: ErrorCode = ErrorCode(1010);
#[allow(dead_code)]
pub const CONFIG_EVAL_NODE: ErrorCode = ErrorCode(1011);
#[allow(dead_code)]
pub const CONFIG_EVAL_FAILED: ErrorCode = ErrorCode(1012);
#[allow(dead_code)]
pub const CONFIG_INVALID_OUTPUT: ErrorCode = ErrorCode(1013);
#[allow(dead_code)]
pub const CONFIG_INVALID_JSON: ErrorCode = ErrorCode(1014);
#[allow(dead_code)]
pub const CONFIG_READ_FAILED: ErrorCode = ErrorCode(1015);
#[allow(dead_code)]
pub const CONFIG_MULTIPLE_FILES: ErrorCode = ErrorCode(1016);
#[allow(dead_code)]
pub const CONFIG_ALREADY_EXISTS: ErrorCode = ErrorCode(1017);

// ── Validation (E1100–E1199) ─────────────────────────────────────────────────
#[allow(dead_code)]
pub const VALIDATE_INVALID_REPO_SPEC: ErrorCode = ErrorCode(1100);
#[allow(dead_code)]
pub const VALIDATE_GITHUB_API: ErrorCode = ErrorCode(1101);
#[allow(dead_code)]
pub const VALIDATE_GITLAB_API: ErrorCode = ErrorCode(1102);
#[allow(dead_code)]
pub const VALIDATE_INVALID_UTF8: ErrorCode = ErrorCode(1103);
#[allow(dead_code)]
pub const VALIDATE_PARSE_FAILED: ErrorCode = ErrorCode(1104);
#[allow(dead_code)]
pub const VALIDATE_FILE_NOT_FOUND: ErrorCode = ErrorCode(1105);
#[allow(dead_code)]
pub const VALIDATE_NO_CONFIG: ErrorCode = ErrorCode(1106);
#[allow(dead_code)]
pub const VALIDATE_REF_REQUIRES_REPO: ErrorCode = ErrorCode(1107);

// ── Git operations (E2000–E2099) ─────────────────────────────────────────────
#[allow(dead_code)]
pub const GIT_NOT_A_REPO: ErrorCode = ErrorCode(2001);
#[allow(dead_code)]
pub const GIT_BARE_REPO: ErrorCode = ErrorCode(2002);
#[allow(dead_code)]
pub const GIT_TAG_EXISTS: ErrorCode = ErrorCode(2003);
#[allow(dead_code)]
pub const GIT_PUSH_BRANCH: ErrorCode = ErrorCode(2004);
#[allow(dead_code)]
pub const GIT_PUSH_REJECTED: ErrorCode = ErrorCode(2005);
#[allow(dead_code)]
pub const GIT_PUSH_TAGS: ErrorCode = ErrorCode(2006);
#[allow(dead_code)]
pub const GIT_FLOATING_TAGS: ErrorCode = ErrorCode(2007);
#[allow(dead_code)]
pub const GIT_REMOTE_NOT_FOUND: ErrorCode = ErrorCode(2008);
#[allow(dead_code)]
pub const GIT_PUSH_VERIFY_FAILED: ErrorCode = ErrorCode(2009);
#[allow(dead_code)]
pub const GIT_REMOTE_BRANCH_NOT_FOUND: ErrorCode = ErrorCode(2010);

// ── GitHub API (E3000–E3099) ─────────────────────────────────────────────────
#[allow(dead_code)]
pub const GITHUB_CREATE_RELEASE: ErrorCode = ErrorCode(3001);
#[allow(dead_code)]
pub const GITHUB_LIST_RELEASES: ErrorCode = ErrorCode(3002);
#[allow(dead_code)]
pub const GITHUB_PARSE_RELEASES: ErrorCode = ErrorCode(3003);
#[allow(dead_code)]
pub const GITHUB_PUBLISH_RELEASE: ErrorCode = ErrorCode(3004);
#[allow(dead_code)]
pub const GITHUB_CREATE_PR: ErrorCode = ErrorCode(3005);
#[allow(dead_code)]
pub const GITHUB_PARSE_PR: ErrorCode = ErrorCode(3006);
#[allow(dead_code)]
pub const GITHUB_PR_MISSING_FIELD: ErrorCode = ErrorCode(3007);
#[allow(dead_code)]
pub const GITHUB_AUTO_MERGE: ErrorCode = ErrorCode(3008);
#[allow(dead_code)]
pub const GITHUB_GRAPHQL_PARSE: ErrorCode = ErrorCode(3009);
#[allow(dead_code)]
pub const GITHUB_AUTO_MERGE_FAILED: ErrorCode = ErrorCode(3010);

// ── GitLab API (E3100–E3199) ─────────────────────────────────────────────────
#[allow(dead_code)]
pub const GITLAB_CREATE_RELEASE: ErrorCode = ErrorCode(3101);
#[allow(dead_code)]
pub const GITLAB_CREATE_MR: ErrorCode = ErrorCode(3102);
#[allow(dead_code)]
pub const GITLAB_PARSE_MR: ErrorCode = ErrorCode(3103);
#[allow(dead_code)]
pub const GITLAB_MR_MISSING_FIELD: ErrorCode = ErrorCode(3104);
#[allow(dead_code)]
pub const GITLAB_MERGE_MR: ErrorCode = ErrorCode(3105);

// ── Version files — TOML (E4100–E4199) ──────────────────────────────────────
#[allow(dead_code)]
pub const TOML_READ: ErrorCode = ErrorCode(4101);
#[allow(dead_code)]
pub const TOML_PARSE: ErrorCode = ErrorCode(4102);
#[allow(dead_code)]
pub const TOML_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4103);
#[allow(dead_code)]
pub const TOML_WRITE: ErrorCode = ErrorCode(4104);
#[allow(dead_code)]
pub const TOML_INVALID_UTF8: ErrorCode = ErrorCode(4105);

// ── Version files — JSON (E4200–E4299) ──────────────────────────────────────
#[allow(dead_code)]
pub const JSON_READ: ErrorCode = ErrorCode(4201);
#[allow(dead_code)]
pub const JSON_PARSE: ErrorCode = ErrorCode(4202);
#[allow(dead_code)]
pub const JSON_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4203);
#[allow(dead_code)]
pub const JSON_WRITE: ErrorCode = ErrorCode(4204);
#[allow(dead_code)]
pub const JSON_INVALID_UTF8: ErrorCode = ErrorCode(4205);

// ── Version files — Helm/YAML (E4300–E4399) ─────────────────────────────────
#[allow(dead_code)]
pub const HELM_READ: ErrorCode = ErrorCode(4301);
#[allow(dead_code)]
pub const HELM_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4302);
#[allow(dead_code)]
pub const HELM_WRITE: ErrorCode = ErrorCode(4303);
#[allow(dead_code)]
pub const HELM_INVALID_UTF8: ErrorCode = ErrorCode(4304);

// ── Version files — XML/CSProj (E4400–E4499) ────────────────────────────────
#[allow(dead_code)]
pub const XML_READ: ErrorCode = ErrorCode(4401);
#[allow(dead_code)]
pub const XML_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4402);
#[allow(dead_code)]
pub const XML_WRITE: ErrorCode = ErrorCode(4403);
#[allow(dead_code)]
pub const XML_INVALID_UTF8: ErrorCode = ErrorCode(4404);
#[allow(dead_code)]
pub const CSPROJ_READ: ErrorCode = ErrorCode(4410);
#[allow(dead_code)]
pub const CSPROJ_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4411);
#[allow(dead_code)]
pub const CSPROJ_WRITE: ErrorCode = ErrorCode(4412);
#[allow(dead_code)]
pub const CSPROJ_INVALID_UTF8: ErrorCode = ErrorCode(4413);

// ── Version files — Gradle (E4500–E4599) ────────────────────────────────────
#[allow(dead_code)]
pub const GRADLE_READ: ErrorCode = ErrorCode(4501);
#[allow(dead_code)]
pub const GRADLE_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4502);
#[allow(dead_code)]
pub const GRADLE_WRITE: ErrorCode = ErrorCode(4503);
#[allow(dead_code)]
pub const GRADLE_INVALID_UTF8: ErrorCode = ErrorCode(4504);

// ── Version files — Go mod (E4600–E4699) ────────────────────────────────────
#[allow(dead_code)]
pub const GOMOD_GIT_DESCRIBE: ErrorCode = ErrorCode(4601);
#[allow(dead_code)]
pub const GOMOD_NO_TAG: ErrorCode = ErrorCode(4602);
#[allow(dead_code)]
pub const GOMOD_UNSUPPORTED: ErrorCode = ErrorCode(4603);

// ── Version files — Text (E4700–E4799) ──────────────────────────────────────
#[allow(dead_code)]
pub const TXT_READ: ErrorCode = ErrorCode(4701);
#[allow(dead_code)]
pub const TXT_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4702);
#[allow(dead_code)]
pub const TXT_WRITE: ErrorCode = ErrorCode(4703);
#[allow(dead_code)]
pub const TXT_INVALID_UTF8: ErrorCode = ErrorCode(4704);

// ── Pre-release (E5000–E5099) ────────────────────────────────────────────────
#[allow(dead_code)]
pub const PRERELEASE_EMPTY_CHANNEL: ErrorCode = ErrorCode(5001);
#[allow(dead_code)]
pub const PRERELEASE_INVALID_CHANNEL: ErrorCode = ErrorCode(5002);

// ── Versioning (E5010–E5019) ─────────────────────────────────────────────────
#[allow(dead_code)]
pub const VERSIONING_INVALID_SEMVER: ErrorCode = ErrorCode(5010);

// ── Hooks (E6000–E6099) ──────────────────────────────────────────────────────
#[allow(dead_code)]
pub const HOOK_FAILED: ErrorCode = ErrorCode(6001);

// ── Query (E7000–E7099) ──────────────────────────────────────────────────────
#[allow(dead_code)]
pub const QUERY_NO_PACKAGES: ErrorCode = ErrorCode(7001);
#[allow(dead_code)]
pub const QUERY_PACKAGE_NOT_FOUND: ErrorCode = ErrorCode(7002);

// ── Monorepo (E8000–E8099) ───────────────────────────────────────────────────
#[allow(dead_code)]
pub const MONOREPO_PACKAGE_NOT_FOUND: ErrorCode = ErrorCode(8001);
#[allow(dead_code)]
pub const MONOREPO_PUSH_FAILED: ErrorCode = ErrorCode(8002);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats_with_leading_zeros() {
        assert_eq!(ErrorCode(1).to_string(), "E0001");
        assert_eq!(ErrorCode(42).to_string(), "E0042");
        assert_eq!(ErrorCode(1001).to_string(), "E1001");
        assert_eq!(ErrorCode(9999).to_string(), "E9999");
    }

    #[test]
    fn doc_url_uses_lowercase() {
        let url = ErrorCode(1001).doc_url();
        assert_eq!(url, "https://ferrflow.com/docs/reference/errors#e1001");
    }

    #[test]
    fn error_code_ext_attaches_code() {
        let err: anyhow::Result<()> = Err(anyhow::anyhow!("something broke"));
        let err = err.error_code(ErrorCode(2001));
        let err = err.unwrap_err();

        // ErrorCode is the outermost context, so downcast_ref on the error itself works
        let code = err.downcast_ref::<ErrorCode>().copied();
        assert!(code.is_some());
        assert_eq!(code.unwrap().0, 2001);
    }

    #[test]
    fn error_without_code_returns_none() {
        let err = anyhow::anyhow!("plain error");
        let code = err.downcast_ref::<ErrorCode>().copied();
        assert!(code.is_none());
    }
}
