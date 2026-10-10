---
title: 'FerrFlow: the crates.io link after a publish points at your crate again'
summary: 'FerrFlow built the crates.io link from the package name in its own config rather than the one in Cargo.toml. A workspace routinely uses short names against prefixed crates, and short crate names are long since taken, so the link led to an unrelated crate instead of yours.'
date: 2026-08-28T15:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/962
docsLink: https://ferrflow.com/docs/configuration/publishers/
---

After publishing to a registry, FerrFlow prints a link to the released version. For cargo it built that link from the package name in your FerrFlow config, not from `[package].name` in the manifest cargo actually publishes under.

In a workspace those routinely differ, because the config keeps short names while the crates carry a prefix:

```json
{ "name": "bridge", "path": "crates/bridge", "publishers": [{ "kind": "cargo" }] }
```

```toml
[package]
name = "idlewarden-bridge"
```

That produced `crates.io/crates/bridge/26.8.27` instead of `crates.io/crates/idlewarden-bridge/26.8.27`. Short crate names have been taken for years, so the link resolved to a real, unrelated crate rather than returning a 404. A log line pointing at somebody else's project as if it were the release that just went out is worse than a missing link, and it is the sort of thing that gets pasted into a release announcement.

FerrFlow now reads the name from `Cargo.toml`, and falls back to the config name when the manifest cannot be read or carries no `[package]` table. The "already exists on the registry" message uses the real name for the same reason: it makes a claim about the registry, so it has to name the crate the registry knows. Messages about which of your packages failed keep the config name, since that is the one you would look for in your config.

The publish itself was never affected. `cargo publish` runs in the package directory and cargo reads the manifest itself, so the right crate was always uploaded. This follows the same fix for the npm publisher, where a short config name against a scoped package name caused the identical problem.
