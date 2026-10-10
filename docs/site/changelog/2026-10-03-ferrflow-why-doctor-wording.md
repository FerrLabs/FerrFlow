---
title: 'FerrFlow: clearer wording in why and doctor'
summary: 'ferrflow why no longer prints "forced bump forced", and ferrflow doctor shows a declared strategy with its config spelling, such as calver-short instead of calvershort.'
date: 2026-10-03T12:10:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/1259
---

Two diagnostics printed text that did not match what you wrote. `ferrflow why` described a version pinned with `--force-version` as `Decision: forced bump forced`; it now reads `forced bump set with --force-version`. `ferrflow doctor` showed a declared versioning strategy with its hyphens dropped, so `calver-short` appeared as `calvershort`, a value the config would reject. It now prints the name exactly as you would write it in `ferrflow.json`.
