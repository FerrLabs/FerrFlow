---
title: 'FerrFlow: One persistent release PR'
summary: 'In `releaseCommitMode: pr`, FerrFlow now keeps a single long-lived release pull request and updates it on each new commit, instead of opening a fresh PR: and leaving a stale one. Every time the version changes.'
date: 2026-07-21T10:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/744
docsLink: https://ferrflow.com/docs/configuration/config-file
---

`releaseCommitMode: pr` opens your version bump and changelog as a pull request instead of committing straight to the branch. Until now that PR was tied to the version it was opened for, so a second `feat:` landing before you merged spawned a _new_ PR and left the first one stale. You'd end up with a pile of `v1.3.0`, `v1.4.0`, … release PRs to clean up.

Now it's one PR. FerrFlow keeps a single release branch per target: `ferrflow/release-<target-branch>`: and on every new commit it recomputes the version and changelog and force-pushes that same branch, so the open PR's title and body update in place. This is the release-please / Changesets model.

`autoMergeReleases` (default `true`) re-enables auto-merge on each update, and is a no-op when off: the PR just waits for a human. PR mode works on GitHub and GitLab.

It also won't stomp on your work: if you push a commit onto the release branch yourself: a review fix, say: FerrFlow notices it isn't a `chore(release):` commit, warns, and leaves the branch and PR untouched for that run rather than force-pushing over it.
