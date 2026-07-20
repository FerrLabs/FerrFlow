# ferrflow-wasm

FerrFlow's core functions compiled to WebAssembly, published to npm as
[`@ferrflow/wasm`](https://www.npmjs.com/package/@ferrflow/wasm) and consumed by
the ferrflow.com playground.

## Versioning

`ferrflow-wasm` is versioned in lockstep with the `ferrflow` crate — there is no
independent version line. The release pipeline is the single source of truth:

- `ferrflow-wasm/Cargo.toml` is listed in the `ferrflow` package's
  `versionedFiles` (see [`.ferrflow`](../.ferrflow)), so every `ferrflow release`
  writes the new version into it in the same commit as `Cargo.toml` and the npm
  platform manifests.
- The npm publish (`npm/scripts/publish-wasm.sh`) stamps the package version from
  the release git tag, so `@ferrflow/wasm@X.Y.Z` always matches `ferrflow@X.Y.Z`.

The playground therefore runs the same logic as the CLI at that version. When a
new major ships, bump the playground's `@ferrflow/wasm` dependency in
`FerrFlow-Cloud` to match.
