use anyhow::Result;
use colored::Colorize;
use std::time::Duration;

use crate::error_code;

pub(super) fn retry_transient<F>(label: &str, mut op: F) -> Result<()>
where
    F: FnMut() -> Result<()>,
{
    const MAX_ATTEMPTS: u32 = 4;
    let mut delay = Duration::from_secs(1);
    let mut last_err: Option<anyhow::Error> = None;

    for attempt in 1..=MAX_ATTEMPTS {
        match op() {
            Ok(()) => {
                if attempt > 1 {
                    tracing::info!(
                        "{}",
                        format!("  ✓ {label} succeeded on attempt {attempt}/{MAX_ATTEMPTS}")
                            .green()
                    );
                }
                return Ok(());
            }
            Err(err) => {
                let transient = is_transient_git_error(&err);
                if !transient || attempt == MAX_ATTEMPTS {
                    return Err(err);
                }
                tracing::warn!(
                    "{}",
                    format!(
                        "  ⚠ {label} attempt {attempt}/{MAX_ATTEMPTS} failed (transient): {err}; \
                         retrying in {}s",
                        delay.as_secs()
                    )
                    .yellow()
                );
                std::thread::sleep(delay);
                delay = delay.saturating_mul(2);
                last_err = Some(err);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("retry loop exited without result")))
}

pub(super) fn is_transient_git_error(err: &anyhow::Error) -> bool {
    let chain = err
        .chain()
        .map(|e| e.to_string().to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    if chain.contains("connection")
        || chain.contains("timeout")
        || chain.contains("timed out")
        || chain.contains("could not resolve host")
        || chain.contains("temporarily unavailable")
        || chain.contains("network")
        || chain.contains("connection reset")
        || chain.contains("rst_stream")
        || chain.contains("broken pipe")
        || chain.contains("ssl")
        || chain.contains("tls")
    {
        return true;
    }
    if chain.contains("502")
        || chain.contains("503")
        || chain.contains("504")
        || chain.contains("bad gateway")
        || chain.contains("service unavailable")
        || chain.contains("gateway timeout")
        || chain.contains("secondary rate limit")
        || chain.contains("rate limit exceeded")
    {
        return true;
    }
    if chain.contains("object is no commit object")
        || chain.contains("no commit object")
        || chain.contains("class=invalid")
        || chain.contains("object not found")
        || chain.contains("odb")
    {
        return true;
    }
    if chain.contains("non-fast-forward")
        || chain.contains("branch protection")
        || chain.contains("rejected by remote")
        || chain.contains("authentication failed")
        || chain.contains("permission denied")
        || chain.contains("repository not found")
    {
        return false;
    }
    false
}

pub fn is_push_rejected_error(err: &anyhow::Error) -> bool {
    // GIT_PUSH_TAGS covers the concurrent-release case: another run published
    // the version this plan plotted, so the tag exists on the remote at a
    // different commit. Regenerating recomputes the plan against the winner's
    // history and picks the next version on top of it, which is the only safe
    // resolution — deleting or force-pushing the tag would destroy a published
    // release.
    let push_codes: &[String] = &[
        error_code::GIT_PUSH_REJECTED.to_string(),
        error_code::GIT_PUSH_BRANCH.to_string(),
        error_code::GIT_PUSH_TAGS.to_string(),
    ];
    err.chain().any(|cause| {
        let raw = cause.to_string();
        if push_codes.iter().any(|c| raw == c.as_str()) {
            return true;
        }
        let msg = raw.to_lowercase();
        msg.contains("rebase conflict")
            || msg.contains("push declined due to repository rule")
            || msg.contains("non-fast-forward")
            || msg.contains("non-fastforward")
            || msg.contains("not fast forward")
    })
}
