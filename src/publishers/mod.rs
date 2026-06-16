//! Declarative publish targets. Each [`PublisherConfig`] kind has a
//! corresponding executor that takes a [`PublishContext`] and returns
//! a [`PublishOutcome`]. Kinds whose executor isn't shipped yet
//! gracefully degrade to a `dry-run only` preview so users can declare
//! their full publishing plan in config today even if half of it
//! doesn't run yet.
//!
//! See RFC #571 + foundation PR #572. v1 ships Cargo only; npm/docker/
//! helm/asset/webhook land in follow-up PRs.

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
    /// Absolute on-disk path to the package root.
    pub package_path: &'a Path,
    pub new_version: &'a str,
    /// Full git tag created for this package (`my-pkg@v1.2.3`). Some
    /// publishers (github-release-asset, webhook) use it; cargo
    /// doesn't — but threading it through every kind uniformly avoids
    /// reshaping the dispatcher when later kinds land.
    pub tag: &'a str,
    pub registries: &'a BTreeMap<String, RegistryConfig>,
    /// True when the parent `ferrflow release` call is a dry run.
    /// Publishers should emit their preview line and skip side
    /// effects.
    pub dry_run: bool,
    /// Reserved for chatty per-publisher tracing once executors grow
    /// retry / probe steps; cargo v1 doesn't honor it yet.
    pub verbose: bool,
}

/// Result of one publisher invocation. The dispatcher renders these
/// uniformly so users see a consistent log shape across every kind.
#[derive(Debug)]
pub enum PublishOutcome {
    /// Successful publish. Optional URL surfaces in the step summary.
    Published { url: Option<String> },
    /// Target was already at the requested state — idempotent skip
    /// (e.g. cargo crate version already on the registry, npm
    /// version already published). Always preferable to surfacing
    /// a generic error.
    Skipped { reason: String },
    /// Dry-run pass: nothing happened, only the preview line was
    /// printed by the caller.
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
    println!(
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
                println!("    [{kind}] {preview} → {} {suffix}", "published".green());
            }
            Ok(PublishOutcome::Skipped { reason }) => {
                println!("    [{kind}] {preview} → {} ({reason})", "skipped".yellow());
            }
            Ok(PublishOutcome::DryRun) => {
                println!("    [{kind}] {preview} {}", "(dry-run)".dimmed());
            }
            Err(e) => {
                eprintln!("    [{kind}] {} {e:#}", "ERROR".red());
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
