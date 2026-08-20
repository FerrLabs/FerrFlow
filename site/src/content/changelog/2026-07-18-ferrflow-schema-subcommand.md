---
title: 'FerrFlow · ferrflow schema'
summary: A new `ferrflow schema` subcommand prints the config JSON schema that is now bundled into the binary — so editor tooling and pre-commit hooks can validate `.ferrflow.json` completely offline.
date: 2026-07-18T10:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/726
docsLink: https://ferrflow.com/docs/reference/cli#ferrflow-schema
---

The FerrFlow config schema has always been published at `ferrflow.com/schema/ferrflow.json` for editor autocompletion — great in VS Code or Zed, useless offline or in a pre-commit hook that can't reach the internet. `ferrflow schema` fixes that by shipping the schema inside the binary.

```bash
ferrflow schema                       # compact JSON to stdout
ferrflow schema --pretty              # formatted
ferrflow schema --pretty --output ferrflow.schema.json
```

Write it to a file and point your editor at the local copy with `"$schema": "./ferrflow.schema.json"`, or generate it in CI to validate `.ferrflow.json` with no network call. The schema is embedded straight from the source of truth, so the bundled copy always matches the version you're running, and the command parses it before printing — a corrupt build artefact exits non-zero rather than emitting garbage.
