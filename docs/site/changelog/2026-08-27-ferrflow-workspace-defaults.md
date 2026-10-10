---
title: 'FerrFlow: a config without a workspace block now keeps its defaults'
summary: 'Omitting the workspace block from a config file silently dropped every workspace default. The remote became an empty string and the release failed at the push; the target branch became empty and said nothing at all.'
date: 2026-08-27T10:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/945
docsLink: https://ferrflow.com/docs/configuration/config-file/
---

The `workspace` block is optional, and a config that leaves it out is the shape most people write first:

```json
{
  "package": [
    { "name": "app", "path": ".", "versionedFiles": [{ "path": "Cargo.toml", "format": "toml" }] }
  ]
}
```

That config lost every workspace default. The failure people would actually hit came at the very end of a release, after the version bump, the changelog and the tag had all been written:

```
error[E2004]: Failed to push branch ''
  Remote '' has no URL
```

Writing `"workspace": {}` behaved correctly, and omitting the key did not, which is not a distinction anyone would expect to matter.

The cause was two sets of defaults that disagreed. Each field declared its own fallback for the case where the block is present but the key is missing, while an entirely absent block took a different path that filled everything with empty values. Only the second path was reachable from a minimal config, and only the first was tested.

The empty remote was the loud symptom. The quieter ones mattered more: the target branch, normally detected from `origin/HEAD` and falling back to `main`, became an empty string that then fed tag lookup and the release branch name without complaint, and auto-merge for release pull requests silently turned itself off despite being documented as on.

Both paths now produce the same values. Nothing to change in your configuration, and a config that already carried a `workspace` block was never affected.
