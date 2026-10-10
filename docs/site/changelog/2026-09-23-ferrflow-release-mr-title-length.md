---
title: 'FerrFlow: the release pull request title now fits the forge'
summary: 'A monorepo releasing enough packages at once built a title longer than the 255 characters a merge request accepts, so the release PR was never opened and the run still exited green. The title is capped, the full list stays in the description, and a rejected PR now fails the release.'
date: 2026-09-23T09:12:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/1175
docsLink: https://ferrflow.com/docs/ci/pipeline-triggers/
---

In `releaseCommitMode: pr`, the pull request title is `chore(release): ` followed by every package being released. Nothing bounded it, and forges do: 255 characters on GitLab, Gitea and Forgejo, 256 on GitHub. A monorepo releasing five packages with long names crosses that line, GitLab answers 400, and the release PR is never opened.

The title is now capped at 255, the lowest of the four, so one title works on every forge. When the list does not fit, FerrFlow keeps as many packages as it can and ends the title with `and N more`. Nothing is lost: the description has always listed every tag on its own line, and still does.

The second half of this is the part you could not see. A release pull request that never got opened only produced a warning, so the branch was pushed, no PR existed, and `ferrflow release` exited 0 with CI green. That is now an error, because a PR-mode release that opened no pull request has released nothing. It covers the forge rejecting the PR, the lookup for an existing one failing, and FerrFlow not reaching a forge at all. Check that last one before you upgrade: a release job with a missing token pushed the branch and said nothing at all, and will now fail. Gitea, Forgejo and Bitbucket stay on a warning only where the forge itself refuses the pull request, since PR mode is not implemented there yet; an unreachable forge fails there too, because a missing token is caught before the forge is known.

The error also says what the forge said. FerrFlow used to drop the response body and report only `failed to created MR: E3102`, which left you guessing. It now reads:

```
Failed to create MR from ferrflow/release-main to main: http status 400: {"message":["Title is too long (maximum is 255 characters)"]}
```
