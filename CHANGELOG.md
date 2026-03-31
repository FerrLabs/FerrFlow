# Changelog

All notable changes to `ferrflow` will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [2.6.0] - 2026-03-31

### Features

- feat(config): add configurable floating tag aliases (#189)

## [2.5.3] - 2026-03-31

## [2.5.2] - 2026-03-31

### Bug Fixes

- fix(docker): recreate bench stub before final build (#187)

## [2.5.1] - 2026-03-31

### Bug Fixes

- fix(docker): recreate wasm stub before final build (#186)

## [2.5.0] - 2026-03-31

### Features

- feat(cli): add --json flag to check command (#183)

## [2.4.0] - 2026-03-31

### Features

- feat(telemetry): sign requests with HMAC-SHA256 (#179)

### Bug Fixes

- fix(ci): use option_env for HMAC secret and add it to benchmark jobs (#182)

## [2.3.0] - 2026-03-31

### Features

- feat(telemetry): send repo_hash and commits_count in events (#178)

## [2.2.2] - 2026-03-31

### Bug Fixes

- fix: add version_bump event and use typed EventType enum (#175)

## [2.2.1] - 2026-03-31

### Bug Fixes

- fix(docker): add missing bench stub to dependency cache layer (#170)

## [2.2.0] - 2026-03-30

### Features

- feat(formats): add Helm Chart.yaml version handler (#162)

## [2.1.0] - 2026-03-30

### Features

- feat: add pre/post-release hooks (#149)

## [2.0.0] - 2026-03-30

### Breaking Changes

- chore!: switch license from MIT to MPL-2.0 and remove stale docs (#140)

## [1.2.0] - 2026-03-29

### Features

- feat: add ferrflow-wasm crate for browser-side usage (#127)
- feat(npm): add scoped platform packages for binary distribution (#123)
- feat(formats): support plain text version files (#122)

### Bug Fixes

- fix(docker): resolve workspace build and bump version to 1.1.0 (#133)
- fix(git): use FERRFLOW_TOKEN and URL credentials for push/fetch auth (#131)
- fix(git): use GITHUB_TOKEN for push/fetch credentials in CI (#129)

## [1.0.0] - 2026-03-29

### Breaking Changes

- feat(ci)!: externalize benchmarks into reusable action (#113)

### Features

- feat(config): configurable release commit strategy (#108)
- feat: add recoverMissedReleases config option for monorepo recovery (#102)
- feat(config): use camelCase for JSON config keys (#93)
- feat(bench): add Criterion micro-benchmarks with PR comments (#86)
- feat(bench): expand benchmark suite with hyperfine, stress tests, and regression detection (#84)
- Feat/tag prefix (#80)
- feat: add version and tag query commands for CI scripting (#74)
- feat: add configurable tag prefix (#72)
- feat(versioning): support per-package versioning strategies (#70)
- feat(ci): add benchmark suite comparing against competitors (#67)
- feat(config): add explicit config path and ambiguity guard (#66)
- Feat/json json5 config (#63)
- feat: add telemetry module with fire-and-forget usage stats (#61)
- Feat/json json5 config (#59)
- Feat/json json5 config (#58)
- feat: support ferrflow.json and ferrflow.json5 config formats (#57)
- Feat/status command (#41)
- feat: write release summary to GITHUB_STEP_SUMMARY (#40)
- feat(status): add status command (#34)
- Feat/GitHub action (#24)
- feat: detect default branch from git remote instead of hardcoding main (#19)
- feat: add GitHub Action for public use (#15)
- feat: create GitHub Release via API after push (#12)
- feat: implement standalone changelog command (#11)
- feat: fallback to FerrFlow identity when git user not configured
- feat: auto-commit and push after release bump
- feat: initial FerrFlow implementation

### Bug Fixes

- fix(ci): use Rust generate-fixtures instead of deleted bash script (#112)
- perf(bench): rewrite fixture generation in Rust with incremental tree building (#106)
- fix(ci): run update-major-tag on workflow_dispatch (#99)
- fix(deps): update rust crate json5 to v1 (#98)
- fix: use contact@ferrflow.com as default commit email (#95)
- fix: use plain English in error messages instead of config key names (#94)
- perf(bench): remove mono-stress fixture (too slow) (#89)
- fix(deps): update rust crate colored to v3 (#82)
- fix(ci): handle missing release in benchmark append step (#79)
- fix(ci): update release workflow and action for v{version} tag format (#75)
- fix(bench): configure git identity in fixture generator (#68)
- fix: handle orphaned release tags (#56)
- fix(deps): update rust crate toml_edit to 0.25 (#52)
- fix(deps): update rust crate quick-xml to 0.39 (#50)
- fix: vendor libgit2 in Dockerfile to fix Alpine musl build (#43)
- fix: push tags individually instead of glob refspec

## [0.4.0] - 2026-03-26

### Features

- feat: add GitHub Action for public use
- feat: detect default branch from git remote instead of hardcoding main
- feat: implement standalone changelog command
- feat: create GitHub Release via API after push
- feat: add status command
- feat: write release summary to GITHUB_STEP_SUMMARY

### Bug Fixes

- fix: vendor libgit2 and openssl to support musl and macOS cross-compilation

### Chores

- ci: release workflow now triggered by published GitHub release event

## [0.3.0] - 2026-03-24

### Features

- feat: fallback to FerrFlow identity when git user not configured

## [0.2.0] - 2026-03-24

### Features

- feat: auto-commit and push after release bump
- feat: initial FerrFlow implementation

### Bug Fixes

- fix: push tags individually instead of glob refspec
