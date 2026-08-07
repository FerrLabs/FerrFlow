use anyhow::{Context, Result, bail};

const DEFAULT_ENDPOINT: &str = "https://api.ferrflow.com/v1/ferrflow/token";
const DEFAULT_AUDIENCE: &str = "ferrflow.ferrlabs.com";

pub fn bot_mode_enabled() -> bool {
    match std::env::var("FERRFLOW_BOT") {
        Ok(value) => {
            let v = value.trim().to_ascii_lowercase();
            matches!(v.as_str(), "true" | "1")
        }
        Err(_) => false,
    }
}

pub struct BotTokenExchange {
    pub endpoint: String,
    pub audience: String,
}

impl Default for BotTokenExchange {
    fn default() -> Self {
        Self {
            endpoint: std::env::var("FERRFLOW_BOT_ENDPOINT")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
            audience: std::env::var("FERRFLOW_BOT_AUDIENCE")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| DEFAULT_AUDIENCE.to_string()),
        }
    }
}

#[derive(Debug)]
pub struct IssuedToken {
    pub token: String,
    pub expires_at: String,
    pub repository: String,
}

#[derive(serde::Deserialize)]
struct IssuedTokenResponse {
    token: String,
    #[serde(default)]
    expires_at: String,
    #[serde(default)]
    repository: String,
}

#[derive(serde::Deserialize)]
struct OidcResponse {
    value: String,
}

impl BotTokenExchange {
    pub fn issue(&self) -> Result<IssuedToken> {
        let req_url = std::env::var("ACTIONS_ID_TOKEN_REQUEST_URL").map_err(|_| {
            anyhow::anyhow!(
                "bot mode requires `permissions: id-token: write` in your workflow — ACTIONS_ID_TOKEN_REQUEST_URL not set"
            )
        })?;
        let req_token = std::env::var("ACTIONS_ID_TOKEN_REQUEST_TOKEN").map_err(|_| {
            anyhow::anyhow!(
                "bot mode requires `permissions: id-token: write` in your workflow — ACTIONS_ID_TOKEN_REQUEST_TOKEN not set"
            )
        })?;

        let separator = if req_url.contains('?') { '&' } else { '?' };
        let oidc_url = format!(
            "{req_url}{separator}audience={}",
            encode_query_component(&self.audience)
        );

        // The bare `ureq::get` / `ureq::post` helpers build an agent with
        // ureq's all-`None` timeout defaults, so a stalled runner or bot
        // service would block the release indefinitely.
        let agent = crate::http::agent();

        let oidc_body: OidcResponse = agent
            .get(&oidc_url)
            .header("Authorization", &format!("Bearer {req_token}"))
            .header("Accept", "application/json")
            .header(
                "User-Agent",
                concat!("ferrflow/", env!("CARGO_PKG_VERSION")),
            )
            .call()
            .context("failed to request OIDC token from GitHub Actions runner")?
            .body_mut()
            .read_json()
            .context("OIDC response from runner was not valid JSON")?;

        if oidc_body.value.is_empty() {
            bail!("OIDC response from GitHub Actions runner was missing the `value` field");
        }

        let payload = serde_json::json!({ "token": oidc_body.value });
        let mut response = match agent
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header(
                "User-Agent",
                concat!("ferrflow/", env!("CARGO_PKG_VERSION")),
            )
            .send_json(payload)
        {
            Ok(r) => r,
            Err(ureq::Error::StatusCode(code)) => {
                return Err(map_status_error(code));
            }
            Err(err) => {
                bail!(
                    "FerrFlow hosted bot unavailable: {err}. Check https://status.ferrlabs.com or fall back to a PAT via `token:`."
                );
            }
        };

        let body: IssuedTokenResponse = response
            .body_mut()
            .read_json()
            .context("FerrFlow bot service response was not valid JSON")?;

        if body.token.is_empty() {
            bail!("FerrFlow bot service response did not contain a token");
        }

        Ok(IssuedToken {
            token: body.token,
            expires_at: body.expires_at,
            repository: body.repository,
        })
    }
}

fn map_status_error(code: u16) -> anyhow::Error {
    match code {
        401 => anyhow::anyhow!(
            "FerrFlow OIDC verification failed (401). The runner's OIDC token was rejected by the hosted bot service."
        ),
        404 => anyhow::anyhow!(
            "FerrFlow App not installed on this repository's owner. Install at https://github.com/apps/ferrflow"
        ),
        429 => anyhow::anyhow!(
            "FerrFlow hosted bot rate limit hit (429). Retry shortly or use `token:` with a PAT."
        ),
        500..=599 => anyhow::anyhow!(
            "FerrFlow hosted bot service unavailable ({code}). Check https://status.ferrlabs.com"
        ),
        _ => anyhow::anyhow!("FerrFlow hosted bot returned unexpected HTTP status {code}"),
    }
}

