---
title: FerrFlow API
description: 'Hosted HTTP endpoints for FerrFlow: validate configs, preview version bumps, resolve the latest release, and fetch the config schema.'
---

The FerrFlow API exposes a small set of hosted HTTP endpoints under `https://api.ferrflow.com/v1/ferrflow/*`. They are backed by the same FerrFlow core the CLI runs, so `validate` and `preview` return results identical to `ferrflow validate` and `ferrflow check`. No second implementation to drift.

Every endpoint is public (no authentication) and safe to call from CI, editors, or a browser. The machine-readable contract is served at [`/v1/ferrflow/openapi.json`](https://api.ferrflow.com/v1/ferrflow/openapi.json) (OpenAPI 3.1).

`https://api.ferrlabs.com/v1/ferrflow/*` reaches the same endpoints and keeps working indefinitely. It is where the API was first published, and released CLI versions still call it. Prefer `api.ferrflow.com` in anything new.

## `GET /v1/ferrflow/health`

Liveness and version probe. Powers status dashboards.

```json
{ "status": "ok", "service": "ferrflow-api", "version": "10.17.0", "time": "2026-07-21T15:00:00Z" }
```

## `GET /v1/ferrflow/schema`

Returns the config JSON Schema (`Content-Type: application/schema+json`), served from the schema bundled in the FerrFlow release: the same bytes the CLI validates against. Sends a strong `ETag` and `Cache-Control`, so point your editor's `$schema` here:

```json
{ "$schema": "https://api.ferrflow.com/v1/ferrflow/schema" }
```

`GET /v1/ferrflow/schema/v{major}` returns the schema frozen at a CLI major (e.g. `/schema/v5`). Only the current major is served today; older majors return `404` until per-major snapshots land.

## `GET /v1/ferrflow/latest`

Resolves the latest FerrFlow release from GitHub, cached server-side. Pass `platform` to get a single asset:

```bash
curl "https://api.ferrflow.com/v1/ferrflow/latest?platform=linux-x64"
```

```json
{
  "version": "5.48.0",
  "tag": "v5.48.0",
  "platform": "linux-x64",
  "download_url": "https://github.com/FerrLabs/FerrFlow/releases/download/v5.48.0/ferrflow-linux-x64.tar.gz",
  "bundle_url": "https://github.com/FerrLabs/FerrFlow/releases/download/v5.48.0/ferrflow-linux-x64.tar.gz.bundle",
  "published_at": "2026-07-27T19:20:00Z"
}
```

Releases are signed with [Sigstore](/verifying-releases/): verify the `.bundle` rather than a checksum (releases up to v5.47.4 carry a `.sig` + `.crt` pair instead). Without `platform`, the response lists `assets` for every platform. Valid platforms: `linux-x64`, `linux-arm64`, `linux-arm`, `darwin-x64`, `darwin-arm64`, `win32-x64`, `win32-arm64`.

## `POST /v1/ferrflow/validate`

Validates a config without a repo. You send the config text and, optionally, the contents of the versioned files it references so the file-existence and version-consistency checks run. The result is identical to `ferrflow validate --json`.

```bash
curl -X POST https://api.ferrflow.com/v1/ferrflow/validate \
  -H 'content-type: application/json' \
  -d '{
    "config": "{\"package\":[{\"name\":\"app\",\"path\":\".\",\"versionedFiles\":[{\"path\":\"package.json\",\"format\":\"json\"}]}]}",
    "files": { "package.json": "{\"version\":\"1.0.0\"}" }
  }'
```

```json
{
  "valid": true,
  "config_file": null,
  "package_count": 1,
  "errors": [],
  "warnings": [],
  "suggestions": []
}
```

An invalid config is still a successful validation: the response is `200` with `"valid": false` and the offending entries. Only a malformed request body returns `400`. The optional `format` field (`json` | `json5` | `toml`) skips format inference.

## `POST /v1/ferrflow/preview`

Computes the version bumps and changelog for an explicit list of commits: the same logic as `ferrflow check`, as a service. No repo access; you pass the commits.

```bash
curl -X POST https://api.ferrflow.com/v1/ferrflow/preview \
  -H 'content-type: application/json' \
  -d '{
    "config": "{\"package\":[{\"name\":\"api\",\"path\":\".\"}]}",
    "commits": [{ "message": "feat(api): add endpoint", "hash": "a1b2" }],
    "current_versions": { "api": "1.2.3" }
  }'
```

```json
{
  "packages": [
    {
      "name": "api",
      "current": "1.2.3",
      "next": "1.3.0",
      "bump": "minor",
      "commits": [{ "hash": "a1b2", "type": "feat", "scope": "api", "breaking": false }],
      "changelog": "### Features\n- ..."
    }
  ]
}
```

In a monorepo config, each commit is assigned to a package when its `files` fall under that package's `path`. Packages with no releasable commit are omitted.
