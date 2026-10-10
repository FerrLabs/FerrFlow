---
title: 'FerrFlow: Faster repeat runs via commit-graph'
summary: 'On large repositories, FerrFlow now writes a git commit-graph on the first run so that later `check` and `release` walks are materially faster: a benefit that repeat users (pre-commit hooks, local dev, warm CI runners) get for free.'
date: 2026-07-18T12:00:00Z
product: ferrflow
type: perf
prLink: https://github.com/FerrLabs/FerrFlow/pull/728
---

FerrFlow's tag scans and commit walks already read a git commit-graph when one exists, but a fresh clone. Every CI checkout: has a pack and no graph, so the speedup never kicked in where FerrFlow runs most.

Now, on a cold `check` (cache miss) or a real `release`, if the repository has no commit-graph and a history larger than ~1000 commits, FerrFlow writes one with `git commit-graph write --reachable`. On a 200-package, 10k-commit monorepo that trims the walk-heavy portion of a run by roughly 30% on every subsequent invocation.

It's transparent and best-effort: the write is skipped once a graph exists (so it's a one-time cost), skipped on small histories where it wouldn't pay off, skipped entirely on `--dry-run` so that stays read-only, and it never fails your command if git can't write the graph.
