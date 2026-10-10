---
title: 'FerrFlow: migrating from semantic-release keeps your tags'
summary: 'ferrflow migrate turned tagFormat "v${version}" into tagTemplate "v{{version}}", which FerrFlow renders as v{1.2.0}. It now writes "v{version}", so the migrated config finds the tags semantic-release created.'
date: 2026-09-28T10:05:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/1229
docsLink: https://ferrflow.com/docs/reference/cli
---

`ferrflow migrate` converts a semantic-release `tagFormat` into a FerrFlow `tagTemplate`. The conversion wrote the placeholder with doubled braces, `{{version}}`, while FerrFlow templates use single ones. A config migrated from `tagFormat: "v${version}"` therefore rendered tags like `v{1.2.0}`: the first release after migrating found none of the existing tags and started over.

The conversion now writes `{version}`. If you already migrated, check the `ferrflow.json` it wrote for `{{version}}` and replace it with `{version}`:

```json
{
  "workspace": {
    "tagTemplate": "v{version}"
  }
}
```

The migration table in the CLI reference and in the entry that announced `ferrflow migrate` showed the same wrong output and have been corrected.
