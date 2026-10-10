---
title: 'FerrFlow: a versionTemplate starting with v no longer resets on every release'
summary: 'A versionTemplate whose first literal was v never read the previous version back, so every release started again from zero. v1.2.3 with a patch bump produced v0.0.1, release after release.'
date: 2026-09-24T16:20:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/1199
docsLink: https://ferrflow.com/docs/configuration/config-file/
---

To bump a version, FerrFlow reads the previous one back through the same template that produced it. That read stripped a leading `v` from the current version before matching, which is right when the template is bare and the version came from a tag, and wrong when the template carries the `v` itself: the pattern then asked for a `v` that had just been removed, and nothing matched.

With no previous version to read, every number fell back to zero. `v1.2.3` with a patch bump became `v0.0.1`, and the release after that became `v0.0.1` again. A `{seq}` reset the same way: `v2026.8.3` became `v2026.8.1`.

`v{major}.{minor}.{patch}` is the first template most people write, so if you use one and your versions never seemed to climb, this is why.

The version is now matched as written first, and only falls back to stripping the `v` when that fails. Both shapes work: a template carrying its own prefix, and a bare template reading a tag-shaped version.

Nothing to change on your side. The next release after upgrading reads your current version correctly and continues from it.
