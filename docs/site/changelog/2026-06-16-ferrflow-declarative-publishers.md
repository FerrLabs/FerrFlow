---
title: 'FerrFlow: declarative publishers: publish to cargo, npm, docker, helm & more on release'
summary: 'FerrFlow can now publish your packages to registries declaratively from ferrflow.json: cargo, npm, docker (multi-arch + Sigstore), helm, GitHub release assets and webhooks: replacing hand-rolled postPublish shell hooks. Shipped across FerrFlow v5.4–v5.7.'
date: 2026-06-16T10:00:00Z
product: ferrflow
type: new
docsLink: https://ferrflow.com/docs/configuration/config-file/#publishers
---

FerrFlow now publishes your packages to registries declaratively. Instead of wiring a `postPublish` shell hook that shells out to `cargo publish`, `npm publish`, or `docker buildx`, you describe the intent in `ferrflow.json` and FerrFlow runs it after the GitHub Release is created: per package, idempotently, and crash-resume safe.

Six publisher kinds are supported:

- **cargo**: `cargo publish` to crates.io or a private registry, with `allowDirty` and `noVerify`.
- **npm**: `npm publish` with a scoped, throwaway `.npmrc` so your project's `.npmrc` is never touched.
- **docker**: `docker buildx build --push` with multi-arch (`{version}`/`{major}`/`{minor}`/`latest` tag templates), and optional `sign: sigstore` (`cosign`).
- **helm**: `helm package` + `helm push` to an OCI registry.
- **github-release-asset**: attach a sidecar file (SBOM, signature, …) to the release with `--clobber`.
- **webhook**: POST to Slack/Discord/your own endpoint with `{name}`, `{version}`, `{tag}` and `{env:NAME}` interpolation.

```json title="ferrflow.json"
{
  "workspace": {
    "registries": {
      "kellnr": { "tokenEnv": "CARGO_REGISTRIES_KELLNR_TOKEN" }
    }
  },
  "package": [{
    "name": "my-crate",
    "path": "crates/my-crate",
    "publishers": [
      { "kind": "cargo", "registry": "kellnr", "noVerify": true }
    ]
  }]
}
```

Registry tokens are referenced by env-var name (`tokenEnv`): the token value never lives in the config file, so `ferrflow.json` stays safe to commit. Every command publisher also takes an `args` array, a verbatim escape hatch for flags FerrFlow doesn't model natively (`--locked`, `--provenance`, `--build-arg`, …).

Publishers are idempotent: an already-published version is detected and skipped, so a re-run after a partial failure picks up exactly where it left off. The block is additive: your existing `postPublish` hooks keep working while you migrate. See the [publishers documentation](https://ferrflow.com/docs/configuration/config-file/#publishers) for the full reference.
