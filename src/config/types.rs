use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ForgeKind {
    #[default]
    Auto,
    #[serde(alias = "GitHub")]
    Github,
    #[serde(alias = "GitLab")]
    Gitlab,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct HooksConfig {
    #[serde(alias = "preBump")]
    pub pre_bump: Option<String>,
    #[serde(alias = "postBump")]
    pub post_bump: Option<String>,
    #[serde(alias = "preCommit")]
    pub pre_commit: Option<String>,
    #[serde(alias = "prePublish")]
    pub pre_publish: Option<String>,
    #[serde(alias = "postPublish")]
    pub post_publish: Option<String>,
    #[serde(default, alias = "onFailure")]
    pub on_failure: Option<OnFailure>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum OnFailure {
    #[default]
    Abort,
    Continue,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum OrphanedTagStrategy {
    #[default]
    Warn,
    TreeHash,
    Message,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BranchChannelConfig {
    pub name: String,
    #[serde(default)]
    pub channel: ChannelValue,
    #[serde(default, alias = "prereleaseIdentifier")]
    pub prerelease_identifier: PrereleaseIdentifier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChannelValue {
    Stable(bool),
    Named(String),
}

impl Default for ChannelValue {
    fn default() -> Self {
        ChannelValue::Stable(false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PrereleaseIdentifier {
    #[default]
    Increment,
    Timestamp,
    ShortHash,
    TimestampHash,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReleaseCommitMode {
    #[default]
    Commit,
    Pr,
    None,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReleaseCommitScope {
    #[default]
    Grouped,
    PerPackage,
}

/// Declarative registry-credential map shared across every publisher
/// kind. Lets users say "the Kellnr registry's token is in
/// CARGO_REGISTRIES_KELLNR_TOKEN" once at the workspace level instead
/// of restating it on every package's publisher entry.
///
/// `tokenEnv` is the env-var name (not the value) — the actual token
/// stays in the runner's secret store. `url` is informational and used
/// for the dry-run preview message + future idempotency probes.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
pub struct RegistryConfig {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(alias = "tokenEnv")]
    pub token_env: Option<String>,
}

/// Declarative replacement for the shell-soup that currently lives in
/// `postPublish` hooks across every FerrLabs cloud repo. Each
/// publisher is a self-contained intent ("push this package to that
/// registry"), evaluated in declaration order after the release
/// commit + tag + GitHub Release have been created. v1 ships parsing
/// and dry-run preview only — actual publish execution lands in
/// follow-up PRs (one publisher kind per PR) so the schema can be
/// reviewed before any side-effecting code does.
///
/// The kind is the externally-tagged discriminator (`{"kind":
/// "cargo", ...}`) — keeps the JSON Schema readable and lets us add
/// kinds without overloading existing fields.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PublisherConfig {
    /// `cargo publish` to crates.io or a custom registry (Kellnr,
    /// Cloudsmith, …). `registry` references the
    /// `workspace.registries.<name>` map; the default `crates-io` is
    /// implicit.
    Cargo {
        #[serde(default)]
        registry: Option<String>,
        /// Mirrors `cargo publish --allow-dirty`. Default false.
        #[serde(default, rename = "allowDirty")]
        allow_dirty: bool,
    },
    /// `npm publish` to npmjs.org, GitHub Packages, or a custom
    /// registry. `registry` references workspace registries.
    Npm {
        #[serde(default)]
        registry: Option<String>,
        /// Distribution tag passed to `npm publish --tag`. Defaults to
        /// `"latest"` for stable, `"<channel>"` for prereleases — the
        /// publisher fills in at execution time.
        #[serde(default)]
        tag: Option<String>,
        /// `--access public|restricted` for scoped packages.
        #[serde(default)]
        access: Option<String>,
    },
    /// Docker `buildx` push with optional Sigstore signing.
    Docker {
        /// Fully-qualified image base, without the tag (e.g.
        /// `ghcr.io/ferrlabs/auth`).
        image: String,
        /// Tag templates. `{version}`, `{major}`, `{minor}`, `latest`
        /// are recognized.
        #[serde(default = "default_docker_tags")]
        tags: Vec<String>,
        /// Target platforms for buildx multi-arch. Defaults to
        /// `linux/amd64` (single-arch) when omitted.
        #[serde(default)]
        platforms: Vec<String>,
        /// Build context (relative to the package path). Defaults to
        /// `"."` (the package root).
        #[serde(default = "default_docker_context")]
        context: String,
        /// `Dockerfile` location relative to `context`. Defaults to
        /// `"Dockerfile"`.
        #[serde(default = "default_dockerfile")]
        dockerfile: String,
        /// Sigstore keyless signing. `sigstore` = cosign keyless,
        /// `none` = no signature.
        #[serde(default)]
        sign: DockerSign,
    },
    /// OCI helm chart push.
    Helm {
        /// Chart directory relative to the package path.
        #[serde(default = "default_helm_chart_path")]
        chart: String,
        /// Target OCI registry, e.g. `oci://ghcr.io/ferrlabs/charts`.
        registry: String,
    },
    /// Attach an extra file to the GitHub Release that
    /// `ferrflow release` just created. Useful for SBOMs, signed
    /// blobs, install scripts, etc.
    GithubReleaseAsset {
        /// Path to the file to upload, relative to the package path.
        path: String,
        /// Override the asset's filename on GitHub. Defaults to the
        /// path's basename.
        #[serde(default, rename = "displayName")]
        display_name: Option<String>,
    },
    /// Generic POST notifier — Slack incoming webhook, Discord,
    /// custom internal services. The JSON body has access to
    /// `{name}`, `{version}`, `{tag}`, `{url}` placeholders.
    Webhook {
        url: String,
        /// JSON body template. If omitted, FerrFlow sends a default
        /// payload `{"package": "...", "version": "..."}`.
        #[serde(default)]
        body: Option<serde_json::Value>,
        /// Header map. Values support `{env:NAME}` interpolation for
        /// secrets, evaluated at publish time.
        #[serde(default)]
        headers: std::collections::BTreeMap<String, String>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DockerSign {
    #[default]
    None,
    Sigstore,
}

fn default_docker_tags() -> Vec<String> {
    vec!["{version}".to_string()]
}

fn default_docker_context() -> String {
    ".".to_string()
}

fn default_dockerfile() -> String {
    "Dockerfile".to_string()
}

fn default_helm_chart_path() -> String {
    ".".to_string()
}

impl PublisherConfig {
    /// Short human-friendly name (`"cargo"`, `"docker"`, …) for log
    /// lines, dry-run output, and step summaries. Stable enough to be
    /// pattern-matched in CI scripts.
    pub fn kind_name(&self) -> &'static str {
        match self {
            PublisherConfig::Cargo { .. } => "cargo",
            PublisherConfig::Npm { .. } => "npm",
            PublisherConfig::Docker { .. } => "docker",
            PublisherConfig::Helm { .. } => "helm",
            PublisherConfig::GithubReleaseAsset { .. } => "github-release-asset",
            PublisherConfig::Webhook { .. } => "webhook",
        }
    }

    /// One-line description for the dry-run preview ("would publish
    /// X to Y"). Resolves placeholders that only depend on per-call
    /// inputs (name + new version); registry-level fields stay
    /// unresolved at this stage.
    pub fn describe(&self, package_name: &str, new_version: &str) -> String {
        match self {
            PublisherConfig::Cargo { registry, .. } => {
                let reg = registry.as_deref().unwrap_or("crates-io");
                format!("cargo publish {package_name}@{new_version} → {reg}")
            }
            PublisherConfig::Npm { registry, tag, .. } => {
                let reg = registry.as_deref().unwrap_or("npmjs.org");
                let tag = tag.as_deref().unwrap_or("latest");
                format!("npm publish {package_name}@{new_version} → {reg} (tag={tag})")
            }
            PublisherConfig::Docker {
                image,
                tags,
                platforms,
                sign,
                ..
            } => {
                let resolved: Vec<String> = tags
                    .iter()
                    .map(|t| t.replace("{version}", new_version))
                    .collect();
                let platforms_s = if platforms.is_empty() {
                    "linux/amd64".to_string()
                } else {
                    platforms.join(",")
                };
                let sign_s = match sign {
                    DockerSign::None => "",
                    DockerSign::Sigstore => " +sigstore",
                };
                format!(
                    "docker push {image}:[{}] platforms={platforms_s}{sign_s}",
                    resolved.join(", ")
                )
            }
            PublisherConfig::Helm { chart, registry } => {
                format!("helm push {chart} {new_version} → {registry}")
            }
            PublisherConfig::GithubReleaseAsset { path, display_name } => {
                let shown = display_name.as_deref().unwrap_or(path);
                format!("upload {shown} → GitHub Release {new_version}")
            }
            PublisherConfig::Webhook { url, .. } => {
                format!("POST {url} ({package_name}@{new_version})")
            }
        }
    }
}
