---
title: 'FerrFlow: one config file per package'
summary: 'A monorepo no longer has to declare every package in a single root config. The new include key points at per-package files, each owning its own settings, and a package path inside one of those files defaults to its directory.'
date: 2026-08-25T12:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/932
docsLink: https://ferrflow.com/docs/configuration/config-file/
---

FerrFlow looked for its config at the repository root and nowhere else, so a monorepo with twenty packages carried one long file that every team edited. The usual consequences followed: merge conflicts on an unrelated package's block, no way to give a project ownership of its own release settings through `CODEOWNERS`, and a file long enough that nobody reads it before changing it.

The `include` key splits it up. The root config keeps the workspace-level settings and points at the per-package files:

```json
{
  "workspace": { "versioning": "semver" },
  "include": ["projects/*/ferrflow.json"]
}
```

Each included file describes one package, using the same keys as a `package` entry:

```json
{ "name": "api", "dependsOn": ["core"] }
```

Paths inside an included file are relative to that file, and `path` defaults to its directory. The example above needs no `path` at all: `projects/api/ferrflow.json` describes the package in `projects/api`, and renaming or moving that directory requires no edit. Dependencies still refer to packages by name, so `dependsOn` works across files and the release order comes out of the same graph as before. Included files may use a different format than the root config, and they can sit alongside an inline `package` array while a repo migrates.

Four situations are rejected rather than quietly ignored: an `include` pattern that matches no file, two packages sharing a name, an included file that declares `workspace`, `include`, or `package`, and one that resolves to a path outside the repository. Each of them would otherwise have produced a run that did nothing and exited successfully, which on a release tool is how a team discovers weeks later that nothing shipped.
