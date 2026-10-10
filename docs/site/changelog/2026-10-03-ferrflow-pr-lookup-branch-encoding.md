---
title: 'FerrFlow: release PRs are found again on branches with special characters'
summary: 'A target branch containing &, # or + broke the lookup for the existing release PR on GitHub and GitLab, so FerrFlow opened a second one. A failed branch update on GitHub also reported the wrong error.'
date: 2026-10-03T12:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/1261
---

With `releaseCommitMode: "pr"`, FerrFlow looks for an open release PR before proposing a new one. The branch name went into that query unencoded, so a branch such as `release/a+b` sent a different name to the API, the existing PR was not found, and a duplicate was opened. Branch names are now percent-encoded on both GitHub and GitLab.

On GitHub, moving the release branch fell back to creating it after any failure, so an expired token showed up as a confusing "create ref" error. FerrFlow now only creates the branch when GitHub says it does not exist, and otherwise reports the real failure under E3015.
