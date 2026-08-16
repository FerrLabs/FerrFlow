use anyhow::Result;
use colored::Colorize;
use std::collections::BTreeMap;
use std::path::Path;

use crate::config::{PublisherConfig, RegistryConfig};

pub mod cargo;
pub mod docker;
pub mod github_release_asset;
pub mod helm;
pub mod npm;
pub mod webhook;

/// Inputs threaded into every publisher invocation. References the
/// release context that the post-publish phase already has on hand —
/// no need to clone fields just to feed a publisher.
#[allow(dead_code)]
pub struct PublishContext<'a> {
    pub package_name: &'a str,
    pub package_path: &'a Path,
    pub new_version: &'a str,
    pub tag: &'a str,
    pub registries: &'a BTreeMap<String, RegistryConfig>,
    pub dry_run: bool,
    pub verbose: bool,
}

/// Result of one publisher invocation. The dispatcher renders these
/// uniformly so users see a consistent log shape across every kind.
#[derive(Debug)]
pub enum PublishOutcome {
    Published { url: Option<String> },
    Skipped { reason: String },
    DryRun,
}

/// Run every publisher declared for a package against `ctx`, rendering
/// a uniform `[kind] action … → status` line per entry. Shared by the
/// `ferrflow release` post-publish phase and the standalone `ferrflow
/// publish` command so both surface publishers identically. Returns the
/// first executor error, after which remaining publishers are skipped.
pub fn run_all(publishers: &[PublisherConfig], ctx: &PublishContext<'_>) -> Result<()> {
    if publishers.is_empty() {
        return Ok(());
    }
    tracing::info!(
        "  {} {} publishers:",
        "→".cyan(),
        publishers.len().to_string().cyan()
    );
    for p in publishers {
        let kind = p.kind_name();
        let preview = p.describe(ctx.package_name, ctx.new_version);
        match run(p, ctx) {
            Ok(PublishOutcome::Published { url }) => {
                let suffix = url.as_deref().unwrap_or("");
                tracing::info!("    [{kind}] {preview} → {} {suffix}", "published".green());
            }
            Ok(PublishOutcome::Skipped { reason }) => {
                tracing::info!("    [{kind}] {preview} → {} ({reason})", "skipped".yellow());
            }
            Ok(PublishOutcome::DryRun) => {
                tracing::info!("    [{kind}] {preview} {}", "(dry-run)".dimmed());
            }
            Err(e) => {
                tracing::error!("    [{kind}] {} {e:#}", "ERROR".red());
                return Err(e);
            }
        }
    }
    Ok(())
}

/// Dispatch one publisher entry to its executor.
pub fn run(p: &PublisherConfig, ctx: &PublishContext<'_>) -> Result<PublishOutcome> {
    match p {
        PublisherConfig::Cargo {
            registry,
            allow_dirty,
            no_verify,
            args,
        } => cargo::run(registry.as_deref(), *allow_dirty, *no_verify, args, ctx),
        PublisherConfig::Npm {
            registry,
            tag,
            access,
            args,
        } => npm::run(
            registry.as_deref(),
            tag.as_deref(),
            access.as_deref(),
            args,
            ctx,
        ),
        PublisherConfig::Docker {
            image,
            tags,
            platforms,
            context,
            dockerfile,
            sign,
            args,
        } => docker::run(
            image, tags, platforms, context, dockerfile, *sign, args, ctx,
        ),
        PublisherConfig::Helm {
            chart,
            registry,
            args,
        } => helm::run(chart, registry, args, ctx),
        PublisherConfig::GithubReleaseAsset {
            path,
            display_name,
            args,
        } => github_release_asset::run(path, display_name.as_deref(), args, ctx),
        PublisherConfig::Webhook { url, body, headers } => {
            webhook::run(url, body.as_ref(), headers, ctx)
        }
    }
}
