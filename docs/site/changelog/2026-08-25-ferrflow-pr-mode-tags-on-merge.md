---
title: 'FerrFlow: release PRs are tagged when they merge, not when they open'
summary: 'In pr mode FerrFlow tagged and published the moment it opened the release PR, on the commit before the version bump. The PR then reported nothing left to release and was never updated again. Tagging now happens after the merge, on the commit that carries the bump.'
date: 2026-08-25T16:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/936
docsLink: https://ferrflow.com/docs/configuration/config-file/
---

`releaseCommitMode: "pr"` is meant to propose a release and let a human accept it. It did not work that way. The run that opened the pull request also pushed the tags and published the GitHub releases, on the current tip of the target branch, which is the commit *before* the version bump. The bump itself lived on the PR branch and was tagged by nothing.

Two things followed from that. A tag such as `v1.1.0` pointed at a tree whose version files still declared `1.0.0`. And because the versions were already tagged, every later run reported "Nothing to release", so the release PR was never regenerated: it froze at the version it was opened with and quietly stopped tracking the branch. If the PR was never merged, the tags and releases were public anyway, for a release that never happened.

`pr` mode now runs in the two phases it always described. The proposing run computes the bump, writes the version files and changelog onto the release branch, and opens or updates the PR, without creating a single tag. Every new commit on the target branch regenerates that branch, so the open PR keeps showing the version that would ship right now. The finalising run happens after the merge: FerrFlow finds the `chore(release):` commit on the target branch, sees that the versions it carries have no tags, and tags exactly those before publishing the releases.

The versions are read from the version files rather than recomputed, which matters more than it sounds. Simply not tagging early would have left the merged release PR looking like fresh unreleased work, so the next run would have proposed another bump on top, and no version would ever have been tagged. Squash merges and merge commits both work, and re-running after a successful release does nothing.

One limit is worth stating: a package declared without `versionedFiles` has no version to read on the target branch, so it is not finalised this way. Use `commit` mode for tag-only packages.

Repositories already using `pr` mode are not repaired by the upgrade. Their existing tags sit one commit behind the bump, and any release published for a PR that never merged is still there. Both need removing by hand before the next clean release.
