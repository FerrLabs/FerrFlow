---
title: 'FerrFlow: rehearse a release with shadow-release'
summary: 'ferrflow shadow-release runs the real release pipeline in a throwaway clone, hooks included, and stops before anything is pushed or published. Use it to check that a release would go through before running it.'
date: 2026-10-02T16:30:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/1256
docsLink: https://ferrflow.com/docs/reference/cli/
---

`--dry-run` shows what a release would change, but it stops short of the steps that can actually fail: writing the files, committing, tagging, running the hooks around them. `ferrflow shadow-release` runs those steps for real, in a temporary clone of your repository, so a broken hook or a file that will not bump shows up before the real release.

```bash
ferrflow shadow-release
```

The clone gets the versioned files, changelogs, release commit, tags and floating tags, and the pre-commit to pre-publish hooks run against it. Nothing is pushed, no forge release is created, and no publisher or post-publish hook runs. The command prints the commits, files and tags it produced, then deletes the clone. Pass `--keep` to inspect it instead. A failing step exits non-zero with its error code, so a CI job can run it as a gate.
