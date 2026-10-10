---
title: 'FerrFlow: JSON output names date and sequence strategies as you configure them'
summary: 'check --json and release --json reported bump_type as calvershortseq for a calver-short-seq package, and why --json did the same for strategy. They now use the configured spelling, such as calver-short-seq. Scripts matching the old value need updating.'
date: 2026-10-07T09:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/1293
---

For packages on a date or sequence strategy, the `bump_type` field of `ferrflow check --json` and `ferrflow release --json`, and the `strategy` field of `ferrflow why --json`, carried the strategy name with its hyphens dropped: `calvershort`, `calverseq`, `calvershortseq`. None of those is a value `ferrflow.json` accepts, and `ferrflow doctor` already printed the hyphenated form.

They now match the config: `calver-short`, `calver-seq`, `calver-short-seq`. `calver`, `sequential` and the semver bump types (`major`, `minor`, `patch`) are unchanged, and so is `FERRFLOW_BUMP_TYPE` in hooks. If a script compares `bump_type` or `strategy` against the old spelling, switch it to the hyphenated one.
