# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| latest  | Yes       |

Only the latest release receives security updates.

## Reporting a Vulnerability

If you discover a security vulnerability, please report it privately via [GitHub Security Advisories](https://github.com/FerrLabs/FerrFlow/security/advisories/new).

Do **not** open a public issue for security vulnerabilities.

You can expect an initial response within 48 hours. We will work with you to understand the issue and coordinate a fix before any public disclosure.

## What the npm package contains

Supply-chain scanners flag the `ferrflow` npm package for containing a remote URL. This section records the review so it does not have to be repeated per release.

The package publishes `bin/` only (`files: ["bin"]`), which is a single file: `bin/ferrflow.js`. It contains exactly one URL:

```js
"Install ferrflow from https://github.com/FerrLabs/FerrFlow/releases"
```

That string is display text inside a `console.error` on the unsupported-platform path. It is never fetched.

**The package cannot make a network request.** Its entire import graph is `child_process`, `fs`, `path`, `url`, `module` and `os`. There is no HTTP client, no `fetch`, and no `net`. Its only job is to resolve the platform binary from the matching `@ferrflow/*` optional dependency and `spawnSync` it.

There are no `preinstall`, `postinstall` or `prepare` scripts in the wrapper or in any platform package. The platform packages carry no JavaScript at all: the binary is placed into them by the release workflow, and they declare no `bin` and no `scripts`.

Releases ship a `SHA256SUMS`, cosign signatures and build-provenance attestations. See [Verifying releases](https://ferrflow.com/docs/verifying-releases) for how to check them yourself; the GitHub Action verifies the digest before extracting.

The npm packages are published with npm trusted publishing over OIDC. There is no long-lived npm token in the release workflow: publishing rights are bound to this repository and the `publish.yml` workflow, and npm generates a provenance attestation for every published package automatically.
