---
title: 'FerrFlow: `diff` now shows only the package you asked about'
summary: 'In a monorepo, `ferrflow diff <package> <from>..<to>` filters the range to commits that touched that package, so its commit list and changelog match what a release would actually produce.'
date: 2026-08-04T10:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/781
docsLink: https://ferrflow.com/docs/reference/cli
---

`ferrflow diff` compares two tags and shows you what changed between them. In a monorepo it was showing the raw git range. Every commit between the two tags, including the ones that only touched other packages. Asking about your API and being told it shipped a frontend redesign is worse than not asking.

The range is now scoped by path: a commit counts if it touched the package's `path` or one of its `sharedPaths`. That is what the commit list, the breaking-change list and the generated changelog are built from. Which is the whole point of previewing it.

This entry first said the scoping matched what `ferrflow release` produced. It did not. `release` went on collecting every commit since the package's last tag until that was fixed on 2026-08-29. Between the two dates the preview and the release disagreed, and the preview was the accurate one.

```bash
ferrflow diff api api@v1.0.0..api@v1.1.0
```

Shared paths still count, so a change to a `proto/` directory listed in the API's `sharedPaths` shows up under the API, as it should. Single-package repositories are unaffected. They own every commit, and nothing is filtered. `--json` output is scoped identically.
