---
title: 'FerrFlow: a changelog your hook rewrote is now what gets published'
summary: 'A `postBump` hook could already rewrite CHANGELOG.md and have it committed, but the tag, the release body and the release commit kept the text generated before the hook ran. They now agree.'
date: 2026-09-01T16:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/982
docsLink: https://ferrflow.com/docs/configuration/config-file/
---

`post_bump` runs after the changelog section is written and receives it in `FERRFLOW_CHANGELOG`. A hook can rewrite `CHANGELOG.md`, and FerrFlow stages the result into the release commit. That much already worked.

What did not is everything else that quotes the changelog. The section is built before the hook runs, and that earlier value fed the git tag, the forge release body and the release commit message. So a hook that turned commit subjects into prose gave you a file that read well and a Releases page that still said `feat(auth): support passkeys`. Same release, two texts, and the one people actually read was the raw one.

FerrFlow now reads the section back from the file after the hook, so all four carry the same thing.

If a rewrite loses the `## [version]` heading, the generated text is published instead. A hook that mangles the changelog should not also empty the release notes.

This closes the gap for anyone wanting human-written release notes without FerrFlow taking a position on how they are produced. Whether the hook calls a model, runs a script, or reformats with `sed` is not FerrFlow's business, and keeping it that way means no API key passes through the release tool, nothing breaks in air-gapped CI, and the result stays reproducible if your rewriter is.

One thing to know: none of this runs under `--dry-run`, which writes no changelog. Preview it with a real release on a branch.
