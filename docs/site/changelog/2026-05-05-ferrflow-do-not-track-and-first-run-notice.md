---
title: 'FerrFlow: DO_NOT_TRACK env var + first-run telemetry notice'
summary: 'Telemetry now honours DO_NOT_TRACK=1 alongside FERRFLOW_TELEMETRY=0, and the first invocation prints a one-line notice pointing to ferrlabs.com/telemetry.'
date: 2026-05-05T19:30:00Z
product: ferrflow
type: new
docsLink: https://ferrlabs.com/telemetry
---

Two small additions to make the FerrFlow CLI's anonymous telemetry friendlier and more discoverable.

**`DO_NOT_TRACK=1`** now disables anonymous telemetry, exactly like `FERRFLOW_TELEMETRY=0`. Same precedence: `DO_NOT_TRACK` wins if set to `1`/`true`/`yes`/`on`. Adopting the [Console Do Not Track](https://consoledonottrack.com/) standard means a single env var in your shell profile turns off telemetry for FerrFlow, Bun, Astro, Next.js, Vite, Deno and the rest of the dev tools that honour it. No need to learn one variable per CLI.

**First-run notice.** The first time you run the CLI on a machine, it prints one line to stderr:

```
Anonymous telemetry on. See https://ferrlabs.com/telemetry: opt out: FERRFLOW_TELEMETRY=0 or DO_NOT_TRACK=1
```

A marker file in `$XDG_STATE_HOME/ferrflow/.telemetry-notice-shown` (or `~/Library/Application Support/ferrflow/` on macOS, `%LOCALAPPDATA%\ferrflow\` on Windows, override with `FERRFLOW_STATE_DIR`) suppresses it on subsequent runs. We never silently re-enable telemetry between versions; if you opted out, the notice doesn't print and nothing is sent.

Part of [FerrLabs-Cloud#166](https://github.com/FerrLabs/FerrLabs-Cloud/issues/166).
