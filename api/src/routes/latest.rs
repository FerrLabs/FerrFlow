use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use axum::{
    Json,
    extract::Query,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult, code};

const RELEASE_URL: &str = "https://api.github.com/repos/FerrLabs/FerrFlow/releases/latest";
const CACHE_TTL: Duration = Duration::from_secs(300);

const PLATFORMS: &[(&str, &str)] = &[
    ("linux-x64", "ferrflow-linux-x64.tar.gz"),
    ("linux-arm64", "ferrflow-linux-arm64.tar.gz"),
    ("linux-arm", "ferrflow-linux-armv7.tar.gz"),
    ("darwin-x64", "ferrflow-darwin-x64.tar.gz"),
    ("darwin-arm64", "ferrflow-darwin-arm64.tar.gz"),
    ("win32-x64", "ferrflow-windows-x64.zip"),
    ("win32-arm64", "ferrflow-windows-arm64.zip"),
];

#[derive(Debug, Deserialize)]
pub struct LatestQuery {
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetEntry {
    pub platform: String,
    pub download_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LatestResponse {
    pub version: String,
    pub tag: String,
    pub published_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Vec<AssetEntry>>,
}

#[derive(Debug, Clone)]
struct Release {
    tag: String,
    published_at: String,
    downloads: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    published_at: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

impl From<GhRelease> for Release {
    fn from(gh: GhRelease) -> Self {
        let downloads = gh
            .assets
            .into_iter()
            .map(|a| (a.name, a.browser_download_url))
            .collect();
        Release {
            tag: gh.tag_name,
            published_at: gh.published_at,
            downloads,
        }
    }
}

static CACHE: LazyLock<Mutex<Option<(Instant, Release)>>> = LazyLock::new(|| Mutex::new(None));
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

pub async fn latest(Query(query): Query<LatestQuery>) -> ApiResult<Response> {
    if let Some(platform) = query.platform.as_deref()
        && asset_name_for(platform).is_none()
    {
        return Err(ApiError::new(
            code::PLATFORM_INVALID,
            StatusCode::BAD_REQUEST,
            format!("unknown platform '{platform}'"),
        ));
    }

    let release = current_release().await?;
    let body = build_response(&release, query.platform.as_deref())?;

    Ok(([(header::CACHE_CONTROL, "public, max-age=300")], Json(body)).into_response())
}

fn asset_name_for(platform: &str) -> Option<&'static str> {
    PLATFORMS
        .iter()
        .find(|(token, _)| *token == platform)
        .map(|(_, name)| *name)
}

fn companion(release: &Release, asset: &str, extension: &str) -> Option<String> {
    release
        .downloads
        .get(&format!("{asset}.{extension}"))
        .cloned()
}

fn asset_entry(release: &Release, platform: &str, asset: &str) -> Option<AssetEntry> {
    let download_url = release.downloads.get(asset)?.clone();
    Some(AssetEntry {
        platform: platform.to_string(),
        download_url,
        bundle_url: companion(release, asset, "sigstore.json"),
        signature_url: companion(release, asset, "sig"),
        cert_url: companion(release, asset, "crt"),
    })
}

fn build_response(release: &Release, platform: Option<&str>) -> ApiResult<LatestResponse> {
    let version = release.tag.trim_start_matches('v').to_string();
    let base = LatestResponse {
        version,
        tag: release.tag.clone(),
        published_at: release.published_at.clone(),
        platform: None,
        download_url: None,
        bundle_url: None,
        signature_url: None,
        cert_url: None,
        assets: None,
    };

    if let Some(platform) = platform {
        let asset = asset_name_for(platform).expect("platform validated before fetch");
        let entry = asset_entry(release, platform, asset).ok_or_else(|| {
            ApiError::not_found(
                code::RELEASE_UPSTREAM,
                format!(
                    "release {} has no asset for platform '{platform}'",
                    release.tag
                ),
            )
        })?;
        Ok(LatestResponse {
            platform: Some(entry.platform),
            download_url: Some(entry.download_url),
            bundle_url: entry.bundle_url,
            signature_url: entry.signature_url,
            cert_url: entry.cert_url,
            ..base
        })
    } else {
        let assets = PLATFORMS
            .iter()
            .filter_map(|(token, asset)| asset_entry(release, token, asset))
            .collect();
        Ok(LatestResponse {
            assets: Some(assets),
            ..base
        })
    }
}

async fn current_release() -> ApiResult<Release> {
    if let Some(release) = cached() {
        return Ok(release);
    }
    let release = fetch_release().await?;
    store(release.clone());
    Ok(release)
}

fn cached() -> Option<Release> {
    let guard = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard
        .as_ref()
        .filter(|(at, _)| at.elapsed() < CACHE_TTL)
        .map(|(_, release)| release.clone())
}

fn store(release: Release) {
    let mut guard = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = Some((Instant::now(), release));
}

async fn fetch_release() -> ApiResult<Release> {
    let mut request = CLIENT
        .get(RELEASE_URL)
        .header(header::USER_AGENT, "ferrflow-api")
        .header(header::ACCEPT, "application/vnd.github+json");
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        request = request.bearer_auth(token);
    }

    let response = request.send().await.map_err(|e| upstream(&e))?;
    if !response.status().is_success() {
        return Err(ApiError::new(
            code::RELEASE_UPSTREAM,
            StatusCode::BAD_GATEWAY,
            format!("GitHub returned {}", response.status()),
        ));
    }
    let gh: GhRelease = response.json().await.map_err(|e| upstream(&e))?;
    Ok(gh.into())
}

fn upstream(err: &reqwest::Error) -> ApiError {
    tracing::warn!(error = %err, "ferrflow /latest upstream failure");
    ApiError::new(
        code::RELEASE_UPSTREAM,
        StatusCode::BAD_GATEWAY,
        "failed to resolve the latest FerrFlow release",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_with(extensions: &[&str]) -> Release {
        let mut downloads = BTreeMap::new();
        for (_, asset) in PLATFORMS {
            let base =
                format!("https://github.com/FerrLabs/FerrFlow/releases/download/v5.48.0/{asset}");
            downloads.insert(asset.to_string(), base.clone());
            for extension in extensions {
                downloads.insert(
                    format!("{asset}.{extension}"),
                    format!("{base}.{extension}"),
                );
            }
        }
        Release {
            tag: "v5.48.0".to_string(),
            published_at: "2026-07-27T19:20:00Z".to_string(),
            downloads,
        }
    }

    fn sample_release() -> Release {
        release_with(&["sigstore.json"])
    }

    fn legacy_release() -> Release {
        release_with(&["sig", "crt"])
    }

    #[test]
    fn platform_query_returns_single_asset_with_bundle() {
        let out = build_response(&sample_release(), Some("linux-x64")).unwrap();
        assert_eq!(out.version, "5.48.0");
        assert_eq!(out.platform.as_deref(), Some("linux-x64"));
        let download_url = out.download_url.unwrap();
        assert!(download_url.ends_with("ferrflow-linux-x64.tar.gz"));
        assert_eq!(
            out.bundle_url.unwrap(),
            format!("{download_url}.sigstore.json"),
            "the bundle must sit next to the artefact it signs"
        );
        assert!(out.signature_url.is_none());
        assert!(out.cert_url.is_none());
        assert!(out.assets.is_none());
    }

    #[test]
    fn legacy_release_keeps_signature_and_cert() {
        let out = build_response(&legacy_release(), Some("linux-x64")).unwrap();
        let download_url = out.download_url.unwrap();
        assert_eq!(out.signature_url.unwrap(), format!("{download_url}.sig"));
        assert_eq!(out.cert_url.unwrap(), format!("{download_url}.crt"));
        assert!(out.bundle_url.is_none());
    }

    #[test]
    fn serialized_response_has_no_empty_strings() {
        for release in [sample_release(), legacy_release(), release_with(&[])] {
            for platform in [Some("linux-x64"), None] {
                let json =
                    serde_json::to_string(&build_response(&release, platform).unwrap()).unwrap();
                assert!(!json.contains("\"\""), "empty string in {json}");
            }
        }
    }

    #[test]
    fn unsigned_release_omits_signature_fields() {
        let json =
            serde_json::to_value(build_response(&release_with(&[]), Some("linux-x64")).unwrap())
                .unwrap();
        for field in ["bundle_url", "signature_url", "cert_url"] {
            assert!(json.get(field).is_none(), "{field} should be omitted");
        }
    }

    #[test]
    fn windows_platform_maps_to_zip_asset() {
        let out = build_response(&sample_release(), Some("win32-x64")).unwrap();
        assert!(
            out.download_url
                .unwrap()
                .ends_with("ferrflow-windows-x64.zip")
        );
    }

    #[test]
    fn no_platform_lists_every_asset() {
        let out = build_response(&sample_release(), None).unwrap();
        let assets = out.assets.unwrap();
        assert_eq!(assets.len(), PLATFORMS.len());
        assert!(assets.iter().all(|a| a.bundle_url.is_some()));
        assert!(out.download_url.is_none());
    }

    #[test]
    fn missing_asset_for_known_platform_is_404() {
        let mut release = sample_release();
        release.downloads.remove("ferrflow-linux-x64.tar.gz");
        let err = build_response(&release, Some("linux-x64")).unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
        assert_eq!(err.code, code::RELEASE_UPSTREAM);
    }

    #[test]
    fn unknown_platform_has_no_asset_mapping() {
        assert!(asset_name_for("solaris-sparc").is_none());
        assert!(asset_name_for("linux-x64").is_some());
    }
}
