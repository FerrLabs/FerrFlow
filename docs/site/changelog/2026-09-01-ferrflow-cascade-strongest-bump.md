---
title: 'FerrFlow: a dependent now takes the strongest bump reaching it'
summary: 'A package fed by two dependency edges of different strength took whichever arrived first, so it could ship as a patch while a package it depends on moved a minor. Expect larger bumps if your graph has a diamond in it.'
date: 2026-09-01T18:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/988
docsLink: https://ferrflow.com/docs/configuration/monorepo/
---

When a release bumps a package, FerrFlow walks the dependency graph and bumps what depends on it. Each edge decides how much of the bump it passes on, through `propagate`.

A package sitting at the end of two edges of different strength took whichever reached it first, not the strongest. Say `web` depends on `shared` with `propagate: patch` and on `api` with the default. A feature on `shared` gives `api` a minor, and `web` should follow with a minor through that edge. It shipped as a patch instead, because the capped edge from `shared` is the shorter path and got there first:

```
web  1.0.0 → 1.0.1  (patch, dependency: shared, api)
```

The line already named both edges, which is what makes it awkward: FerrFlow knew about the minor and did not use it. Within a single step it always took the strongest of the edges available. The rule only broke down across steps.

It now settles: the walk repeats until no package can be improved, so the same release produces `web 1.0.0 → 1.1.0`.

**Some releases get larger.** If your graph has a diamond, a package fed by a strong and a weak path will now take the strong one, where before it depended on which path was shorter. Worth a look before upgrading if you have automation keyed to major or minor. Repositories with a straight dependency chain and no diamonds are unaffected, since there was never a second bump competing.

`ferrflow graph --impact` follows the same rule, so you can see what a release would now do before running one.
