---
title: Telemetry
description: FerrFlow no longer collects telemetry. What older versions sent, and how to opt out on those versions.
---

**FerrFlow does not collect telemetry.** Starting with v5.33, the CLI makes no network requests of its own beyond the git and forge operations you explicitly ask for. There is nothing to opt out of, no environment variable to set, and no data to worry about.

## If you run a version before v5.33

Versions up to v5.32 sent anonymous usage events (command type, commit count, a SHA-256 hash of the git remote URL) to `api.ferrflow.com`. No source code, file names, commit messages, IP-based profiles, or personal information were ever collected. On those versions you can opt out with either:

```bash
export FERRFLOW_TELEMETRY=0
# or the cross-tool convention
export DO_NOT_TRACK=1
```

or per repository in `ferrflow.json`:

```json
{ "workspace": { "anonymous_telemetry": false } }
```

## Why it was removed

The telemetry added measurable latency to every command for monorepo users, and the useful signal it provided is available without it: error diagnostics link to per-code documentation pages, and adoption is visible through release downloads. Removing it was simpler and more honest than fixing it. The `anonymous_telemetry` config key remains accepted (and ignored) so existing configurations stay valid.
