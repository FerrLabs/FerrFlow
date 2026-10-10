---
title: 'FerrFlow: your own commits set the floor for a bump, not the answer'
summary: 'A package with a fix of its own shipped as a patch even when a dependency in the same release took a minor. It now takes the stronger of the two. Expect larger bumps where a shared package and its consumers are released together.'
date: 2026-09-01T20:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/991
docsLink: https://ferrflow.com/docs/configuration/monorepo/
---

An earlier fix today made a package take the strongest bump reaching it through the dependency graph. That covered packages with nothing of their own to release. A package that had its own commits kept whatever those commits implied, however far its dependencies moved.

So a `fix` in `web`, released alongside an `api` that took a minor, shipped as a patch:

```
● shared  1.0.0 → 1.1.0  (minor)
● api     1.0.0 → 1.1.0  (minor, dependency: shared)
● web     1.0.0 → 1.0.1  (patch)
```

`web` depends on `api`. Under the default `propagate: same`, a minor in `api` is a minor for `web`, and the fix of its own does not make that smaller. Its commits set the floor. The same release now produces `web 1.0.0 → 1.1.0`.

The changelog is unchanged in content: `web` still lists its own commit, because that is still what it did. Only the version moved further.

**This reaches more releases than the earlier fix.** That one only touched packages with no commits of their own. This one applies to any package that has commits and a dependency moving in the same release, which in most monorepos is the common case rather than the edge one. If you release a shared library and its consumers together, expect the consumers to move up. Set `propagate` on the dependency to `patch` or `none` where that is not what you want.
