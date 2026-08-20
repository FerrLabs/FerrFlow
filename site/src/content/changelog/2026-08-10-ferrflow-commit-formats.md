---
title: 'FerrFlow · commit formats are configurable, and the defaults are more permissive'
summary: 'workspace.commitFormats lets you declare which commit subjects map to which bump level. The defaults now also recognise Feat:, Fix/, feature: and friends — which is why this ships as v7.'
date: 2026-08-10T18:00:00Z
product: ferrflow
type: breaking
prLink: https://github.com/FerrLabs/FerrFlow/pull/824
docsLink: https://ferrflow.com/docs/configuration/config-file
---

FerrFlow only ever recognised strict lowercase conventional prefixes. Teams whose history uses `Feat/add-login`, `Fix: resolve crash` or `feature:` — the shapes that come out of branch-named squash merges — got no releases at all, with nothing in the output explaining why.

`workspace.commitFormats` now declares the mapping. Each of `major`, `minor` and `patch` takes a pattern, a list of patterns, or `"all"`:

```json
{
  "workspace": {
    "commitFormats": {
      "minor": ["feat:*", "feat(?*):*", "Feat/*"],
      "patch": "all",
      "caseSensitive": false
    }
  }
}
```

`*` matches any run of characters (including `/`, since commit subjects are prose and routinely contain file paths), `?` matches exactly one. Resolution runs major → minor → patch, first match wins. `"all"` is a catch-all, useful for repos that follow no convention at all: `"patch": "all"` releases on every commit while `minor` and `major` patterns still promote the ones that match.

**What changes without you doing anything.** The built-in defaults are now permissive: `Feat:`, `Feat/`, `feature:`, `Fix:`, `Fix/`, `Refactor:` and `Refactor/` all trigger releases, alongside the lowercase conventional forms that already did. If your repository has commits in those shapes, they will start producing versions where they previously produced none. That is the breaking part, and the reason this is v7.

To keep the old strictness, declare it:

```json
{
  "workspace": {
    "commitFormats": {
      "minor": ["feat:*", "feat(?*):*"],
      "patch": ["fix:*", "fix(?*):*", "perf:*", "perf(?*):*", "refactor:*", "refactor(?*):*"]
    }
  }
}
```

Breaking markers are not configurable and never needed to be: `feat!:`, `fix(api)!:`, the `feat(api!):` typo and a `BREAKING CHANGE:` footer are detected structurally, whatever patterns you set. A glob cannot express them precisely — `*!:*` would match any subject containing `!:` anywhere, turning `fix: handle the !: token in the parser` into a major release.

**Action tag:** `v7` is the current floating major. Update `uses: FerrLabs/FerrFlow@v6` to `@v7`; `v6` will not receive further releases.
