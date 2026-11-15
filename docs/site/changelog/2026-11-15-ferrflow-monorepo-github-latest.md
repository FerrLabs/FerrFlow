---
title: 'FerrFlow: only one package of a monorepo can become GitHub''s Latest release'
summary: 'A package release in a monorepo no longer takes the Latest flag from the package that owns the repo-wide latestTag. FerrFlow now sends make_latest: false for the others, on create and when publishing a draft.'
date: 2026-11-15T16:00:00Z
product: ferrflow
type: fix
docsLink: https://ferrflow.com/docs/configuration/monorepo/
---

GitHub gives the Latest flag to the release published last, so in a monorepo a release of any package could replace the one that `releases/latest/download/...` and install scripts rely on.

When one package owns the repo-wide `latestTag` (set, with no `{name}` placeholder), every other package's release is now created and published with `make_latest: false`. That covers drafts published by a later `ferrflow release`. Single-package repos, and monorepos where no package owns the alias, send nothing and behave as before. GitLab, Gitea and Bitbucket have no such flag and are unchanged.
