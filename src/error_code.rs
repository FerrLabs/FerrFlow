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
