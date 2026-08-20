---
title: 'FerrFlow v4.2 · pre-release channels'
summary: 'Ship `alpha`, `beta`, and `rc` releases without touching your floating tags.'
date: 2026-04-29T09:00:00Z
product: 'ferrflow'
type: 'new'
docsLink: 'https://ferrflow.com/docs/guides/prerelease-channels'
---

You can now ship `alpha`, `beta`, and `rc` releases without touching your floating tags. Add `--prerelease=beta` to the CLI or set `prerelease.channel` in `.ferrflow` and FerrFlow will tag `v4.2.0-beta.1`, `v4.2.0-beta.2` and so on without bumping `latest`.

```bash
ferrflow release --prerelease=beta
```

## Why we built it

Three teams asked for it last quarter — every one of them was using a hacky `--no-publish` workaround. This fixes the workaround.

## What changes

- New `prerelease.channel` config field (string, no default).
- New `--prerelease=<channel>` CLI flag (overrides config).
- Floating tags (`latest`, `next`) stay pinned to non-prerelease versions.
- Generated changelogs include a `Pre-releases` section under each major.

No breaking changes. Existing repos get the feature for free on next bump to `v4.2.0`.
