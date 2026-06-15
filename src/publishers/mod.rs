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
use std::collections::BTreeMap;
use std::path::Path;

use crate::config::{PublisherConfig, RegistryConfig};

pub mod cargo;

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
    /// Kind's executor isn't implemented in this build yet. Treated
    /// as a non-fatal warning so users can plan ahead.
    NotImplementedYet,
}

/// Dispatch one publisher entry to its executor.
pub fn run(p: &PublisherConfig, ctx: &PublishContext<'_>) -> Result<PublishOutcome> {
    match p {
        PublisherConfig::Cargo {
            registry,
            allow_dirty,
        } => cargo::run(registry.as_deref(), *allow_dirty, ctx),
        PublisherConfig::Npm { .. }
        | PublisherConfig::Docker { .. }
        | PublisherConfig::Helm { .. }
        | PublisherConfig::GithubReleaseAsset { .. }
        | PublisherConfig::Webhook { .. } => Ok(PublishOutcome::NotImplementedYet),
    }
}
