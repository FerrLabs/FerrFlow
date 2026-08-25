# Verifying FerrFlow releases

Every release tarball, the Docker image, and the SBOM that ships
alongside them are signed via [Sigstore](https://www.sigstore.dev/)
keyless signing. No public keys to track — the signing identity is the
GitHub Actions workload identity, anchored in the Rekor transparency
log.

## What ships per release

| Artifact | Sidecar |
|---|---|
| `ferrflow-linux-x64.tar.gz` | `.sigstore.json` |
| `ferrflow-linux-arm64.tar.gz` | `.sigstore.json` |
| `ferrflow-darwin-x64.tar.gz` | `.sigstore.json` |
| `ferrflow-darwin-arm64.tar.gz` | `.sigstore.json` |
| `ferrflow-windows-x64.zip` | `.sigstore.json` |
| `sbom.cdx.json` | `.sigstore.json` |
| `ghcr.io/ferrlabs/ferrflow:vX.Y.Z` | Cosign signature recorded in
GHCR + Rekor |

All sidecars are downloadable from the GitHub Release page next to the
binary.

> **Releases up to v5.47.4** ship a `.sig` + `.crt` pair instead of a single
> `.sigstore.json`. Verify those with `--certificate <file>.crt --signature <file>.sig`
> in place of `--bundle`. The switch came with cosign v3, which deprecated the
> separate signature and certificate outputs in favour of one bundle.

## Verifying a tarball

```bash
# install cosign (one-time)
curl -L https://github.com/sigstore/cosign/releases/latest/download/cosign-linux-amd64 \
  -o /usr/local/bin/cosign && chmod +x /usr/local/bin/cosign

# download the artifact + sidecars from the release page
TAG=v5.48.0
gh release download "$TAG" --repo FerrLabs/FerrFlow \
  -p 'ferrflow-linux-x64.tar.gz*'

# verify
cosign verify-blob \
  --bundle ferrflow-linux-x64.tar.gz.sigstore.json \
  --certificate-identity-regexp "https://github.com/FerrLabs/FerrFlow/.*" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  ferrflow-linux-x64.tar.gz
# → Verified OK
```

A passing verification means:

- The tarball bytes haven't been tampered with since the release
  workflow signed them.
- The signing identity was a workflow running in `FerrLabs/FerrFlow`
  triggered by GitHub Actions' OIDC issuer.
- The signature is recorded in the public Rekor log (
  https://search.sigstore.dev/ — search for the artifact's digest).

## Verifying the Docker image

```bash
cosign verify ghcr.io/ferrlabs/ferrflow:v5.0.1 \
  --certificate-identity-regexp "https://github.com/FerrLabs/FerrFlow/.*" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

## Verifying the SBOM

The SBOM (`sbom.cdx.json`) is a [CycloneDX](https://cyclonedx.org/)
document listing every transitive dependency of the published binary.
It's signed the same way as the tarballs:

```bash
cosign verify-blob \
  --certificate sbom.cdx.json.crt \
  --signature   sbom.cdx.json.sig \
  --certificate-identity-regexp "https://github.com/FerrLabs/FerrFlow/.*" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  sbom.cdx.json
```

Feed the verified SBOM into your scanner of choice (Grype, Trivy,
Snyk, JFrog Xray, Anchore — all CycloneDX-aware).

## Why this matters

- **Supply chain attacks**: an attacker who compromises a CDN, a
  mirror, or a malicious typosquat package can't forge the signature
  because the signing identity is anchored to the GitHub Actions OIDC
  flow.
- **Compliance**: SOC2/ISO27001 customers can attest that the binary
  they pulled is what their auditor approved.
- **No key management**: nobody at FerrLabs has a private signing key
  to lose or rotate. The workflow proves its identity at the moment of
  signing.

## What's NOT signed

- Source tarballs from `git archive` — these come from GitHub, not from
  the release workflow. If you need an attestation for source, use
  `gh attestation verify` on the build provenance.

## Provenance (SLSA)

In addition to Sigstore signatures, every release also ships a
[SLSA build provenance attestation](https://slsa.dev/) via
`actions/attest-build-provenance`. Verify with:

```bash
gh attestation verify ferrflow-linux-x64.tar.gz --repo FerrLabs/FerrFlow
```

Tracks the workflow run, the source commit SHA, and the build inputs.
