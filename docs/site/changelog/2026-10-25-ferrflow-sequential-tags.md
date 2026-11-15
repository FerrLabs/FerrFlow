---
title: 'FerrFlow: sequential release tags are read as releases again'
summary: 'With sequential versioning, tags like v1, v2 and v3 were mistaken for floating tags and ignored, so FerrFlow planned 0 to 1 and then refused to release because v1 already existed. It now continues from the highest tag.'
date: 2026-10-25T12:00:00Z
product: ferrflow
type: fix
docsLink: https://ferrflow.com/docs/configuration/config-file#floating-tags
---

FerrFlow decided whether a tag was a floating alias from its shape alone: any all-digit version with at most one dot. That is also exactly what a `sequential` release tag (`v3`) or a two-part `{year}.{seq}` tag (`v2026.3`) looks like, so without a versioned file to read the version from, every release tag was skipped. `ferrflow check` planned `0 -> 1 (bootstrapped)` and `ferrflow release` stopped on "tag v1 already exists".

A numeric tag is now treated as floating only when a longer release tag under the same prefix extends it, so `v1` next to `v1.2.3` is still an alias while `v1`, `v2`, `v3` on their own are releases. The current version is read from the highest of them. `floatingTags` levels that would not shorten the version are skipped too, so `["major"]` with sequential versioning no longer re-creates the release tag as its own alias.
