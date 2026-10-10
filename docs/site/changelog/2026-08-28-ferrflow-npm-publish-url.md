---
title: 'FerrFlow: the npm link after a publish points at your package again'
summary: 'FerrFlow built the npmjs.com link from the package name in its own config rather than the one in package.json. In a monorepo those differ by default, and the short names are all taken on npm, so the link led to a stranger''s package instead of yours.'
date: 2026-08-28T14:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/961
docsLink: https://ferrflow.com/docs/configuration/publishers/
---

After publishing to npm, FerrFlow prints a link to the released version. It built that link from the package name in your FerrFlow config, not from the `name` field npm actually publishes under.

In a monorepo those are rarely the same. The usual shape is a short name in the config and a scoped name in the manifest:

```json
{ "name": "site", "path": "site", "versionedFiles": [{ "path": "site/package.json", "format": "json" }] }
```

```json
{ "name": "@acme/site" }
```

That produced `npmjs.com/package/site` instead of `npmjs.com/package/@acme/site`. And because short names like `site`, `api` and `core` are long since taken on npm, the link resolved to somebody else's package rather than returning a 404, which is the kind of wrong that goes unnoticed for a while.

FerrFlow now reads the name from `package.json`, and falls back to the config name when the manifest cannot be read. The "already exists on the registry" message uses the real name too, for the same reason: it makes a claim about what is on npm, so it has to name the package npm knows.

The publish itself was never affected. `npm publish` runs inside the package directory and npm reads the manifest itself, so the right thing was always uploaded. Only what FerrFlow printed afterwards was wrong.
