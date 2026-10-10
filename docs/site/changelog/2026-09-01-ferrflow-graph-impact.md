---
title: 'FerrFlow: see what a release drags along before you run it'
summary: 'In a monorepo, releasing a widely depended-on package pulls others out with it. `ferrflow graph --impact` names them, with the version each would get, before anything is tagged.'
date: 2026-09-01T10:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/979
docsLink: https://ferrflow.com/docs/reference/cli/
---

Releasing a package that others depend on releases those too. FerrFlow has always done this, and `ferrflow graph` has always printed the topology it works from, but the two never met: you could read the graph and work out the consequences yourself, or run the release and find out.

`--impact` answers it directly:

```bash
ferrflow graph --impact shared
```

```
shared 1.0.0 → 1.1.0 (minor, assumed)

would also release
  api                  1.0.0 → 1.1.0      minor bump, depends on shared
  web                  1.0.0 → 1.0.1      patch bump, depends on shared
  mobile               1.0.0 → 1.1.0      minor bump, through api (2 deep)
```

The bump is assumed rather than read from your commits, which is the point: the useful question is usually hypothetical. `--bump major` shows what a breaking change would cost before you write it.

Three things it accounts for that are easy to get wrong on a whiteboard. A `dependsOn` entry can set `propagate` to `none`, which keeps that dependent out entirely, or to `patch`, which caps what the edge passes on, so `web` above takes a patch from the same upstream that gives `api` a minor. Members of a `linked` or `fixed` group come along with no dependency edge between them at all, which is the one most often forgotten. And transitive dependents carry their distance, so `mobile` reads as reached through `api` rather than directly.

A preview is only worth having if it matches what happens, so this shares its propagation with the release rather than reimplementing it. The rule the cascade applies each round was extracted and both call it, which means the two cannot drift apart as the logic changes.

`--json` works here as it does for the graph, if you want to gate a workflow on the answer rather than read it.
