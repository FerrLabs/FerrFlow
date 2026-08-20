---
title: 'FerrFlow · ferrflow publish auto-scopes to the triggering tag'
summary: ferrflow publish now figures out which package to publish from the tag that triggered the run — so one workflow publishes each package on its own tag, no per-package wiring. It also takes multiple package names, and a --all flag to force everything.
date: 2026-06-25T11:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/602
docsLink: https://ferrflow.com/docs/reference/cli/#ferrflow-publish
---

`ferrflow publish` runs your declarative publishers in a separate CI job. Until now it published one named package or all of them — which meant a tag-triggered job had to parse `api@v2.2.1` and pass `api` itself. No more: `ferrflow publish` reads the triggering tag (`GITHUB_REF` / `CI_COMMIT_TAG`) and publishes just that package.

So a single workflow handles a whole monorepo — every package publishes on its own tag, with zero per-package wiring:

```yaml title=".github/workflows/publish.yml"
on:
  push:
    tags: ['v*', '*@v*'] # single-repo and monorepo per-package tags
jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: FerrLabs/FerrFlow@v5
        with:
          mode: publish # self-scopes to the tag that fired
```

The selection rules:

- **`ferrflow publish`** — triggering tag → that package; otherwise every package (unchanged, so the existing release→publish job is unaffected).
- **`ferrflow publish api web`** — explicit packages (now more than one).
- **`ferrflow publish --all`** — every package, ignoring any tag scope.

Scope is by package name / tag, not conventional-commit scope, and the version is still read from package state — the tag's version suffix is just informational.
