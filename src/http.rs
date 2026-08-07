use std::time::Duration;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_GLOBAL_TIMEOUT: Duration = Duration::from_secs(120);

const GLOBAL_TIMEOUT_ENV: &str = "FERRFLOW_HTTP_TIMEOUT";

/// Every timeout in `ureq::Config` defaults to `None`, so an agent built
/// with `Agent::new_with_defaults()` blocks on a stalled peer until the
/// OS gives up — minutes to hours, with the release lock held the whole
/// time and nothing for `retry_transient` to classify. Build release-path
/// agents here instead.
pub fn agent() -> ureq::Agent {
    agent_with_global_timeout(global_timeout())
}

fn agent_with_global_timeout(global: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(DEFAULT_CONNECT_TIMEOUT))
        .timeout_recv_response(Some(DEFAULT_RESPONSE_TIMEOUT))
        .timeout_global(Some(global))
        .build()
        .into()
}

fn global_timeout() -> Duration {
    std::env::var(GLOBAL_TIMEOUT_ENV)
        .ok()
        .and_then(|raw| parse_global_timeout(&raw))
        .unwrap_or(DEFAULT_GLOBAL_TIMEOUT)
}

fn parse_global_timeout(raw: &str) -> Option<Duration> {
    let secs: u64 = raw.trim().parse().ok()?;
    (secs > 0).then(|| Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_global_timeout_accepts_positive_seconds() {
        assert_eq!(parse_global_timeout("45"), Some(Duration::from_secs(45)));
        assert_eq!(
            parse_global_timeout("  600 "),
            Some(Duration::from_secs(600))
        );
    }

    #[test]
    fn parse_global_timeout_rejects_zero_and_garbage() {
        // Zero would mean "no timeout" to a reader but ureq treats every
        // duration as a real deadline, so it must not silently disable the
        // budget this module exists to enforce.
        assert_eq!(parse_global_timeout("0"), None);
        assert_eq!(parse_global_timeout(""), None);
        assert_eq!(parse_global_timeout("30s"), None);
        assert_eq!(parse_global_timeout("-5"), None);
    }

    /// The regression this module exists for: an agent whose config still
    /// carries ureq's all-`None` defaults will hang a release. Assert the
    /// three timeouts we care about are actually set.
    #[test]
    fn agent_config_has_connect_response_and_global_timeouts() {
        let agent = agent_with_global_timeout(Duration::from_secs(90));
        let timeouts = agent.config().timeouts();
        assert_eq!(timeouts.connect, Some(DEFAULT_CONNECT_TIMEOUT));
        assert_eq!(timeouts.recv_response, Some(DEFAULT_RESPONSE_TIMEOUT));
        assert_eq!(timeouts.global, Some(Duration::from_secs(90)));
    }

    #[test]
    fn stalled_peer_fails_within_the_global_budget() {
        use std::io::Read;
        use std::net::TcpListener;

        // Accept the connection, then never write a response — the exact
        // shape that hangs forever on ureq's defaults.
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut sink = [0u8; 1024];
                let _ = stream.read(&mut sink);
                std::thread::sleep(Duration::from_secs(30));
            }
        });

        let agent = agent_with_global_timeout(Duration::from_secs(2));
        let started = std::time::Instant::now();
        let result = agent.get(&format!("http://127.0.0.1:{port}/")).call();
        let elapsed = started.elapsed();

        assert!(result.is_err(), "a stalled peer must not resolve");
        assert!(
            elapsed < Duration::from_secs(10),
            "call should abort near the 2s budget, took {elapsed:?}"
        );
        drop(handle);
    }
}
