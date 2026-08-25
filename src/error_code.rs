use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct ErrorCode(pub u16);

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E{:04}", self.0)
    }
}

impl std::error::Error for ErrorCode {}

impl ErrorCode {
    pub fn doc_url(&self) -> String {
        format!(
            "https://ferrflow.com/docs/reference/errors#{}",
            self.to_string().to_lowercase()
        )
    }
}

#[allow(dead_code)]
pub trait ErrorCodeExt<T> {
    fn error_code(self, code: ErrorCode) -> anyhow::Result<T>;
}

impl<T> ErrorCodeExt<T> for anyhow::Result<T> {
    fn error_code(self, code: ErrorCode) -> anyhow::Result<T> {
        self.map_err(|e| e.context(code))
    }
}

#[allow(dead_code)]
pub fn code_from_error(err: &anyhow::Error) -> Option<String> {
    err.downcast_ref::<ErrorCode>().map(ToString::to_string)
}

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
#[allow(dead_code)]
pub const CONFIG_INVALID_PATH: ErrorCode = ErrorCode(1018);
#[allow(dead_code)]
pub const CONFIG_INCLUDE_NOT_FOUND: ErrorCode = ErrorCode(1019);
#[allow(dead_code)]
pub const CONFIG_INCLUDE_INVALID: ErrorCode = ErrorCode(1020);
#[allow(dead_code)]
pub const CONFIG_INCLUDE_OUTSIDE_ROOT: ErrorCode = ErrorCode(1021);
#[allow(dead_code)]
pub const CONFIG_DUPLICATE_PACKAGE: ErrorCode = ErrorCode(1022);
#[allow(dead_code)]
pub const CONFIG_MISSING_PACKAGE_PATH: ErrorCode = ErrorCode(1023);

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
#[allow(dead_code)]
pub const GIT_LOCKED: ErrorCode = ErrorCode(2011);
#[allow(dead_code)]
pub const GIT_FORCE_PUSH_BRANCH: ErrorCode = ErrorCode(2012);
#[allow(dead_code)]
pub const GIT_INSPECT_RELEASE_BRANCH: ErrorCode = ErrorCode(2013);

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
#[allow(dead_code)]
pub const GITHUB_FIND_PR: ErrorCode = ErrorCode(3011);
#[allow(dead_code)]
pub const GITHUB_UPDATE_PR: ErrorCode = ErrorCode(3012);

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
#[allow(dead_code)]
pub const GITLAB_FIND_MR: ErrorCode = ErrorCode(3106);
#[allow(dead_code)]
pub const GITLAB_UPDATE_MR: ErrorCode = ErrorCode(3107);

#[allow(dead_code)]
pub const GITEA_CREATE_RELEASE: ErrorCode = ErrorCode(3201);
#[allow(dead_code)]
pub const GITEA_LIST_RELEASES: ErrorCode = ErrorCode(3202);
#[allow(dead_code)]
pub const GITEA_PUBLISH_RELEASE: ErrorCode = ErrorCode(3203);

#[allow(dead_code)]
pub const BITBUCKET_CREATE_RELEASE: ErrorCode = ErrorCode(3301);

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

#[allow(dead_code)]
pub const HELM_READ: ErrorCode = ErrorCode(4301);
#[allow(dead_code)]
pub const HELM_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4302);
#[allow(dead_code)]
pub const HELM_WRITE: ErrorCode = ErrorCode(4303);
#[allow(dead_code)]
pub const HELM_INVALID_UTF8: ErrorCode = ErrorCode(4304);

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

#[allow(dead_code)]
pub const GRADLE_READ: ErrorCode = ErrorCode(4501);
#[allow(dead_code)]
pub const GRADLE_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4502);
#[allow(dead_code)]
pub const GRADLE_WRITE: ErrorCode = ErrorCode(4503);
#[allow(dead_code)]
pub const GRADLE_INVALID_UTF8: ErrorCode = ErrorCode(4504);

#[allow(dead_code)]
pub const GOMOD_GIT_DESCRIBE: ErrorCode = ErrorCode(4601);
#[allow(dead_code)]
pub const GOMOD_NO_TAG: ErrorCode = ErrorCode(4602);
#[allow(dead_code)]
pub const GOMOD_UNSUPPORTED: ErrorCode = ErrorCode(4603);

#[allow(dead_code)]
pub const TXT_READ: ErrorCode = ErrorCode(4701);
#[allow(dead_code)]
pub const TXT_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4702);
#[allow(dead_code)]
pub const TXT_WRITE: ErrorCode = ErrorCode(4703);
#[allow(dead_code)]
pub const TXT_INVALID_UTF8: ErrorCode = ErrorCode(4704);

