use crate::error_code::ErrorCode;

/// The lines to log for a failed command, one physical line per element.
///
/// Walking the chain and emitting a line per cause — rather than a single
/// `{err:?}` — keeps every cause on its own clean line through the human log
/// layer, which records the message field with `{:?}` and would otherwise
/// render anyhow's multi-line Debug with an orphaned `Caused by:` and a blank
/// line. Both the coded and uncoded paths now format the same way (see #694).
pub fn error_report_lines(err: &anyhow::Error) -> Vec<String> {
    // anyhow finds the ErrorCode through the whole chain, but the code is
    // attached with `.context(code)`, so it also surfaces as a chain link whose
    // Display is the bare code (`E1001`). Filter that link out by value rather
    // than by `downcast_ref` on the chain: the context wrapper doesn't downcast
    // back to ErrorCode on `&dyn Error`, so the naive filter leaves a duplicated
    // `error[E1001]: E1001` head.
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
            lines.push(format!("error[{code}]: {head}"));
            for cause in causes.iter().skip(1) {
                lines.push(format!("  {cause}"));
            }
            lines.push(String::new());
            lines.push(format!("  For help: {}", code.doc_url()));
        }
        None => {
            let head = causes
                .first()
                .map(String::as_str)
                .unwrap_or("unknown error");
            lines.push(format!("Error: {head}"));
            for cause in causes.iter().skip(1) {
                lines.push(format!("  Caused by: {cause}"));
            }
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_code::{self, ErrorCodeExt};
    use anyhow::Context;

    #[test]
    fn uncoded_error_keeps_every_cause_on_its_own_line() {
        // The bot-token shape from #694: a context wrapping a specific cause,
        // with no ErrorCode attached.
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
        // The ErrorCode itself is a chain link but must not print as a cause.
        assert!(
            !lines
                .iter()
                .any(|l| l.contains("E1001:") && l.starts_with("  Caused by:"))
        );
    }

    // The ErrorCode is stripped from the causes so it never doubles as a
    // "Caused by:" line.
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
}
