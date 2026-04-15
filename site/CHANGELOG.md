# Changelog

## [2.5.0] - 2026-04-15

### Features

- feat(site): theme navbar colors, fix logo on Chrome, widen dropdown bridge (#260)

## [2.4.0] - 2026-04-15

### Features

- feat(site): redesign products section with zigzag column layout (#259)

## [2.3.0] - 2026-04-15

### Features

- feat(site): localStorage-based language preference with redirect (#258)

## [2.2.0] - 2026-04-15

### Features

- feat: site redesign + dashboard neutral shell + product pages + pricing (#256)

## [2.1.0] - 2026-04-15

### Features

- feat(site): add Secrets and Agent cards, update features for platform (#245)
- feat(app): add Dockerfile and Docker CI workflow (#242)
- feat(app): create dashboard package with auth and issues viewer (#233)

### Bug Fixes

- fix(config): add app package to ferrflow config (#241)

## [2.0.0] - 2026-04-15

### Breaking Changes

- chore!: switch license from MIT to MPL-2.0 (#226)

### Features

- feat(site): redesign landing page as multi-product portal (#232)

## [1.18.0] - 2026-04-15

### Features

- feat(site): document PR/MR preview comments for GitHub and GitLab (#225)

## [1.17.0] - 2026-04-15

### Features

- feat(site): document ferrflow.ts and ferrflow.js config support (#224)

## [1.16.0] - 2026-04-14

### Features

- feat(site): add error codes reference page (#223)
- feat(site): persist user-selected format preference across pages (#221)
- feat(site): add French translation for telemetry page (#216)
- feat(site): add telemetry page to v0 versioned docs (#213)

### Bug Fixes

- fix(site): update docs to reference anonymous_telemetry instead of deprecated telemetry (#220)
- fix(api): add input validation for commits_count and repo_hash (#219)
- fix: remove dead package scripts and unused concurrently dependency (#218)
- fix(playground): rename telemetry to anonymous_telemetry in config types (#217)
- fix: migrate renovate config to matchCategories syntax (#212)
- fix(playground): always resolve {name} placeholder in tag template (#211)

## [1.15.0] - 2026-04-11

### Features

- feat(site): add telemetry page to v0 versioned docs (#213)

### Bug Fixes

- fix: migrate renovate config to matchCategories syntax (#212)
- fix(playground): always resolve {name} placeholder in tag template (#211)

## [1.14.0] - 2026-04-11

### Features

- feat(site): document pipeline trigger types with examples (#210)

## [1.13.0] - 2026-04-10

### Features

- feat(site): document --force-version CLI flag (#208)

## [1.12.0] - 2026-04-10

### Features

- feat(site): show rolling 30-day metrics with animated counters (#207)

## [1.11.2] - 2026-04-10

### Bug Fixes

- fix(site): disable Google auto-translate on localized pages (#205)
- fix(playground): collapse advanced workspace fields behind toggle (#204)

## [1.11.1] - 2026-04-06

### Bug Fixes

- fix(site): sync JSON schema with source of truth (#201)

## [1.11.0] - 2026-04-05

### Features

- feat(site): document dependsOn package config for auto-bump dependents (#189)

### Bug Fixes

- fix(site): add missing og.png, favicon.ico, and PNG icons (#188)

## [1.10.0] - 2026-04-05

### Features

- feat(site): unify navbar across landing page and docs (#185)

### Bug Fixes

- fix: update renovate config to use recommended preset (#183)
- fix: remove references to non-existent packages from README (#182)

## [1.9.1] - 2026-04-05

### Bug Fixes

- fix(site): remove redundant Tools section from docs sidebar (#180)
- fix: disable skipCi in release commits (#179)

## [1.9.0] - 2026-04-04

### Features

- feat(site): show config examples in all supported formats (JSON, TOML, JSON5, YAML) (#174)
- feat(playground): interactive commit/tag simulator in the playground (#150)

### Bug Fixes

- fix(ci): add FERRFLOW_TOKEN env var to release job (#178)
- fix(playground): Update @ferrflow/wasm to v2.14.0 (#171)
- fix(playground): Update @ferrflow/wasm to v2.13.1 (#170)
- fix(playground): Update @ferrflow/wasm to v2.13.0 (#169)
- fix(api): add x-signature and x-timestamp to CORS allowed headers (#168)
- fix(playground): Update @ferrflow/wasm to v2.12.7 (#167)
- fix(playground): Update @ferrflow/wasm to v2.12.6 (#166)
- fix(playground): Update @ferrflow/wasm to v2.12.5 (#165)
- fix(playground): Update @ferrflow/wasm to v2.12.4 (#162)
- fix(playground): Update @ferrflow/wasm to v2.12.3 (#161)
- fix(playground): Update @ferrflow/wasm to v2.12.2 (#160)
- fix(playground): Update @ferrflow/wasm to v2.11.0 (#152)
- fix(playground): Update @ferrflow/wasm to v2.10.0 (#149)

## [1.8.1] - 2026-04-01

### Bug Fixes

- fix(site): update MCP server docs for stdio-only transport (#147)
- fix(playground): Update @ferrflow/wasm to v2.9.2 (#145)
- fix(playground): Update @ferrflow/wasm to v2.9.1 (#144)

## [1.8.0] - 2026-04-01

### Features

- feat(api): real-time analytics stats via SSE (#143)

## [1.7.1] - 2026-04-01

### Bug Fixes

- fix(docker): patch Alpine zlib CVE and add Renovate Docker automerge (#142)
- fix(playground): Update @ferrflow/wasm to v2.8.5 (#140)
- fix(playground): Update @ferrflow/wasm to v2.8.4 (#139)
- fix(playground): Update @ferrflow/wasm to v2.8.1 (#138)
- fix(playground): Update @ferrflow/wasm to v2.7.1 (#137)
- fix: remove broken license badge on private repo (#136)

## [1.7.0] - 2026-04-01

### Features

- feat(playground): add markdown rendering toggle in changelog preview (#135)
- feat(site): document orphanedTagStrategy config option (#134)

### Bug Fixes

- fix(playground): Update @ferrflow/wasm to v2.7.0 (#132)

## [1.6.1] - 2026-03-31

### Bug Fixes

- fix(site): correct telemetry documentation inaccuracies (#131)

## [1.6.0] - 2026-03-31

### Features

- feat(site): use tabs for supported formats page and nudge copy button (#130)

## [1.5.0] - 2026-03-31

### Features

- feat(site): show file paths as titles on code blocks (#128)
- feat(playground): hide undefined config fields with Show all toggle (#127)

### Bug Fixes

- fix(playground): Update @ferrflow/wasm to v2.6.1 (#126)
- fix(playground): Update @ferrflow/wasm to v2.6.0 (#125)

## [1.4.2] - 2026-03-31

### Bug Fixes

- fix(site): handle 404 for missing versioned pages (#124)

## [1.4.1] - 2026-03-31

### Bug Fixes

- fix(site): add telemetry page to v1 sidebar (#123)

## [1.4.0] - 2026-03-31

### Features

- feat(site): add floating tags documentation and sync schema (#122)

## [1.3.1] - 2026-03-31

### Bug Fixes

- fix(playground): Update @ferrflow/wasm to v2.5.3 (#116)

## [1.3.0] - 2026-03-31

### Features

- feat(playground): add telemetry preview panel (#109)

## [1.2.1] - 2026-03-31

### Bug Fixes

- fix(playground): Update @ferrflow/wasm to v2.5.2 (#107)

## [1.2.0] - 2026-03-31

### Features

- feat: rework landing page metrics (repos tracked, commits analyzed) (#103)
- feat(playground): run simulation live on input change (#95)

### Bug Fixes

- fix(deps): update rust crate sha2 to 0.11 (#104)
- fix(playground): Update @ferrflow/wasm to v2.3.0 (#105)
- fix(deps): update rust crate hmac to 0.13 (#102)
- fix(playground): Update @ferrflow/wasm to v2.2.2 (#98)
- fix(playground): add vite-plugin-wasm and renovate auto-merge rule (#91)
- fix(api): fix analytics events insert and remove PII columns (#90)
- fix(playground): add @ferrflow/wasm dependency (#89)

## [1.1.0] - 2026-03-31

### Features

- feat(site): add i18n support with French translation (#85)

### Bug Fixes

- fix(playground): refresh build for correct wasm
- fix(ci): remove build provenance attestation from Docker workflow (#82)

## [1.0.1] - 2026-03-31

### Bug Fixes

- fix(site): update documentation for v2 and correct github icon color (#81)

## [1.0.0] - 2026-03-30

### Breaking Changes

- chore!: switch license from MIT to MPL-2.0 (#69)

### Bug Fixes

- fix(site): correct docs accuracy issues and add hooks documentation (#73)

## [0.12.0] - 2026-03-29

### Features

- feat(site): add playground button and GitHub icon to landing navbar (#67)

### Bug Fixes

- fix(deps): update dependency shiki to v4 (#65)
- fix(deps): update dependency @astrojs/react to v5 (#64)

## [0.11.0] - 2026-03-29

### Features

- feat(playground): interactive config builder and release simulator (#63)

## [0.10.1] - 2026-03-29

### Bug Fixes

- fix(site): add version selector to custom header (#62)
- fix(api): bind to 0.0.0.0 instead of 127.0.0.1 (#60)

## [0.10.0] - 2026-03-29

### Features

- feat(site): add v1 docs versioning and update documentation (#54)

## [0.9.1] - 2026-03-29

### Bug Fixes

- fix(ci): use release event to trigger Docker builds (#58)

## [0.9.0] - 2026-03-29

### Features

- feat(api): add uptime to health endpoint (#49)

## [0.8.0] - 2026-03-29

### Features

- feat: sync schema tag prefix (#36)
- feat(site): publish JSON Schema at /schema/ferrflow.json (#27)
- feat: add live metrics section on landing page
- feat: add custom code block theme matching landing page terminal
- feat: add POST /events endpoint and enhance public stats

### Bug Fixes

- fix(api): use explicit CORS headers instead of wildcard with credentials (#46)
- fix(ci): trigger Docker builds on tag creation instead of paths-filter (#40)
- fix: remove non-existent packages from .ferrflow config (#38)
- fix(deps): update rust crate validator to 0.20 (#35)
- fix(site): sync ferrflow schema with tag_prefix field (#32)
- fix(api): commit Cargo.lock for reproducible builds (#31)
- fix(deps): update rust crate rand to 0.10 (#18)
- fix(deps): update rust crate governor to 0.10 (#17)
- fix(deps): update rust crate config to 0.15 (#15)
- fix: remove connector line in How it works section
- fix: log API version on startup
- fix: add pull-requests read permission for paths-filter
- fix: allow dead_code on AnalyticsEvent fields used via FromRow
- fix: center GitHub logo in docs header

## [0.7.0] - 2026-03-28

### Features

- feat: add custom Header with logo and bicolor text in docs navbar

## [0.6.2] - 2026-03-28

### Bug Fixes

- fix: remove shared package references from site Dockerfile

## [0.6.1] - 2026-03-28

## [0.6.0] - 2026-03-27

### Features

- feat: add SEO, sitemap, robots.txt and favicon
- feat: add versioned docs with starlight-versions, snapshot v0

### Bug Fixes

- fix: update docs links to /docs/introduction
- fix: remove eyebrow badge from hero

## [0.5.0] - 2026-03-27

### Features

- feat: merge docs into site, drop packages/docs

## [0.4.0] - 2026-03-27

### Features

- feat(docs): add starlight docs site at /docs

## [0.3.2] - 2026-03-27

### Bug Fixes

- fix: remove app.ferrflow.com links, replace with github ctас

## [0.3.1] - 2026-03-26

### Bug Fixes

- fix: add tailwind v4 css entry point and theme config
- fix: remove invalid manager keys from renovate config
- fix: update rust builder image to 1.85

## [0.3.0] - 2026-03-26

### Features

- feat(site): redesign landing page with pastel orange theme

## [0.2.2] - 2026-03-26

### Bug Fixes

- fix: use fully-qualified image names and fix API build context for buildah

## [0.2.1] - 2026-03-26

### Bug Fixes

- fix: resolve Docker build failures for api, site, and app

## [0.2.0] - 2026-03-26

### Features

- feat(app): initial release
- feat(site): initial release
- feat(api): initial release
- feat(config): migrate ferrflow config from TOML to JSON
- feat: add workflow_dispatch to allow manual CI runs with configurable steps
- feat: complete frontend implementations for app and site
- feat: full integration — scaffold site/app/mcp, Dockerfiles, CI change detection
- feat(ci): add per-package change detection and Docker build/push
- feat(mcp): scaffold TypeScript MCP server package
- feat(app): scaffold React 19 + Vite package with production Dockerfile
- feat(site): scaffold Astro 5 package with production Dockerfile
- feat: complete backend API implementation
- feat: add CI with FerrFlow release action
- feat: initialize FerrFlow monorepo with backend API foundations

### Bug Fixes

- fix: install sqlx-cli before running migrations in CI
- fix: apply rustfmt formatting
- fix: resolve clippy warnings in API (dead code, Send, VaultClient clone)
- fix(ci): fix inputs coercion bug causing all jobs to skip on push events
- fix(ci): always run tests on push to main, use change detection only for PRs
- fix(ci): add token to paths-filter and cascade shared changes to site and app
- fix(ci): add fetch-depth: 0 to changes job checkout
- fix: multiple Rust compilation errors and warnings
- fix: prevent release and build when tests fail
- fix: replace .gen() with rand::random() for Rust edition 2024 compatibility
- fix: make Rust checks optional in pre-push hook
- fix: move MCP to its own repo, remove from Application monorepo
- fix(mcp): add DOM to tsconfig lib to resolve console type error
- fix: copy pnpm-lock.yaml and all package.json in Dockerfiles
- fix: pin FerrFlow action to @v0
