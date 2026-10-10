---
title: 'FerrFlow: check no longer serves a stale plan after an uncommitted edit'
summary: 'The cache behind ferrflow check keyed on the commit, the tags and the config, but not on the version files it actually reads. Editing a version without committing returned the previous answer for up to five minutes. Interrupted runs also left temp files that nothing ever cleaned up.'
date: 2026-08-28T16:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/963
docsLink: https://ferrflow.com/docs/cli/check/
---

`ferrflow check` caches its result for five minutes so repeated previews are instant. The cache key was built from the commit, the tag refs and the config file, which covers everything `check` reads from git. It does not cover everything `check` reads.

To render `1.0.0 → 1.1.0`, `check` opens the version files on disk. Change one without committing, by hand or through a `release` that wrote files before failing, and none of the three parts of the key move. The old answer came straight back, presented exactly like a fresh one. A wrong answer delivered confidently is worse than a slow one, which is the whole reason to fix a five-minute window on a preview command.

The key now includes the contents of every configured version file. We hashed contents rather than timestamps and sizes: `1.0.0` and `1.1.0` are the same length, so size alone misses precisely the edit this is meant to catch, and file timestamps are too coarse on some filesystems to be trusted for it. Reading a few small manifests costs far less than the history walk the cache is there to avoid.

Separately, the cache writes each entry to a temporary file and renames it into place. A run killed between those two steps, a cancelled CI job or a panic, left the temporary file behind, and the cleanup pass only ever looked at finished entries. On a repository with a lot of interrupted runs, `.git/ferrflow-cache/` grew without bound. Leftovers older than an hour are now cleaned up alongside everything else, while a file a write may still be using is left alone.

Nothing to change on your side. Both fixes are internal to the cache, and `ferrflow cache clear` still empties it outright.
