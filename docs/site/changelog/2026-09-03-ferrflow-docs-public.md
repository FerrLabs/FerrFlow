---
title: 'FerrFlow: the documentation is now open to pull requests'
summary: 'The pages behind ferrflow.com/docs moved to a public repo. A wrong flag name, a stale example or a paragraph that never quite landed can now be fixed by anyone, and the fix goes live when it merges.'
date: 2026-09-03T09:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow-Cloud/pull/908
docsLink: https://github.com/FerrLabs/FerrFlow-Docs
---

The FerrFlow CLI has been open source since the start, but its documentation was not. The pages lived in the private repo that builds ferrflow.com, so spotting a mistake in them and being able to fix it were two very different things.

They now live in [FerrLabs/FerrFlow-Docs](https://github.com/FerrLabs/FerrFlow-Docs), which is public and holds markdown and nothing else. Every page served under `/docs`, current and frozen, is in there under `content/`.

To fix something, open a pull request against that repo. The site fetches it at build time, so merging is the whole publication step and there is nothing to do afterwards.

The frozen per-version snapshots came along too, and they stay frozen. `content/docs-v5/` records what FerrFlow v5 actually documented, mistakes included, because anyone still pinned to v5 reads those pages to understand the binary they are running. CI rejects a pull request that edits one and points you at the current docs instead.
