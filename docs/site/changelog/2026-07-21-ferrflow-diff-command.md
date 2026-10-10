---
title: 'FerrFlow: Compare two versions with `diff`'
summary: 'A new `ferrflow diff v1.4.0..v1.6.0` command shows exactly what went into a version range: the commits and their bumps, the breaking changes, the files touched, and the changelog FerrFlow would generate for the range.'
date: 2026-07-21T15:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/751
docsLink: https://ferrflow.com/docs/reference/cli
---

Sometimes you want to look at a release after the fact: what actually shipped between two versions, why the bump landed where it did, or you need release notes for a range you never tagged notes for. `ferrflow diff` answers that.

```bash
ferrflow diff v1.4.0..v1.6.0            # single-package repo
ferrflow diff api v1.4.0..v1.6.0        # monorepo: name the package
```

It prints the range's commits each tagged with its own bump (`major` / `minor` / `patch` / `none`), highlights the breaking changes, summarises the files changed, and renders the changelog section for the range using your changelog config. Add `--json` for a structured object you can pipe into anything.

Each side of the range is a version or tag: `v1.4.0` resolves to the tag directly in a single-package repo, or to the package's tag (`api@v1.4.0`) in a monorepo.
