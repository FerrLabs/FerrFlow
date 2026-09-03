---
title: Monorepo
description: Version multiple packages independently in a single repository.
slug: v0/docs/configuration/monorepo
---

FerrFlow treats a repository as a monorepo when `ferrflow.toml` defines more than one `[[package]]`. Each package is versioned independently based on its own git history.

## Package isolation

FerrFlow uses path prefixes to determine which commits belong to which package. Only commits that touch files under `path` (or `shared_paths`) trigger a release for that package.

```toml
[[package]]
name = "api"
path = "packages/api"      # only commits touching packages/api/ trigger api releases

[[package]]
name = "site"
path = "packages/site"     # only commits touching packages/site/ trigger site releases
```

## Shared dependencies

If you have code shared between packages (e.g., a `packages/shared/` library), you can declare it as a `shared_paths` entry. A change to any shared path triggers a release for every package that lists it:

```toml
[[package]]
name = "api"
path = "packages/api"
shared_paths = ["packages/shared/"]   # changing shared/ also releases api

[[package]]
name = "site"
path = "packages/site"
shared_paths = ["packages/shared/"]   # and also releases site
```

## Git tag format

Each package gets its own tag namespace: `{name}@v{version}`.

```
api@v1.2.0
site@v0.4.1
```

FerrFlow looks for the most recent tag matching `{name}@v*` to determine what commits are new.

## Independent cadences

Packages release independently. In a single `ferrflow release` run:

- `api` may bump from `1.2.0` → `1.3.0` (new `feat:` commit)
- `site` may bump from `0.4.0` → `0.4.1` (only `fix:` commits)
- `shared` may not release at all (only `chore:` commits)

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Use <code>ferrflow check</code> to preview exactly which packages would be released and at what version before committing to a release.</p>
</div></aside>
