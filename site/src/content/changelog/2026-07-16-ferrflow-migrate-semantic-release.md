---
title: 'FerrFlow · Migrate from semantic-release in one command'
summary: A new `ferrflow migrate` command reads your existing `.releaserc` and generates the equivalent FerrFlow config — mapping tag format, branches, and the common plugins, and surfacing anything without an equivalent instead of guessing.
date: 2026-07-16T09:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/699
docsLink: https://ferrflow.com/docs/reference/cli#ferrflow-migrate
---

Switching release tools shouldn't mean rewriting your config by hand. `ferrflow migrate` reads an existing semantic-release setup and writes the equivalent `ferrflow.json` for you.

```bash
ferrflow migrate                        # auto-detects .releaserc
ferrflow migrate --from semantic-release
```

It maps what has a FerrFlow equivalent:

- `tagFormat: "v${version}"` → `tagTemplate: "v{{version}}"`
- `branches` → FerrFlow channels — `main`/`master` become the stable line, a `prerelease` branch becomes a channel
- `@semantic-release/changelog` → the package's `changelog` path
- `@semantic-release/exec` → hooks (`prepareCmd` → `preBump`, `publishCmd` → `postPublish`, and the rest)
- `@semantic-release/github` / `gitlab` → `forge`

The point is what it does with the rest. Anything without a FerrFlow equivalent is **surfaced, never guessed**: `@semantic-release/npm`, custom `commit-analyzer` release rules, and `repositoryUrl` are each reported with a note on what to do, not silently dropped. The command prints a summary of what mapped, what it ignored, and what needs a manual look — and it won't overwrite a config you already have.

```
Migrated semantic-release config from .releaserc → ferrflow.json

Mapped:
  • tagFormat → tagTemplate (v${version})
  • branch main → stable
  • branch beta → prerelease `beta`
  • @semantic-release/github → forge

Review manually:
  ! @semantic-release/npm → not mapped. Configure `publishers` by hand if you publish to npm.
```

JSON `.releaserc` is supported today; a YAML `.releaserc` or a JavaScript `release.config.js` is reported as unsupported rather than mis-read. After migrating, review the generated config, then run `ferrflow validate` and `ferrflow check`.
