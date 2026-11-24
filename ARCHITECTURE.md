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
  `/validate`, `/preview` and `/schema` run the same code as the CLI. It is a path dependency on the root
  (`path = ".."`), so the API always builds against the CLI in the same commit.
- `site` depends on `docs/site/`. `site/scripts/sync-docs.mjs` copies the pages, the changelog entries and the
  JSON schemas from there into `site/src/content/` and `site/public/schema/` before every `dev` and `build`.
  `site/Dockerfile` copies `docs/site` into the build context for the same reason.
- Nothing depends on `site` or `api`.

## Versioning and release

`.ferrflow` declares the three packages and `workspace.nestedPackages = "exclusive"`, so a file under `site/` or
`api/` counts for that package only, and never for the root package that contains it.

The `Release` job in `ci.yml` runs `ferrflow release --draft` on every push to `main`, after the `gate` job and the
benchmark aggregate pass. It tags each package that has releasable commits since its last tag, with a
`latest`-style floating tag next to it: `latest` and the major (`v7`) for the CLI, `site@latest` and `api@latest`
for the others. Pushes that only touch `docs/site/**` skip the workflow, so a documentation change cannot cut a CLI
release.

`site` declares `dependsOn: ferrflow` with `propagate: patch`. `docs/site/` belongs to `ferrflow` under
`nestedPackages = "exclusive"`, and the CLI's `postBump` hook rewrites `docs/site/data/` (benchmarks, schemas) and
the `@ferrflow/doc` version into the release commit, whose `chore(release)` subject never counts as a releasable
commit. Without the dependency, nothing ever bumped `site` for those changes and its image was not rebuilt. With it,
every CLI release also tags `site@v*` on the same release commit, so the image carries the numbers that were just
embedded. Nothing depends on `site`, and the `Release` job skips `chore(release)` pushes, so this cannot loop.

The `v*` tag is consumed by `publish.yml`: it builds the binaries, attaches the release assets, then publishes the
crate, the npm packages, `@ferrflow/wasm`, `@ferrflow/doc` and the `ghcr.io/ferrlabs/ferrflow` image.

`site@v*` and `api@v*` are consumed by `docker.yml`. It resolves the package and version from the tag (the version
must be `X.Y.Z`), builds `site/Dockerfile` or `api/Dockerfile` with the context at the repo root, for `linux/amd64`,
and pushes `ghcr.io/ferrlabs/ferrflow-cloud/site` or `.../api` tagged `X.Y.Z` and `X.Y`. The build and the push live
in the reusable workflow `FerrLabs/.github/.github/workflows/reusable-docker-build.yml`, along with the hadolint
lint, the Trivy scan, the cosign signature and provenance attestation, and the SBOM. Once the images
are pushed, a final job publishes the draft GitHub release of that tag with `--latest=false`, so the CLI release
stays the latest one. `docker.yml` can also be started by hand (`workflow_dispatch`) with a package and a version.

## CI on `main`

| Workflow | Runs on | Covers |
| --- | --- | --- |
| `ci.yml` | every PR and push to `main` (a push touching only `docs/site/**` is ignored) | See below. |
| `changes.yml` | called by `ci.yml` | Runs `scripts/changed-paths.sh` on the diff and outputs `cli`, `api` and `site`. |
| `docker.yml` | `site@v*` and `api@v*` tags, manual | Builds and pushes the site or API image, then publishes the draft release. |
| `publish.yml` | `v*` tags | CLI release artefacts and registries |
| `site-docs.yml` | PRs and pushes touching `docs/site/**` | Frontmatter validation, frozen snapshot guard |
| `fuzz.yml`, `scorecard.yml`, `security-scan.yml`, `sonarqube.yml`, `dco.yml`, `pr-title.yml` | per workflow | repository wide |

`ci.yml` jobs, by the output that gates them:

| Gate | Jobs |
| --- | --- |
| `cli` | `test`, `kani`, `msrv`, `build`, the `fixture-*` jobs, and the micro-benchmarks on pushes to `main`. `fmt`, `clippy` and `nextest` run on the root package; `msrv` checks the whole workspace. |
| `cli` or `api` | `security` (cargo audit, deny, machete, vet), since the API shares `Cargo.lock`. |
| `api` | `check-api` (fmt, clippy, tests in `api/`), and `image-api` on PRs (builds `api/Dockerfile` without pushing). |
| `site` | `check-site` (typecheck, tests, i18n parity, build in `site/`), and `image-site` on PRs (builds `site/Dockerfile` without pushing). |
| none | `gate` waits for all of the above and fails if any failed or was cancelled; `release` needs `gate` and the benchmark aggregate. |

Skipped jobs do not fail the gate. All of them also skip on `chore(release):` pushes.

`scripts/changed-paths.sh` classifies each changed file, and `scripts/changed-paths.test.sh` checks it on every run:

| Changed file | Sets |
| --- | --- |
| `.github/workflows/{ci,changes,sonarqube,fuzz,docker}.yml`, `scripts/changed-paths.sh`, `.dockerignore` | `cli`, `api`, `site` |
| `site/*`, `docs/site/*` | `site` |
| `api/*` | `api` |
| `src/*`, `schema/*`, `ferrflow-wasm/*`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/*` | `cli`, `api` |
| anything else | `cli` |

On a PR the diff is `HEAD^1..HEAD`; on a push it is the pushed range. When no range is available (manual run, new
branch), all three are `true`.

## Licences

Root MIT, `site/` and `api/` MPL-2.0 (their original licence from the former FerrFlow-Cloud repository),
`docs/site/` MIT. Each folder under MPL-2.0 carries its own `LICENSE`. `deny.toml` allows both MIT and MPL-2.0
for dependencies. The published crate (`include` in `Cargo.toml`) and the npm packages contain nothing from
`site/` or `api/`.
