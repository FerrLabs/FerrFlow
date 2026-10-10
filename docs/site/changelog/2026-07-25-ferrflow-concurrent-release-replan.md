---
title: 'FerrFlow: Concurrent releases no longer fail with E2006'
summary: 'When two release runs race, the losing run now replans against the version the winner published instead of aborting with E2006. And no longer silently drops the other packages it was about to release.'
date: 2026-07-25T10:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/766
---

On an active repo, merging a few PRs in quick succession triggers near-simultaneous release runs. One of them could fail with `error[E2006]: Tag(s) already exist on remote pointing to a different commit`, and the message suggested deleting the divergent tag or re-running with `--force`. Both were wrong here: the tag belonged to a concurrent run's perfectly valid release, so either remedy would have destroyed or overwritten a published version.

The run rebased its release commit onto the new remote tip but kept the version plan it had computed *before* rebasing, so it still tried to create a tag another run had already taken. Worse, the failure aborted the whole run: a second package with unreleased changes was left unreleased with no signal beyond the failed job.

A losing run now resets to the remote tip and recomputes its plan against the winner's history. The bump is applied on top of whatever was just published, so a fix planned as `1.0.1` becomes `1.1.1` if a concurrent run shipped `1.1.0` in the meantime. It never lands below the latest release. Every package in the run is replanned together, so nothing is dropped. E2006 is now reserved for genuine divergence, which makes its "delete the divergent remote tag" advice correct when it does appear.
