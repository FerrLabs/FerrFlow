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
    if chain.contains("fatal error in commit_refs")
        || chain.contains("internal server error")
        || chain.contains("remote end hung up")
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

/// Whether the push failed because the remote moved under us, which the release
/// flow answers by regenerating the release commit against the new tip.
///
/// This deliberately does not match `GIT_PUSH_BRANCH` or `GIT_PUSH_TAGS`. Those
/// are the generic "push failed" codes carried by every push error, transient
/// and permanent alike, so matching them reported a stale branch for causes that
/// were nothing of the sort and burned the regenerate attempts on errors no
/// amount of regenerating could fix.
pub fn is_push_rejected_error(err: &anyhow::Error) -> bool {
    let rejected_code = error_code::GIT_PUSH_REJECTED.to_string();
    err.chain().any(|cause| {
        let raw = cause.to_string();
        if raw == rejected_code {
            return true;
        }
        let msg = raw.to_lowercase();
        msg.contains("rebase conflict")
            || msg.contains("push declined due to repository rule")
            || msg.contains("non-fast-forward")
            || msg.contains("non-fastforward")
            || msg.contains("not fast forward")
            // git says "fetch first" when the remote advanced and we have not
            // fetched since, which is the usual shape in CI, and "stale info"
            // when a --force-with-lease expectation is out of date.
            || msg.contains("fetch first")
            || msg.contains("stale info")
            || msg.contains("already exist on remote pointing to a different commit")
            || msg.contains("expected branch to point to")
    })
}
