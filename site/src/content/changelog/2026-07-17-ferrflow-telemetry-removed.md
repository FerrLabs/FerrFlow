---
title: 'FerrFlow · Telemetry removed — the CLI no longer phones home'
summary: FerrFlow no longer collects any usage telemetry. The module is gone from the binary entirely, along with the network calls it made on every command — which also makes monorepo commands dramatically faster.
date: 2026-07-17T15:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/716
docsLink: https://ferrflow.com/docs/legal/telemetry
---

Starting with v5.33, FerrFlow makes no network requests of its own beyond the git and forge operations you explicitly ask for. The anonymous-usage telemetry that previous versions sent on each command has been removed outright — not made opt-in, removed. There is nothing to disable and no data to think about.

There is a performance angle too, and it is not small. The old telemetry sent one event per bumped package and blocked the CLI's exit until every request completed. On a 50-package monorepo `check`, that was the difference between 1.5 seconds and 80 milliseconds — most of what felt like "work" was the CLI waiting on its own telemetry. Removing it made the wait disappear for every command, on every repository.

Existing configurations need no changes: the `anonymousTelemetry` key (and its `telemetry` alias) remains accepted and simply does nothing, so no config warns or fails validation. The `FERRFLOW_TELEMETRY` and `DO_NOT_TRACK` environment variables still work on older versions — see the updated telemetry page for what ≤v5.32 sent and how to opt out there.
