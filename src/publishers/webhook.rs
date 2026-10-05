use anyhow::{Context, Result, anyhow};
use std::collections::BTreeMap;

use super::{PublishContext, PublishOutcome};
use crate::error_code::{self, ErrorCodeExt};

pub fn run(
    url: &str,
    body: Option<&serde_json::Value>,
    headers: &BTreeMap<String, String>,
    ctx: &PublishContext<'_>,
) -> Result<PublishOutcome> {
    let interpolated_url = interpolate(url, ctx)?;
    if ctx.dry_run {
        return Ok(PublishOutcome::DryRun);
    }

    let payload = match body {
        Some(template) => interpolate_value(template, ctx)?,
        None => default_body(ctx),
    };

    let agent = crate::http::agent();
    let mut req = agent.post(&interpolated_url);
    for (k, v) in headers {
        let resolved = interpolate(v, ctx)?;
        req = req.header(k, &resolved);
    }
    if !headers.contains_key("Content-Type") && !headers.contains_key("content-type") {
        req = req.header("Content-Type", "application/json");
    }

    let response = req
        .send_json(payload)
        .with_context(|| format!("POST {interpolated_url}"))
        .error_code(error_code::PUBLISH_FAILED)?;

    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!(
            "webhook POST {interpolated_url} returned HTTP {status}"
        ))
        .error_code(error_code::PUBLISH_FAILED);
    }

    Ok(PublishOutcome::Published {
        url: Some(interpolated_url),
    })
}

fn default_body(ctx: &PublishContext<'_>) -> serde_json::Value {
    serde_json::json!({
        "package": ctx.package_name,
        "version": ctx.new_version,
        "tag": ctx.tag,
    })
}

fn interpolate(s: &str, ctx: &PublishContext<'_>) -> Result<String> {
    let mut out = s.to_string();
    out = out.replace("{name}", ctx.package_name);
    out = out.replace("{version}", ctx.new_version);
    out = out.replace("{tag}", ctx.tag);
    out = resolve_env_placeholders(&out)?;
    Ok(out)
}

fn interpolate_value(v: &serde_json::Value, ctx: &PublishContext<'_>) -> Result<serde_json::Value> {
    match v {
        serde_json::Value::String(s) => Ok(serde_json::Value::String(interpolate(s, ctx)?)),
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(|x| interpolate_value(x, ctx))
            .collect::<Result<Vec<_>>>()
            .map(serde_json::Value::Array),
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, val) in map {
                out.insert(k.clone(), interpolate_value(val, ctx)?);
            }
            Ok(serde_json::Value::Object(out))
        }
        _ => Ok(v.clone()),
    }
}

