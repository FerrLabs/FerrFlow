# Changelog

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