fn encode_query_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        let safe = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~');
        if safe {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

pub fn ensure_bot_token() -> Result<()> {
    if !bot_mode_enabled() {
        return Ok(());
    }

    static EXCHANGED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    if EXCHANGED.get().is_some() {
        return Ok(());
    }

    let exchange = BotTokenExchange::default();
    let issued = exchange
        .issue()
        .context("failed to obtain FerrFlow bot token")?;

    // SAFETY: set_var mutates the process environment, which is UB (#710) if
    // another thread is concurrently reading or writing it. main() calls this
    // before concurrency::init spawns the rayon pool, so no other thread is
    // alive at this point. The EXCHANGED guard above keeps it one-shot.
    unsafe {
        std::env::set_var("GITHUB_TOKEN", &issued.token);
        std::env::set_var("FERRFLOW_TOKEN", &issued.token);
    }

    println!("::add-mask::{}", issued.token);

    let repo_note = if issued.repository.is_empty() {
        String::new()
    } else {
        format!(" on {}", issued.repository)
    };
    let expires_note = if issued.expires_at.is_empty() {
        String::new()
    } else {
        format!(" (expires at {})", issued.expires_at)
    };
    tracing::info!("Authenticated as ferrflow[bot]{repo_note}{expires_note}.");

    configure_bot_git_identity();

    let _ = EXCHANGED.set(());
    Ok(())
}

const DEFAULT_BOT_LOGIN: &str = "ferrflow[bot]";
const DEFAULT_BOT_USER_ID: &str = "278126555";

fn configure_bot_git_identity() {
    if let Ok(cwd) = std::env::current_dir() {
        configure_bot_git_identity_in(&cwd);
    }
}

fn configure_bot_git_identity_in(repo_dir: &std::path::Path) {
    let login = std::env::var("FERRFLOW_BOT_LOGIN")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_BOT_LOGIN.to_string());
    let user_id = std::env::var("FERRFLOW_BOT_USER_ID")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_BOT_USER_ID.to_string());
    let email = format!("{user_id}+{login}@users.noreply.github.com");

    let _ = std::process::Command::new("git")
        .args(["config", "--local", "user.name", &login])
        .current_dir(repo_dir)
        .status();
    let _ = std::process::Command::new("git")
        .args(["config", "--local", "user.email", &email])
        .current_dir(repo_dir)
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous: Vec<(String, Option<String>)> = vars
            .iter()
            .map(|(k, _)| ((*k).to_string(), std::env::var(*k).ok()))
            .collect();
        for (k, v) in vars {
            unsafe {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
        f();
        for (k, v) in previous {
            unsafe {
                match v {
                    Some(val) => std::env::set_var(&k, val),
                    None => std::env::remove_var(&k),
                }
            }
        }
    }

    #[test]
    fn bot_mode_detection() {
        with_env(&[("FERRFLOW_BOT", Some("true"))], || {
            assert!(bot_mode_enabled());
        });
        with_env(&[("FERRFLOW_BOT", Some("1"))], || {
            assert!(bot_mode_enabled());
        });
        with_env(&[("FERRFLOW_BOT", Some("TRUE"))], || {
            assert!(bot_mode_enabled());
        });
        with_env(&[("FERRFLOW_BOT", Some("false"))], || {
            assert!(!bot_mode_enabled());
        });
        with_env(&[("FERRFLOW_BOT", Some(""))], || {
            assert!(!bot_mode_enabled());
        });
        with_env(&[("FERRFLOW_BOT", None)], || {
            assert!(!bot_mode_enabled());
        });
    }

    // Asserted against the literals, not the constants: the point is that these
    // two values are a contract with the deployed service, not that the struct
    // copies its own defaults. The audience in particular is what the server
    // checks the OIDC token against, so a change here rejects every runner.
    #[test]
    fn defaults_use_hosted_endpoint_and_audience() {
        with_env(
            &[
                ("FERRFLOW_BOT_ENDPOINT", None),
                ("FERRFLOW_BOT_AUDIENCE", None),
            ],
            || {
                let ex = BotTokenExchange::default();
                assert_eq!(ex.endpoint, "https://api.ferrflow.com/v1/ferrflow/token");
                assert_eq!(ex.audience, "ferrflow.ferrlabs.com");
            },
        );
    }

    #[test]
    fn overrides_applied() {
        with_env(
            &[
                ("FERRFLOW_BOT_ENDPOINT", Some("https://example.test/t")),
                ("FERRFLOW_BOT_AUDIENCE", Some("aud.example.test")),
            ],
            || {
                let ex = BotTokenExchange::default();
                assert_eq!(ex.endpoint, "https://example.test/t");
                assert_eq!(ex.audience, "aud.example.test");
            },
        );
    }

    #[test]
    fn empty_overrides_fall_back_to_defaults() {
        with_env(
            &[
                ("FERRFLOW_BOT_ENDPOINT", Some("")),
                ("FERRFLOW_BOT_AUDIENCE", Some("")),
            ],
            || {
                let ex = BotTokenExchange::default();
                assert_eq!(ex.endpoint, DEFAULT_ENDPOINT);
                assert_eq!(ex.audience, DEFAULT_AUDIENCE);
            },
        );
    }

    #[test]
    fn issue_errors_when_runner_env_missing() {
        with_env(
            &[
                ("ACTIONS_ID_TOKEN_REQUEST_URL", None),
                ("ACTIONS_ID_TOKEN_REQUEST_TOKEN", None),
            ],
            || {
                let err = BotTokenExchange::default().issue().unwrap_err();
                let msg = err.to_string();
                assert!(
                    msg.contains("id-token: write"),
                    "expected id-token hint in error, got: {msg}"
                );
            },
        );
    }

    #[test]
    fn encode_query_component_leaves_safe_chars() {
        assert_eq!(
            encode_query_component("ferrflow.ferrlabs.com"),
            "ferrflow.ferrlabs.com"
        );
    }

    #[test]
    fn encode_query_component_escapes_unsafe() {
        assert_eq!(encode_query_component("a b&c=d"), "a%20b%26c%3Dd");
    }

    fn read_local_git_config(repo_dir: &std::path::Path, key: &str) -> Option<String> {
        let out = std::process::Command::new("git")
            .args(["config", "--local", "--get", key])
            .current_dir(repo_dir)
            .output()
            .ok()?;
        if out.status.success() {
            Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            None
        }
    }

    fn init_repo(dir: &std::path::Path) {
        let ok = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "git init must succeed for the test setup");
    }

    #[test]
    fn configure_bot_git_identity_uses_hosted_defaults() {
        if std::process::Command::new("git")
            .arg("--version")
            .status()
            .is_err()
        {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        init_repo(tmp.path());

        with_env(
            &[("FERRFLOW_BOT_LOGIN", None), ("FERRFLOW_BOT_USER_ID", None)],
            || {
                configure_bot_git_identity_in(tmp.path());
            },
        );

        assert_eq!(
            read_local_git_config(tmp.path(), "user.name").as_deref(),
            Some("ferrflow[bot]")
        );
        assert_eq!(
            read_local_git_config(tmp.path(), "user.email").as_deref(),
            Some("278126555+ferrflow[bot]@users.noreply.github.com")
        );
    }

    #[test]
    fn configure_bot_git_identity_honours_env_overrides() {
        if std::process::Command::new("git")
            .arg("--version")
            .status()
            .is_err()
        {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        init_repo(tmp.path());

        with_env(
            &[
                ("FERRFLOW_BOT_LOGIN", Some("vault-bot[bot]")),
                ("FERRFLOW_BOT_USER_ID", Some("999")),
            ],
            || {
                configure_bot_git_identity_in(tmp.path());
            },
        );

        assert_eq!(
            read_local_git_config(tmp.path(), "user.name").as_deref(),
            Some("vault-bot[bot]")
        );
        assert_eq!(
            read_local_git_config(tmp.path(), "user.email").as_deref(),
            Some("999+vault-bot[bot]@users.noreply.github.com")
        );
    }

    #[test]
    fn configure_bot_git_identity_treats_blank_overrides_as_unset() {
        if std::process::Command::new("git")
            .arg("--version")
            .status()
            .is_err()
        {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        init_repo(tmp.path());

        with_env(
            &[
                ("FERRFLOW_BOT_LOGIN", Some("")),
                ("FERRFLOW_BOT_USER_ID", Some("")),
            ],
            || {
                configure_bot_git_identity_in(tmp.path());
            },
        );

        assert_eq!(
            read_local_git_config(tmp.path(), "user.name").as_deref(),
            Some("ferrflow[bot]")
        );
    }
}
