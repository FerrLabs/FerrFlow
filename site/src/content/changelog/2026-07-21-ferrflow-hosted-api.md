---
title: 'FerrFlow · Hosted API for validate, preview, schema and releases'
summary: 'A new hosted API at api.ferrlabs.com/v1/ferrflow/* lets you validate a config, preview version bumps and changelog, fetch the config JSON Schema, and resolve the latest release — over HTTP, with results identical to the CLI.'
date: 2026-07-21T16:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrLabs-Cloud/pull/668
docsLink: https://ferrflow.com/docs/reference/api
---

Some things are easier over HTTP than by shelling out to the CLI — a web validator, an editor integration, a PR-preview comment, a "is there a newer version?" check. FerrFlow now exposes a small hosted API for exactly those, under `https://api.ferrlabs.com/v1/ferrflow/*`. Every endpoint is public (no auth) and safe to call from CI, an editor, or a browser.

The important part: `validate` and `preview` run the **same FerrFlow core the CLI runs**, so their results are identical to `ferrflow validate` and `ferrflow check` — there's no second implementation to drift out of sync.

```bash
# Preview the bump + changelog for a set of commits, no repo needed
curl -X POST https://api.ferrlabs.com/v1/ferrflow/preview \
  -H 'content-type: application/json' \
  -d '{
    "config": "{\"package\":[{\"name\":\"api\",\"path\":\".\"}]}",
    "commits": [{ "message": "feat(api): add endpoint" }],
    "current_versions": { "api": "1.2.3" }
  }'
```

The full set: `POST /validate` (hosted config validation), `POST /preview` (bump + changelog), `GET /schema` (the config JSON Schema, straight from the source of truth — point your editor's `$schema` at it), `GET /latest` (latest release + per-platform download and signature URLs), and `GET /health`. The machine-readable contract lives at `/v1/ferrflow/openapi.json`. See the [API reference](https://ferrflow.com/docs/reference/api) for every endpoint.