#[allow(dead_code)]
pub const PUBSPEC_READ: ErrorCode = ErrorCode(4801);
#[allow(dead_code)]
pub const PUBSPEC_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4802);
#[allow(dead_code)]
pub const PUBSPEC_WRITE: ErrorCode = ErrorCode(4803);
#[allow(dead_code)]
pub const PUBSPEC_INVALID_UTF8: ErrorCode = ErrorCode(4804);

#[allow(dead_code)]
pub const MIX_EXS_READ: ErrorCode = ErrorCode(4811);
#[allow(dead_code)]
pub const MIX_EXS_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4812);
#[allow(dead_code)]
pub const MIX_EXS_WRITE: ErrorCode = ErrorCode(4813);
#[allow(dead_code)]
pub const MIX_EXS_INVALID_UTF8: ErrorCode = ErrorCode(4814);

#[allow(dead_code)]
pub const CHART_YAML_READ: ErrorCode = ErrorCode(4821);
#[allow(dead_code)]
pub const CHART_YAML_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4822);
#[allow(dead_code)]
pub const CHART_YAML_WRITE: ErrorCode = ErrorCode(4823);
#[allow(dead_code)]
pub const CHART_YAML_INVALID_UTF8: ErrorCode = ErrorCode(4824);

#[allow(dead_code)]
pub const GEMSPEC_READ: ErrorCode = ErrorCode(4831);
#[allow(dead_code)]
pub const GEMSPEC_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4832);
#[allow(dead_code)]
pub const GEMSPEC_WRITE: ErrorCode = ErrorCode(4833);
#[allow(dead_code)]
pub const GEMSPEC_INVALID_UTF8: ErrorCode = ErrorCode(4834);

#[allow(dead_code)]
pub const PACKAGE_SWIFT_READ: ErrorCode = ErrorCode(4841);
#[allow(dead_code)]
pub const PACKAGE_SWIFT_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4842);
#[allow(dead_code)]
pub const PACKAGE_SWIFT_WRITE: ErrorCode = ErrorCode(4843);
#[allow(dead_code)]
pub const PACKAGE_SWIFT_INVALID_UTF8: ErrorCode = ErrorCode(4844);

#[allow(dead_code)]
pub const CABAL_READ: ErrorCode = ErrorCode(4851);
#[allow(dead_code)]
pub const CABAL_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4852);
#[allow(dead_code)]
pub const CABAL_WRITE: ErrorCode = ErrorCode(4853);
#[allow(dead_code)]
pub const CABAL_INVALID_UTF8: ErrorCode = ErrorCode(4854);

#[allow(dead_code)]
pub const CMAKE_READ: ErrorCode = ErrorCode(4861);
#[allow(dead_code)]
pub const CMAKE_VERSION_NOT_FOUND: ErrorCode = ErrorCode(4862);
#[allow(dead_code)]
pub const CMAKE_WRITE: ErrorCode = ErrorCode(4863);
#[allow(dead_code)]
pub const CMAKE_INVALID_UTF8: ErrorCode = ErrorCode(4864);

#[allow(dead_code)]
pub const PRERELEASE_EMPTY_CHANNEL: ErrorCode = ErrorCode(5001);
#[allow(dead_code)]
pub const PRERELEASE_INVALID_CHANNEL: ErrorCode = ErrorCode(5002);

#[allow(dead_code)]
pub const VERSIONING_INVALID_SEMVER: ErrorCode = ErrorCode(5010);

#[allow(dead_code)]
pub const HOOK_FAILED: ErrorCode = ErrorCode(6001);

#[allow(dead_code)]
pub const QUERY_NO_PACKAGES: ErrorCode = ErrorCode(7001);
#[allow(dead_code)]
pub const QUERY_PACKAGE_NOT_FOUND: ErrorCode = ErrorCode(7002);
#[allow(dead_code)]
pub const DIFF_BAD_RANGE: ErrorCode = ErrorCode(7003);
#[allow(dead_code)]
pub const DIFF_PACKAGE_REQUIRED: ErrorCode = ErrorCode(7004);
#[allow(dead_code)]
pub const DIFF_ENDPOINT_UNRESOLVED: ErrorCode = ErrorCode(7005);

#[allow(dead_code)]
pub const MONOREPO_PACKAGE_NOT_FOUND: ErrorCode = ErrorCode(8001);
#[allow(dead_code)]
pub const MONOREPO_PUSH_FAILED: ErrorCode = ErrorCode(8002);
#[allow(dead_code)]
pub const MONOREPO_DEPENDENCY_CYCLE: ErrorCode = ErrorCode(8003);

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

    #[test]
    fn code_from_error_finds_wrapped_code() {
        let err = anyhow::anyhow!("boom")
            .context(ErrorCode(2004))
            .context("while pushing");
        assert_eq!(code_from_error(&err).as_deref(), Some("E2004"));
    }

    #[test]
    fn code_from_error_none_when_absent() {
        let err = anyhow::anyhow!("plain").context("still plain");
        assert!(code_from_error(&err).is_none());
    }
}
