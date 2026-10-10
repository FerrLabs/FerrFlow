---
title: 'FerrFlow: Migrate from changesets, release-please & standard-version'
summary: '`ferrflow migrate` now imports configs from changesets, release-please, and standard-version too: not just semantic-release: generating an equivalent `ferrflow.json` and a report of what mapped, what was ignored, and what needs a look.'
date: 2026-07-21T13:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/745
docsLink: https://ferrflow.com/docs/reference/cli
---

Switching to FerrFlow shouldn't mean rewriting your release config by hand. `ferrflow migrate` reads your existing release tool's config and writes the equivalent `ferrflow.json`. It already understood semantic-release; now it covers the other three big ones too.

- **release-please** (`release-please-config.json`): the `packages` map becomes FerrFlow packages, each package's `release-type` picks the right version file (Cargo.toml, package.json, pyproject.toml, Chart.yaml, …), `include-component-in-tag` becomes a `tagTemplate`, and the PR flow becomes `releaseCommitMode: pr`.
- **standard-version** (`.versionrc`): `tagPrefix` becomes a `tagTemplate` and `bumpFiles` / `packageFiles` become `versionedFiles`.
- **changesets** (`.changeset/config.json`): `baseBranch` becomes `branch`, and `linked` / `fixed` become version groups.

It auto-detects the source, or you can force it with `--from release-please`. Every run prints what mapped, what it ignored, and what needs manual review. Nothing is silently guessed. For changesets it also flags the one thing that matters: changesets versions from `.changeset/*.md` files, while FerrFlow versions from conventional commits.

```bash
ferrflow migrate               # auto-detect
ferrflow migrate --from standard-version
```

Then review the generated config and run `ferrflow validate`.
