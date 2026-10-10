---
title: 'FerrFlow: the Action verifies the binary it downloads'
summary: 'Releases now ship a signed SHA256SUMS, and the GitHub Action checks the archive against it: and its build provenance: before extracting anything.'
date: 2026-08-07T09:30:00Z
product: ferrflow
type: security
prLink: https://github.com/FerrLabs/FerrFlow/pull/811
---

The FerrFlow Action installed the CLI by piping a release asset straight into `tar`. Nothing checked what came back before it was extracted and run: and it runs holding a repository write token. The release pipeline had been producing cosign signatures and build provenance attestations for a while, but nothing consumed them, and no checksum manifest was published at all.

Releases now include a `SHA256SUMS` covering every platform archive. It is generated before the signing steps, so it is itself attested and cosign-signed, and it ships alongside `SHA256SUMS.bundle`.

The Action downloads the archive to a temporary directory, verifies its digest against `SHA256SUMS`, and only then extracts. When you pin a specific version it additionally verifies build provenance with `gh attestation verify`: provenance binds to a concrete release, so it cannot be checked against the mutable `latest` URL.

**What you need to do:** nothing. This is transparent on a current version of the Action.

Two behaviours worth knowing. Releases cut before this change carry no `SHA256SUMS`; the Action warns and continues rather than breaking every workflow pinned to an older version. But a `SHA256SUMS` that exists and has no entry for the archive being installed is a hard failure: otherwise removing one line would be a trivial bypass.

You can run the same checks yourself:

```bash
cosign verify-blob \
  --bundle ferrflow-linux-x64.tar.gz.bundle \
  --certificate-identity-regexp "https://github.com/FerrLabs/FerrFlow/.*" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  ferrflow-linux-x64.tar.gz
```
