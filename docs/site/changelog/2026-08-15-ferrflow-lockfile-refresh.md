---
title: 'FerrFlow: lockfiles no longer go stale after a release'
summary: 'updateLockfiles could leave Cargo.lock at the previous version while the release still reported success. Both causes are fixed: the crate name is read from the manifest, and a cold registry cache no longer defeats the update.'
date: 2026-08-15T10:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/844
---

If you set `updateLockfiles: true`, FerrFlow could bump `Cargo.toml` and leave `Cargo.lock` pointing at the previous version. Nothing failed: the update error was logged as a warning, the release finished green, and the drift surfaced later as a dirty working tree or a lockfile check in CI.

There were two separate causes. FerrFlow ran `cargo update -p <name>` with the package name from your config, which is not the cargo crate name: a package called `api` holding a crate called `my-app-api` produced `package ID specification 'api' did not match any packages`. FerrFlow now reads the real crate name from the manifest it is versioning, so the two can differ freely. Separately, the update ran with `--offline`, which cannot resolve dependencies when the cargo registry cache is empty. That is the normal state of a release job that checks out and installs the toolchain without building, and it made the failure look intermittent: the same repo would update its lock on a warm runner and skip it on a cold one.

`--offline` is still tried first, because it is faster and works with no network and no private-registry credentials. When it fails, FerrFlow retries once without it.

Nothing to change on your side. If a lockfile drifted while this was broken, commit it once and releases will keep it in sync from here.
