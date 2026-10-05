use crate::error_code::ErrorCode;

/// The lines to log for a failed command, one physical line per element.
///
/// Walking the chain and emitting a line per cause — rather than a single
/// `{err:?}` — keeps every cause on its own clean line through the human log
/// layer, which records the message field with `{:?}` and would otherwise
/// render anyhow's multi-line Debug with an orphaned `Caused by:` and a blank
/// line. Both the coded and uncoded paths now format the same way (see #694).
pub fn error_report_lines(err: &anyhow::Error) -> Vec<String> {
    let code = err.downcast_ref::<ErrorCode>().copied();
    let code_str = code.map(|c| c.to_string());

    let causes: Vec<String> = err
        .chain()
        .map(|c| c.to_string())
        .filter(|s| code_str.as_deref() != Some(s.as_str()))
        .collect();

    let mut lines = Vec::new();

    match code {
        Some(code) => {
            let head = causes
                .first()
                .map(String::as_str)
                .unwrap_or("unknown error");
            push_message(&mut lines, &format!("error[{code}]: "), "  ", head);
            for cause in causes.iter().skip(1) {
                push_message(&mut lines, "  ", "    ", cause);
            }
            lines.push(String::new());
            lines.push(format!("  For help: {}", code.doc_url()));
        }
        None => {
            let head = causes
                .first()
                .map(String::as_str)
                .unwrap_or("unknown error");
            push_message(&mut lines, "Error: ", "  ", head);
            for cause in causes.iter().skip(1) {
                push_message(&mut lines, "  Caused by: ", "    ", cause);
            }
        }
    }

    lines
}

fn push_message(lines: &mut Vec<String>, lead: &str, indent: &str, message: &str) {
    let mut physical = message.lines();
    lines.push(format!("{lead}{}", physical.next().unwrap_or_default()));
    lines.extend(physical.map(|line| format!("{indent}{line}")));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_code::{self, ErrorCodeExt};
    use anyhow::Context;

    #[test]
    fn uncoded_error_keeps_every_cause_on_its_own_line() {
        let err = anyhow::Result::<()>::Err(anyhow::anyhow!(
            "FerrFlow hosted bot service unavailable (503). Check https://status.ferrlabs.com"
        ))
        .context("failed to obtain FerrFlow bot token")
        .unwrap_err();

        let lines = error_report_lines(&err);
        assert_eq!(
            lines,
            [
                "Error: failed to obtain FerrFlow bot token",
                "  Caused by: FerrFlow hosted bot service unavailable (503). Check https://status.ferrlabs.com",
            ]
        );
    }

    #[test]
    fn a_bare_error_is_a_single_line() {
        let err = anyhow::anyhow!("something broke");
        assert_eq!(error_report_lines(&err), ["Error: something broke"]);
    }

    #[test]
    fn coded_error_shows_the_code_and_help_url() {
        let err = anyhow::Result::<()>::Err(anyhow::anyhow!("config file not found"))
            .error_code(error_code::CONFIG_NOT_FOUND)
            .unwrap_err();

        let lines = error_report_lines(&err);
        assert_eq!(lines[0], "error[E1001]: config file not found");
        assert_eq!(lines[lines.len() - 2], "");
        assert!(lines.last().unwrap().starts_with("  For help: "));
        assert!(
            !lines
                .iter()
                .any(|l| l.contains("E1001:") && l.starts_with("  Caused by:"))
        );
    }

    #[test]
    fn coded_error_with_extra_context_lists_the_real_causes_only() {
        let err = anyhow::Result::<()>::Err(anyhow::anyhow!("permission denied"))
            .context("could not read config")
            .error_code(error_code::CONFIG_NOT_FOUND)
            .unwrap_err();

        let lines = error_report_lines(&err);
        assert_eq!(lines[0], "error[E1001]: could not read config");
        assert_eq!(lines[1], "  permission denied");
    }

    #[test]
    fn a_multi_line_coded_message_keeps_its_continuation_indented() {
        let err = anyhow::Result::<()>::Err(anyhow::anyhow!(
            "cargo publish failed for core on forgejo: error: failed to prepare local package\nCaused by:\nno matching package named `core` found"
        ))
        .error_code(error_code::PUBLISH_FAILED)
        .unwrap_err();

        let lines = error_report_lines(&err);
        assert_eq!(
            lines[..3],
            [
                "error[E6102]: cargo publish failed for core on forgejo: error: failed to prepare local package",
                "  Caused by:",
                "  no matching package named `core` found",
            ]
        );
    }

    #[test]
    fn a_multi_line_cause_indents_under_its_own_line() {
        let err = anyhow::Result::<()>::Err(anyhow::anyhow!("first line\nsecond line"))
            .context("could not publish")
            .unwrap_err();

        assert_eq!(
            error_report_lines(&err),
            [
                "Error: could not publish",
                "  Caused by: first line",
                "    second line",
            ]
        );
    }
}
