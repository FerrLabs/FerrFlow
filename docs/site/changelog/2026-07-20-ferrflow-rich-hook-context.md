---
title: 'FerrFlow: Richer context for release hooks'
summary: Release hooks now receive the rendered changelog, the parsed commits, the list of bumped files, and whether the run is a monorepo or pre-release. Both as `FERRFLOW_*` env vars and on the JS/TS `ctx` object.
date: 2026-07-20T12:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/736
docsLink: https://ferrflow.com/docs/configuration/config-file
---

Hooks used to see only the basics: package name, old and new version, bump type, tag. That was enough to echo a message, but not enough to post a real release note or gate on what actually changed.

Every hook now gets the full picture. Shell hooks read new `FERRFLOW_*` environment variables; JS/TS function hooks get the same data as properties on the `ctx` object (both are built from one source, so they never drift):

- `changelog`: the rendered changelog section for this bump, in markdown.
- `commits`: the commits in the bump as structured objects (`hash`, `message`, `type`, `scope`, `breaking`), so you can branch on a breaking change instead of grepping a string.
- `bumpedFiles`. Every file the release actually modified, with its format.
- `monorepo` and `isPrerelease`: flags to tell a workspace release apart from a single-package one, and a channel pre-release from a stable one.

`commits` and `bumpedFiles` arrive as real arrays in function hooks (`FERRFLOW_COMMITS_JSON` / `FERRFLOW_BUMPED_FILES_JSON` as JSON for shell hooks, ready to pipe through `jq`):

```js
export default {
  workspace: {
    hooks: {
      postBump(ctx) {
        const breaking = ctx.commits.filter((c) => c.breaking);
        if (breaking.length) notify(`${breaking.length} breaking change(s) in ${ctx.newVersion}`);
      },
    },
  },
};
```

The enrichment reaches every hook point: pre/post bump, commit, tag, publish, release, plus `onSuccess` / `onError`.
