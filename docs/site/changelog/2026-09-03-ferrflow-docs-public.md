---
title: 'FerrFlow: the documentation is now open to pull requests'
summary: 'The pages behind ferrflow.com/docs live in the public FerrFlow repository. A wrong flag name, a stale example or a paragraph that never quite landed can now be fixed by anyone, and the fix goes live when it merges.'
date: 2026-09-03T09:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow-Cloud/pull/908
docsLink: https://github.com/FerrLabs/FerrFlow/tree/main/docs/site
---

The FerrFlow CLI has been open source since the start, but its documentation was not. The pages lived in the private repo that builds ferrflow.com, so spotting a mistake in them and being able to fix it were two very different things.

They now live in [`docs/site`](https://github.com/FerrLabs/FerrFlow/tree/main/docs/site) of the public FerrFlow repository, next to the code they describe. Every page served under `/docs`, current and frozen, is in there.

To fix something, open a pull request against that repo. The pages ship in the `@ferrflow/doc` package and the site picks them up at build time, so there is nothing to do once the change is released.

The frozen per-version snapshots came along too, and they stay frozen. `docs/site/docs-v5/` records what FerrFlow v5 actually documented, mistakes included, because anyone still pinned to v5 reads those pages to understand the binary they are running. CI rejects a pull request that edits one and points you at the current docs instead.
