---
title: 'FerrFlow: Hooks see the whole release batch'
summary: 'Release hooks now receive `allPackages`. Every package the run bumps, with its new version and bump type: so a monorepo hook can act on the batch, not just its own package.'
date: 2026-07-20T16:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/741
docsLink: https://ferrflow.com/docs/configuration/config-file
---

A hook used to see only its own package. In a monorepo that made batch-aware automation awkward: a `postBump` on one package had no idea what else was being released alongside it.

Every hook now also gets `allPackages`: the full list of packages the run is bumping, each with its resolved new version and bump type. It's the same snapshot for every package's hook and for the batch-level `onSuccess` / `onError`, computed once after versions are settled, so even the first package's hook already sees the whole release.

Shell hooks read `FERRFLOW_ALL_PACKAGES_JSON`; JS/TS function hooks get `ctx.allPackages` as a real array:

```js
export default {
  workspace: {
    hooks: {
      postBump(ctx) {
        const summary = ctx.allPackages
          .map((p) => `${p.name}@${p.version} (${p.bump})`)
          .join(', ');
        console.log(`Releasing: ${summary}`);
      },
    },
  },
};
```

For a single-package repo it's just the one entry.