fn resolve_env_placeholders(s: &str) -> Result<String> {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("{env:") {
        out.push_str(&rest[..start]);
        let after_marker = &rest[start + 5..];
        let end = after_marker
            .find('}')
            .ok_or_else(|| anyhow!("publisher webhook: unterminated `{{env:...}}` in template"))?;
        let var_name = &after_marker[..end];
        let value = std::env::var(var_name).map_err(|_| {
            anyhow!("publisher webhook: env var `{var_name}` referenced via {{env:...}} is not set")
        })?;
        out.push_str(&value);
        rest = &after_marker[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RegistryConfig;

    fn make_ctx<'a>(registries: &'a BTreeMap<String, RegistryConfig>) -> PublishContext<'a> {
        let pkg_path = std::path::PathBuf::from(".");
        PublishContext {
            package_name: "ferrflow",
            package_path: Box::leak(Box::new(pkg_path)),
            new_version: "5.3.0",
            tag: "v5.3.0",
            registries,
            dry_run: true,
            verbose: false,
        }
    }

    #[test]
    fn interpolate_replaces_all_placeholders() {
        let registries = BTreeMap::new();
        let ctx = make_ctx(&registries);
        assert_eq!(
            interpolate("released {name}@{version} ({tag})", &ctx).unwrap(),
            "released ferrflow@5.3.0 (v5.3.0)"
        );
    }

    #[test]
    fn env_placeholder_resolves_when_set() {
        // SAFETY: This is a test-only var name that's never used elsewhere; the
        unsafe {
            std::env::set_var("FERRFLOW_TEST_WEBHOOK_TOKEN", "secret-123");
        }
        let s = resolve_env_placeholders("Bearer {env:FERRFLOW_TEST_WEBHOOK_TOKEN}").unwrap();
        assert_eq!(s, "Bearer secret-123");
    }

    #[test]
    fn env_placeholder_missing_is_an_error() {
        let err = resolve_env_placeholders("Bearer {env:__FERRFLOW_NEVER_SET_TOKEN}")
            .expect_err("must error");
        assert!(format!("{err:?}").contains("is not set"));
    }

    #[test]
    fn env_placeholder_unterminated_is_an_error() {
        let err = resolve_env_placeholders("{env:FOO").expect_err("must error");
        assert!(format!("{err:?}").contains("unterminated"));
    }

    #[test]
    fn default_body_has_package_version_tag() {
        let registries = BTreeMap::new();
        let ctx = make_ctx(&registries);
        let body = default_body(&ctx);
        assert_eq!(body["package"], "ferrflow");
        assert_eq!(body["version"], "5.3.0");
        assert_eq!(body["tag"], "v5.3.0");
    }

    fn live_ctx<'a>(registries: &'a BTreeMap<String, RegistryConfig>) -> PublishContext<'a> {
        PublishContext {
            dry_run: false,
            ..make_ctx(registries)
        }
    }

    #[test]
    fn posts_the_default_body_to_the_interpolated_url_as_json() {
        use crate::forge::test_server::{FakeServer, Reply};
        let server = FakeServer::start(vec![Reply::json(204, serde_json::json!({}))]);
        let registries = BTreeMap::new();
        let url = format!("{}/hooks/{{name}}/{{version}}", server.url());

        let outcome = run(&url, None, &BTreeMap::new(), &live_ctx(&registries)).unwrap();

        let expected_url = format!("{}/hooks/ferrflow/5.3.0", server.url());
        assert!(
            matches!(&outcome, PublishOutcome::Published { url: Some(u) } if *u == expected_url),
            "{outcome:?}"
        );
        let req = server.only_request();
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/hooks/ferrflow/5.3.0");
        assert_eq!(req.header("content-type"), Some("application/json"));
        assert_eq!(
            req.json(),
            serde_json::json!({ "package": "ferrflow", "version": "5.3.0", "tag": "v5.3.0" })
        );
    }

    #[test]
    fn a_body_template_and_headers_are_interpolated_before_sending() {
        use crate::forge::test_server::{FakeServer, Reply};
        let server = FakeServer::start(vec![Reply::json(200, serde_json::json!({}))]);
        let registries = BTreeMap::new();
        let template = serde_json::json!({
            "text": "released {name} {tag}",
            "meta": { "versions": ["{version}", 3, true] },
        });
        let headers = BTreeMap::from([
            (
                "Authorization".to_string(),
                "Bearer {env:FERRFLOW_TEST_WEBHOOK_LIVE_TOKEN}".to_string(),
            ),
            (
                "content-type".to_string(),
                "application/x-custom".to_string(),
            ),
        ]);

        {
            let _guard = crate::test_utils::ENV_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            unsafe {
                std::env::set_var("FERRFLOW_TEST_WEBHOOK_LIVE_TOKEN", "hook-secret");
            }
            run(
                &format!("{}/hook", server.url()),
                Some(&template),
                &headers,
                &live_ctx(&registries),
            )
            .unwrap();
            unsafe {
                std::env::remove_var("FERRFLOW_TEST_WEBHOOK_LIVE_TOKEN");
            }
        }

        let req = server.only_request();
        assert_eq!(req.header("authorization"), Some("Bearer hook-secret"));
        assert_eq!(req.header("content-type"), Some("application/x-custom"));
        assert_eq!(
            req.json(),
            serde_json::json!({
                "text": "released ferrflow v5.3.0",
                "meta": { "versions": ["5.3.0", 3, true] },
            })
        );
    }

    #[test]
    fn a_failing_endpoint_is_an_error_naming_the_url() {
        use crate::forge::test_server::{FakeServer, Reply};
        let server = FakeServer::start(vec![Reply::json(500, serde_json::json!({}))]);
        let registries = BTreeMap::new();
        let url = format!("{}/hook", server.url());

        let err = run(&url, None, &BTreeMap::new(), &live_ctx(&registries)).unwrap_err();

        assert!(format!("{err:#}").contains(&url), "{err:#}");
        assert_eq!(
            crate::error_code::code_from_error(&err).as_deref(),
            Some("E6102")
        );
    }

    #[test]
    fn an_unresolvable_header_aborts_before_anything_is_sent() {
        use crate::forge::test_server::{FakeServer, Reply};
        let server = FakeServer::start(vec![Reply::json(200, serde_json::json!({}))]);
        let registries = BTreeMap::new();
        let headers = BTreeMap::from([(
            "Authorization".to_string(),
            "Bearer {env:__FERRFLOW_NEVER_SET_WEBHOOK_HEADER}".to_string(),
        )]);

        let err = run(
            &format!("{}/hook", server.url()),
            None,
            &headers,
            &live_ctx(&registries),
        )
        .unwrap_err();

        assert!(format!("{err:#}").contains("is not set"), "{err:#}");
        assert!(server.requests().is_empty());
    }

    #[test]
    fn dry_run_short_circuits() {
        let registries = BTreeMap::new();
        let ctx = make_ctx(&registries);
        let outcome =
            run("https://example.com/hook", None, &BTreeMap::new(), &ctx).expect("dry-run");
        assert!(matches!(outcome, PublishOutcome::DryRun));
    }
}
