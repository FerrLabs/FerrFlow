---
title: 'FerrFlow · Linked and fixed package version groups'
summary: New `linked` and `fixed` workspace options let a set of monorepo packages share a version — when one bumps, the whole group moves to the same version, the way changesets and lerna do it.
date: 2026-07-17T09:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/724
docsLink: https://ferrflow.com/docs/configuration/monorepo#linked-and-fixed-version-groups
---

Until now the only way to couple packages in a monorepo was `dependsOn`, which cascades a bump but leaves each package on its own version. Some packages, though, are meant to move as one — `react` and `react-dom`, or a set of packages you publish under a single version line. `linked` and `fixed` express exactly that.

```toml
[workspace]
linked = [["react", "react-dom"]]
fixed  = [["@scope/a", "@scope/b", "@scope/c"]]
```

When any member of a group has a releasable commit, every member is bumped to the same version — the highest the group would reach. A `feat` on one package and a `fix` on another release the whole group on the minor, and a member with no commits of its own is pulled in at the shared version. Package names stay distinct; only the number is shared.

`linked` packages share a version line when they're released together. `fixed` packages are locked to an identical version forever — the same behaviour, plus `ferrflow validate` warns if their versions have drifted apart so you catch a stray manual edit before the next release realigns them.

Misconfigurations are caught before anything is written: a group with a package that isn't defined, a package listed in two groups, or a group with fewer than two members stops the release with a clear error — the same pre-flight guarantee as dependency-cycle detection.
