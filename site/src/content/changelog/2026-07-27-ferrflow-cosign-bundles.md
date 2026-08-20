---
title: 'FerrFlow · Releases are signed with a single Sigstore bundle'
summary: 'From v5.48.0 each release artifact ships one `.bundle` instead of a separate `.sig` and `.crt`. Verification is one flag shorter; older releases keep working as before.'
date: 2026-07-27T20:00:00Z
product: ferrflow
type: breaking
prLink: https://github.com/FerrLabs/FerrFlow/pull/774
docsLink: https://ferrflow.com/docs/verifying-releases
---

Sigstore's tooling moved on: cosign v3 replaced the separate signature and certificate outputs with a single bundle that carries both. FerrFlow releases follow suit. Every artifact — the platform tarballs and zips, the completions archive, the SBOM — now ships one `<artifact>.bundle` sidecar instead of a `.sig` + `.crt` pair.

Verification loses a flag:

```bash
cosign verify-blob \
  --bundle ferrflow-linux-x64.tar.gz.bundle \
  --certificate-identity-regexp "https://github.com/FerrLabs/FerrFlow/.*" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  ferrflow-linux-x64.tar.gz
```

**If you verify releases in CI, this is the change to make.** A pipeline that downloads `<artifact>.sig` and `<artifact>.crt` will fail on v5.48.0 and later, because those files are no longer published. Releases up to **v5.47.4** are untouched and still verify with `--certificate` and `--signature` — nothing was removed retroactively.

The hosted API follows the same rule: `GET /v1/ferrflow/latest` now returns `bundle_url` for releases that have one, and keeps returning `signature_url` and `cert_url` for the older releases that carry them, so a client can simply use whichever field is present.
