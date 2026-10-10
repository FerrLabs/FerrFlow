---
title: 'FerrFlow: a schema for per-package config files'
summary: 'Files pulled in through include now have their own JSON schema, so editors autocomplete and validate them like the root config. It is served at ferrflow.com/schema/ferrflow-package.json and bundled in the binary behind ferrflow schema --package.'
date: 2026-09-22T08:07:34Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/1127
docsLink: https://ferrflow.com/docs/reference/cli/
---

The `include` key lets a monorepo keep one config file per package, but those files had no schema. Pointing their `$schema` at `ferrflow.json` did not help: that schema describes the root config, with `workspace`, `include` and a `package` array, so an editor flagged every key of a single package as unexpected and offered no completion at all.

Since FerrFlow 7.23.0 an included file has a schema of its own:

```json
{
  "$schema": "https://ferrflow.com/schema/ferrflow-package.json",
  "name": "api",
  "dependsOn": ["core"]
}
```

It accepts exactly the keys of a `package` entry, with `name` required and `path` optional, since `path` defaults to the file's directory. Anything else is rejected, so a misspelled key shows up in the editor before FerrFlow ever runs. The two schemas are checked against each other in FerrFlow's test suite, so the package schema cannot fall behind when a new package field lands.

For repos that would rather not fetch it, the same schema ships inside the binary:

```bash
ferrflow schema --package --pretty --output ferrflow-package.schema.json
```

Then point `$schema` at the local file.
