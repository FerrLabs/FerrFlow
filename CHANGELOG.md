# Changelog

All notable changes to `ferrflow` will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [2.20.2] - 2026-04-09

### Bug Fixes

- fix(ci): download cross binary instead of compiling from git (#315)

## [2.20.1] - 2026-04-09

### Bug Fixes

- fix(branches): match wildcard patterns across / in branch names (#313)

## [2.20.0] - 2026-04-08

### Features

- feat: auto-resolve branch name in detached HEAD (CI environments) (#310)

## [2.19.3] - 2026-04-08

### Bug Fixes

- fix(versioning): use UTC instead of local time for CalVer strategies (#307)

## [2.19.2] - 2026-04-08

### Bug Fixes

- fix: use CI env vars as fallback for branch detection in detached HEAD (#304)
- fix: handle detached HEAD in CI and GitLab auto-merge fallback (#303)

## [2.19.1] - 2026-04-06

## [2.19.0] - 2026-04-06

### Features

- feat(forge): support self-hosted GitHub Enterprise and GitLab instances (#299)

## [2.18.0] - 2026-04-06

### Features

- feat(config): support ferrflow.ts and ferrflow.js (#291)

## [2.17.0] - 2026-04-05

### Features

- feat(config): auto-bump dependent packages in monorepo (#290)

## [2.16.0] - 2026-04-05

### Features

- feat(fixtures): add head branch auto-detection test definitions (#289)

## [2.15.5] - 2026-04-05

### Bug Fixes

- fix(release): publish orphaned drafts when nothing was bumped (#287)

## [2.15.4] - 2026-04-05

### Bug Fixes

- fix(ci): revert to tag push trigger with CI filter for release commits (#286)

## [2.15.3] - 2026-04-05

### Bug Fixes

- fix(ci): add comment to publish trigger (#285)

## [2.15.2] - 2026-04-04

### Bug Fixes

- fix: publish orphaned draft releases and enable skipCi (#282)

## [2.15.1] - 2026-04-04

### Bug Fixes

- fix(ci): filter floating tags from publish trigger and dedupe benchmark section (#280)

## [2.15.0] - 2026-04-04

### Features

- feat: benchmark tool_configs, reset version, disable skipCi (#279)
- feat: migrate benchmark definitions to tool_configs format (#276)
- feat(test): fixture-based integration tests (#268)

### Bug Fixes

- fix(npm): set license to MIT and include README in published packages (#278)
- fix(git): filter floating tags from tag resolution (#266)
- fix: fall back to GITHUB_TOKEN/GITLAB_TOKEN for git push credentials (#263)

## [2.15.2] - 2026-04-04

### Bug Fixes

- fix(npm): set license to MIT and include README in published packages (#278)

## [2.15.1] - 2026-04-04

## [2.15.0] - 2026-04-04

### Features

- feat(test): fixture-based integration tests (#268)

## [2.14.3] - 2026-04-04

## [2.14.2] - 2026-04-04

### Bug Fixes

- fix(git): filter floating tags from tag resolution (#266)

## [2.14.1] - 2026-04-04

### Bug Fixes

- fix: fall back to GITHUB_TOKEN/GITLAB_TOKEN for git push credentials (#263)

## [2.14.0] - 2026-04-04

### Features

- feat: draft release support for GitHub (#260)

## [2.13.1] - 2026-04-04

### Bug Fixes

- fix(telemetry): send package_name and package_version in release events (#252)

## [2.13.0] - 2026-04-03

### Features

- feat: group release output by package instead of by phase (#248)

## [2.12.7] - 2026-04-03

### Bug Fixes

- fix: telemetry dry run (#246)

## [2.12.6] - 2026-04-03

### Bug Fixes

- fix(telemetry): send events regardless of dry-run mode (#243)

## [2.12.5] - 2026-04-03

### Bug Fixes

- fix: correct GitHub org slug in Cargo.toml and README (#240)
- fix(monorepo): replace .unwrap() with proper error handling in package lookup (#239)

## [2.12.4] - 2026-04-03

### Bug Fixes

- fix(git): correct orphaned tag strategy documentation URL (#238)

## [2.12.3] - 2026-04-02

### Bug Fixes

- fix(git): override CI-embedded credentials when FERRFLOW_TOKEN is set (#237)

## [2.12.2] - 2026-04-02

### Bug Fixes

- fix(git): use oauth2 username for GitLab push instead of x-access-token (#235)

## [2.12.1] - 2026-04-02

### Bug Fixes

- fix(release): target current branch for pre-release commits and PRs (#233)

## [2.12.0] - 2026-04-02

### Features

- feat: pre-release channels (alpha, beta, rc, dev) (#228)

## [2.11.0] - 2026-04-02

### Features

- feat: add GitLab support for releases, merge requests, and auto-merge (#226)

## [2.10.0] - 2026-04-01

### Features

- feat(cli): add validate command with local and remote source support (#219)

## [2.9.2] - 2026-04-01

### Bug Fixes

- fix(telemetry): join spawned threads before process exit (#217)

## [2.9.1] - 2026-04-01

### Bug Fixes

- fix(docker): upgrade Alpine packages to patch zlib CVE (#215)

## [2.9.0] - 2026-04-01

### Features

- feat: add shell completions (bash, zsh, fish, powershell, elvish) (#213)

## [2.8.6] - 2026-04-01

### Bug Fixes

- fix(telemetry): normalize remote URL before hashing repo identifier (#211)

## [2.8.5] - 2026-04-01

### Bug Fixes

- fix(ci): handle grep exit code in benchmark append step (#208)

## [2.8.4] - 2026-04-01

### Bug Fixes

- fix(release): use GraphQL API for auto-merge instead of REST merge endpoint (#204)

## [2.8.3] - 2026-04-01

### Bug Fixes

- fix(ci): wait for GitHub release propagation before appending benchmarks (#203)

## [2.8.2] - 2026-04-01

### Bug Fixes

- perf(ci): use pre-built binaries for Docker publish (#202)

## [2.8.1] - 2026-04-01

### Bug Fixes

- fix(ci): build ferrflow from source instead of downloading from releases (#200)

## [2.8.0] - 2026-04-01

### Features

- feat(formats): add csproj format handler for .NET project files (#198)

## [2.7.1] - 2026-04-01

### Bug Fixes

- fix(git): handle tags pointing to orphaned commits after rebase + force-push (#197)

## [2.7.0] - 2026-03-31

### Features

- feat(ci): include benchmark results in GitHub release notes (#192)

### Bug Fixes

- fix(ci): repair YAML syntax in release benchmark step (#193)

## [2.6.1] - 2026-03-31

### Bug Fixes

- fix(git): split branch and tag pushes with per-refspec error detection (#190)

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
