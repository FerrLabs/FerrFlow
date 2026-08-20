---
title: 'FerrFlow · Robust BREAKING CHANGE detection'
summary: FerrFlow now recognises mixed-case, hyphenated, and lowercase `BREAKING CHANGE` footers — and a `!` typed inside the scope (`feat(api!):`) — so a real breaking change no longer slips through and skips the major bump.
date: 2026-07-18T11:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/727
docsLink: https://ferrflow.com/docs/reference/conventional-commits#accepted-footer-variants
---

A breaking change that FerrFlow doesn't recognise is quietly misclassified: it skips the major bump and ships a release that understates the change. Several real-world spellings of the `BREAKING CHANGE` marker used to fall through — now they don't.

Newly detected as breaking:

- **Case-insensitive footers**: `breaking-change:`, `breaking change:`, `Breaking Change:` — alongside the spec's `BREAKING CHANGE:` and `BREAKING-CHANGE:`.
- **`!` inside the scope**: `feat(api!):`, the common typo for `feat(api)!:`.

Detection stays strict where it matters, so a stray mention never triggers an accidental major bump: the footer still has to start a line and be followed by a colon **and a space**, the plural `BREAKING CHANGES:` and prose mentions are ignored, and a `!` that isn't right before the closing paren (`feat(a!b):`) is not a marker.

If your project used any of these variants, the next release will now correctly bump the major version. Run `ferrflow check` to preview.
