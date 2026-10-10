---
title: 'FerrFlow: doctor now catches a lockfile that disagrees with its manifest'
summary: 'A package whose manifest and lockfile record different versions cannot build with `--locked`, and `ferrflow doctor` used to report it as healthy. It now says so, and warns before the drift happens.'
date: 2026-09-01T14:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/981
docsLink: https://ferrflow.com/docs/reference/cli/
---

A release bumps the version in your manifest. If the lockfile beside it is not bumped too, cargo stops building:

```
error: cannot update the lock file because --locked was passed to prevent this
```

`doctor` had nothing to say about this. It read the manifest, printed the version it found, and moved on. A repository could sit in that state for days with a green-looking report while every `--locked` build failed.

It now reports the disagreement:

```
✓ api: v2026.8.1
⚠ api lockfile: api/Cargo.lock records v6.1.0 while api/Cargo.toml says v2026.8.1; builds passing --locked will refuse to run
```

The second check is the one worth acting on, because it fires before anything breaks. `updateLockfiles` is off by default, and a repository with a lockfile sitting next to a managed manifest and the flag unset is not broken yet, it is simply waiting for the next release to break it:

```
⚠ api lockfile: api/Cargo.lock is not updated on release; set updateLockfiles so the next bump does not leave it behind
```

With the flag on and the versions agreeing you get a plain `✓`, and a package with no lockfile is not mentioned.

Version comparison applies to `Cargo.lock`. pnpm, yarn, poetry, uv, bundler and mix lock dependencies without restating the version of the package that owns them, so a mismatch of this shape cannot occur there and there is nothing to compare. Those still get the `updateLockfiles` warning, which is about the setting rather than the file.

Nothing to change on your side. `doctor` stays read-only and runs no package manager, so it works on a machine with no toolchain installed and no registry access.
