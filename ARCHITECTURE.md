# Architecture

One repository, three released packages. Diagrams are in [docs/diagrams/monorepo.md](docs/diagrams/monorepo.md).

## Packages

| Package | Path | Kind | Version | Tag | Licence |
| --- | --- | --- | --- | --- | --- |
| `ferrflow` | `.` | Rust crate, binary and library, plus the `ferrflow` and `@ferrflow/*` npm packages, `ferrflow-wasm`, `@ferrflow/doc` | semver | `v1.2.3` | MIT |
| `site` | `site/` | Angular + Analog site, prerendered to static nginx | calver-seq | `site@v2026.11.10` | MPL-2.0 |
| `api` | `api/` | Rust + axum API, crate `ferrflow-api`, `publish = false` | calver-seq | `api@v2026.11.6` | MPL-2.0 |

`docs/site/` is the npm package `@ferrflow/doc` (MIT). It is versioned and published with the CLI, and the site
reads it from the working tree at build time.

The Cargo workspace is `.`, `api` and `ferrflow-wasm`. `fuzz/` and `api/fuzz/` are separate crates outside it.
The pnpm workspace root is `site/` (`package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`, `.npmrc` and the
Prettier config live there), so a dependency bump touches no file that belongs to the `ferrflow` package.

## Dependencies between packages

- `api` depends on the `ferrflow` crate with `default-features = false`, so no git or process code comes along.
  `/validate`, `/preview` and `/schema` run the same code as the CLI. Today that dependency is the published
  crate (`version = "7"`, bumped by Renovate), not a path dependency. Its path form is FLOW-10.
- `site` depends on `docs/site/`. `site/scripts/sync-docs.mjs` copies the pages, the changelog entries and the
  JSON schemas from there into `site/src/content/` and `site/public/schema/` before every `dev` and `build`.
  `site/Dockerfile` copies `docs/site` into the build context for the same reason.
- Nothing depends on `site` or `api`.

## Versioning and release

`.ferrflow` declares the three packages and `workspace.nestedPackages = "exclusive"`, so a file under `site/` or
`api/` counts for that package only, and never for the root package that contains it.

The `Release` job in `ci.yml` runs `ferrflow release --draft` on every push to `main`, after the test, fixture
and benchmark jobs pass. It tags each package that has releasable commits since its last tag, with a
`latest`-style floating tag next to it: `latest` and the major (`v7`) for the CLI, `site@latest` and `api@latest`
for the others. Pushes that only touch `docs/site/**` skip the workflow, so a documentation change cannot cut a CLI
release.

`site` declares `dependsOn: ferrflow` with `propagate: patch`. `docs/site/` belongs to `ferrflow` under
`nestedPackages = "exclusive"`, and the CLI's `postBump` hook rewrites `docs/site/data/` (benchmarks, schemas) and
the `@ferrflow/doc` version into the release commit, whose `chore(release)` subject never counts as a releasable
commit. Without the dependency, nothing ever bumped `site` for those changes and its image was not rebuilt. With it,
every CLI release also tags `site@v*` on the same release commit, so the image carries the numbers that were just
embedded. Nothing depends on `site`, and the `Release` job skips `chore(release)` pushes, so this cannot loop.

Only the `v*` tag has a consumer on `main`: `publish.yml` builds the binaries, attaches the release assets, then
publishes the crate, the npm packages, `@ferrflow/wasm`, `@ferrflow/doc` and the `ghcr.io/ferrlabs/ferrflow` image.
No workflow on `main` reacts to `site@v*` or `api@v*`.

## CI on `main`

| Workflow | Runs on | Covers |
| --- | --- | --- |
| `ci.yml` | every PR and push | CLI only: `fmt`, `clippy`, `nextest` and the fixtures run on the root package. `cargo check -p ferrflow-wasm`, the MSRV check (`--workspace`), and cargo audit, deny, machete and vet look at the whole workspace and `Cargo.lock`. Nothing builds `site/` or tests `api/`. |
| `publish.yml` | `v*` tags | CLI release artefacts and registries |
| `site-docs.yml` | PRs and pushes touching `docs/site/**` | Frontmatter validation, frozen snapshot guard |
| `fuzz.yml`, `scorecard.yml`, `security-scan.yml`, `sonarqube.yml`, `dco.yml`, `pr-title.yml` | per workflow | repository wide |

The site and API image builds, per-package jobs and the change detection that decides which of them run are in
PR #1409. When it lands, this file and the diagram need the new workflows, and the `api` dependency line above
changes to a path dependency.

## Licences

Root MIT, `site/` and `api/` MPL-2.0 (their original licence from the former FerrFlow-Cloud repository),
`docs/site/` MIT. Each folder under MPL-2.0 carries its own `LICENSE`. `deny.toml` allows both MIT and MPL-2.0
for dependencies. The published crate (`include` in `Cargo.toml`) and the npm packages contain nothing from
`site/` or `api/`.
