---
title: Verifying releases
description: Every FerrFlow release ships Sigstore signatures, a CycloneDX SBOM, and a SLSA provenance attestation. Here's how to verify them.
---

Every release tarball, the Docker image, the binary completions archive, and the SBOM that ships alongside them are signed via [Sigstore](https://www.sigstore.dev/) keyless signing. There are no public keys to track — the signing identity is the GitHub Actions workload identity running in `FerrLabs/FerrFlow`, anchored in the public [Rekor](https://docs.sigstore.dev/logging/overview/) transparency log.

Available since v5.2.

## What ships per release

| Artifact                           | Sidecars                         |
| ---------------------------------- | -------------------------------- |
| `ferrflow-linux-x64.tar.gz`        | `.sig`, `.crt`                   |
| `ferrflow-linux-arm64.tar.gz`      | `.sig`, `.crt`                   |
| `ferrflow-linux-armv7.tar.gz`      | `.sig`, `.crt`                   |
| `ferrflow-darwin-x64.tar.gz`       | `.sig`, `.crt`                   |
| `ferrflow-darwin-arm64.tar.gz`     | `.sig`, `.crt`                   |
| `ferrflow-windows-x64.zip`         | `.sig`, `.crt`                   |
| `ferrflow-windows-arm64.zip`       | `.sig`, `.crt`                   |
| `ferrflow-completions.tar.gz`      | `.sig`, `.crt`                   |
| `sbom.cdx.json`                    | `.sig`, `.crt`                   |
| `ghcr.io/ferrlabs/ferrflow:vX.Y.Z` | Cosign signature in GHCR + Rekor |

All sidecars are downloadable from the GitHub Release page next to the binary.

## Verifying a tarball

```bash
# install cosign (one-time)
curl -L https://github.com/sigstore/cosign/releases/latest/download/cosign-linux-amd64 \
  -o /usr/local/bin/cosign && chmod +x /usr/local/bin/cosign

# download the artifact + sidecars from the release page
TAG=v5.2.3
gh release download "$TAG" --repo FerrLabs/FerrFlow \
  -p 'ferrflow-linux-x64.tar.gz*'

# verify
cosign verify-blob \
  --certificate ferrflow-linux-x64.tar.gz.crt \
  --signature   ferrflow-linux-x64.tar.gz.sig \
  --certificate-identity-regexp "https://github.com/FerrLabs/FerrFlow/.*" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  ferrflow-linux-x64.tar.gz
# → Verified OK
```

A passing verification means:

- The tarball bytes haven't been tampered with since the release workflow signed them.
- The signing identity was a workflow running in `FerrLabs/FerrFlow` triggered by GitHub Actions' OIDC issuer.
- The signature is recorded in the public Rekor log — search [search.sigstore.dev](https://search.sigstore.dev/) for the `.sig` value.

## Verifying the Docker image

```bash
cosign verify ghcr.io/ferrlabs/ferrflow:v5.2.3 \
  --certificate-identity-regexp "https://github.com/FerrLabs/FerrFlow/.*" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

## Verifying the SBOM

The SBOM (`sbom.cdx.json`) is a [CycloneDX](https://cyclonedx.org/) document listing every transitive dependency of the published binary. It's signed the same way as the tarballs:

```bash
cosign verify-blob \
  --certificate sbom.cdx.json.crt \
  --signature   sbom.cdx.json.sig \
  --certificate-identity-regexp "https://github.com/FerrLabs/FerrFlow/.*" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  sbom.cdx.json
```

Feed the verified SBOM into your scanner of choice — Grype, Trivy, Snyk, JFrog Xray, Anchore, anything CycloneDX-aware.

## SLSA build provenance

In addition to Sigstore signatures, every release also ships a [SLSA build provenance attestation](https://slsa.dev/) generated via [`actions/attest-build-provenance`](https://github.com/actions/attest-build-provenance). It records the workflow run, the source commit SHA, and the build inputs.

```bash
gh attestation verify ferrflow-linux-x64.tar.gz --repo FerrLabs/FerrFlow
```

## What's not signed

Source tarballs from `git archive` (the auto-generated "Source code (zip)" and "Source code (tar.gz)" assets on the GitHub Release page) come from GitHub, not from the release workflow, and have no `.sig`/`.crt` sidecars. If you need an attestation for source, use `gh attestation verify` against the build provenance bundle instead.

## Why this matters

- **Supply-chain attacks.** An attacker who compromises a CDN, a mirror, or pushes a typosquat package can't forge the signature — the signing identity is anchored to the GitHub Actions OIDC flow.
- **Compliance.** SOC 2 / ISO 27001 customers can attest that the binary they pulled is what their auditor approved.
- **No key management.** Nobody at FerrLabs has a private signing key to lose or rotate. The workflow proves its identity at the moment of signing.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>If you&#39;re shipping FerrFlow into a regulated environment, pin both the version <strong>and</strong> the <code>.sig</code>/<code>.crt</code> pair into your provisioning step. A future workflow compromise can&#39;t backdate a signature that already exists in Rekor.</p>
</div></aside>
