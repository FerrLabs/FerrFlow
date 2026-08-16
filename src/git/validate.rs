use anyhow::{Result, anyhow};

/// Reject obviously-unsafe ref name fragments before passing them to
/// `git` subprocesses or interpolating them into refspecs.
///
/// This is **not** the full `git check-ref-format` algorithm — it only
/// catches the attack-relevant slice: leading `-` (flag-confusion on
/// older git), NUL / newline / control chars (header smuggling, broken
/// formatted error strings), and the empty string. Legitimate exotic
/// names (unicode emoji, slashes, dots) are intentionally allowed.
///
/// Call this before any `Command::new("git").arg(<refname>)` where the
/// `<refname>` could plausibly come from user-controlled input (config
/// file, env var, commit message, package name).
pub fn ensure_safe_refname_fragment(name: &str, context: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("{context}: ref name is empty"));
    }
    if name.starts_with('-') {
        return Err(anyhow!(
            "{context}: ref name '{name}' starts with '-', which some git versions \
             may misinterpret as a command-line flag. Rename it."
        ));
    }
    for ch in name.chars() {
        let bad = ch == '\0'
            || ch == '\n'
            || ch == '\r'
            || ch == '\x7f'
            || (ch.is_control() && !matches!(ch, '\t'));
        if bad {
            return Err(anyhow!(
                "{context}: ref name '{name}' contains a disallowed control character (\
                 U+{:04X}). Rename it.",
                ch as u32
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert!(ensure_safe_refname_fragment("", "tag").is_err());
    }

    #[test]
    fn rejects_leading_dash() {
        let err = ensure_safe_refname_fragment("--exec=ls", "tag")
            .expect_err("should reject leading dash");
        assert!(format!("{err:?}").contains("starts with '-'"));
    }

    #[test]
    fn rejects_newline() {
        assert!(ensure_safe_refname_fragment("foo\nbar", "tag").is_err());
        assert!(ensure_safe_refname_fragment("foo\r", "tag").is_err());
    }

    #[test]
    fn rejects_null_byte() {
        assert!(ensure_safe_refname_fragment("foo\0bar", "tag").is_err());
    }

    #[test]
    fn rejects_control_chars() {
        assert!(ensure_safe_refname_fragment("foo\x01bar", "tag").is_err());
        assert!(ensure_safe_refname_fragment("foo\x7fbar", "tag").is_err());
    }

    #[test]
    fn allows_normal_names() {
        for ok in &[
            "v1.0.0",
            "release/v1.0.0",
            "api@v1.0.0",
            "pkg/v1.2.3",
            "feature-branch_2",
            "1.x",
            "v0.0.1-beta.42",
            "我的分支",
        ] {
            assert!(
                ensure_safe_refname_fragment(ok, "test").is_ok(),
                "should allow '{ok}'"
            );
        }
    }

    #[test]
    fn allows_tab() {
        assert!(ensure_safe_refname_fragment("foo\tbar", "tag").is_ok());
    }
}
