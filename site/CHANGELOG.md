# Changelog

## [5.22.0] - 2026-05-02

### Features

- feat(site): add FerrFleet to footer products grid (#438)

## [5.21.1] - 2026-05-01

### Bug Fixes

- fix(site): align docs with editorial brand + fix navbar collapse (#436)

## [5.21.0] - 2026-05-01

### Features

- feat(site): add legal pages — mentions, privacy, terms, cookies (EN+FR) (#435)

## [5.20.1] - 2026-05-01

### Bug Fixes

- fix(site): hero italic 'Automatically.' was bleeding into terminal column at wide viewports (#433)

## [5.20.0] - 2026-05-01

### Features

- feat(site): port editorial bundle design to marketing surface (Fraunces, paper, shared Navbar/Footer) (#432)

## [5.19.0] - 2026-04-30

### Features

- feat(site): rebuild after design revert (force release) (#428)

## [5.18.0] - 2026-04-30

### Features

- feat(site): adopt Claude/Anthropic design language on the marketing surface (#424)

### Bug Fixes

- fix(ci): pnpm/action-setup@v6 needs standalone:true on self-hosted runner (#425)

## [5.17.0] - 2026-04-30

### Features

- feat(ci): i18n parity + key resolution check (closes #420) (#421)

### Bug Fixes

- fix(site): disable language toggle on pages without a mirror (#422)

## [5.16.0] - 2026-04-29

### Features

- feat(ci): i18n parity + key resolution check (closes #420) (#421)

## [5.15.0] - 2026-04-29

### Features

- feat(site): Claude-style docs chrome (#419)

## [5.14.0] - 2026-04-29

### Features

- feat(site): editorial scrollbar + translate=no on code blocks (#417)

## [5.13.0] - 2026-04-29

### Features

- feat(site): re-add lang pill + auto-redirect FR browsers on first visit (#416)

## [5.12.0] - 2026-04-29

### Features

- feat(site): finish editorial refactor — stats strip, features grid, install grid, dark CTA (#415)

## [5.11.1] - 2026-04-29

### Bug Fixes

- fix(docker): refresh lockfile + restore --frozen-lockfile (#414)

## [5.11.0] - 2026-04-29

### Features

- feat(site): brand-glyph favicon matching the navbar logo (#412)
- feat(site): editorial Navbar + Footer matching the design bundle (#411)

### Bug Fixes

- fix(docker): switch site runtime to nginx-unprivileged (non-root + 8080) (#413)

## [5.10.0] - 2026-04-29

### Features

- feat(site): editorial Navbar + Footer matching the design bundle (#411)

## [5.9.1] - 2026-04-29

### Bug Fixes

- fix(docker): use --no-frozen-lockfile to match CI install behaviour (#410)

## [5.9.0] - 2026-04-28

### Features

- feat(site): editorial hero on the FerrFlow landing (#405)

## [5.8.0] - 2026-04-23

### Features

- feat(site): docs Header uses shared @ferrlabs/ui-astro Navbar (delete 555-line local copy) (#403)

## [5.7.0] - 2026-04-23

### Features

- feat: marketing landing uses unified @ferrlabs/ui-astro chrome (#402)

## [5.6.1] - 2026-04-23

### Bug Fixes

- fix(site): drop radial gradient from cta, keep solid primary-950 bg (#397)

## [5.6.0] - 2026-04-23

### Features

- feat(site): restore 2-line structured CTA with 'Ready to ship / without the ceremony' (#396)

## [5.5.0] - 2026-04-22

### Features

- feat(site): revert hero copy to release/automatically, drop stale cli.* keys (#391)
- feat(site): home page becomes cli landing, drop multi-product marketing (#390)

### Bug Fixes

- fix(ci): drop version: 10 from pnpm/action-setup, defer to package.json packageManager (#392)
- fix(ci): regenerate pnpm-lock.yaml after api/app/admin were moved out (#387)
- fix(ci): pin pnpm version for action-setup@v5 (#386)
- fix(api): mirror monorepo layout in Dockerfile for include_str CHANGELOG paths (#382)
- fix: add module-level doc headers to api and app entrypoints (#381)
- fix(ci): install build-essential on Lint API job (#380)
- fix: add per-package READMEs for api and app (#379)
- fix(ci): run buildah with --isolation=chroot (#378)

## Unreleased

### Changed

- Home page (`/` and `/fr/`) now shows the CLI-focused landing that was previously at `/cli`. Old `/cli` URLs 301-redirect to `/`. FerrFlow is versioning-only — removed the old multi-product marketing copy.

## [5.4.1] - 2026-04-21

### Bug Fixes

- fix(ci): rebrand GHCR + GitHub URLs from ferrflow-org to ferrlabs (#376)

## [5.4.0] - 2026-04-20

### Features

- feat(site): FR mirrors for /cli, /secrets, /pricing (#373)

## [5.3.0] - 2026-04-20

### Features

- feat(site): FR mirror for /issues, remove landing intro, disabled FR option on untranslated pages (#372)

## [5.2.0] - 2026-04-19

### Features

- feat(site): unify language selector with Starlight style + SEO content pass (#371)
- feat(api): OIDC workload-identity exchange for cluster tokens (#366) (#367)

## [5.1.0] - 2026-04-19

### Features

- feat(site): snapshot v3 docs and promote current to v4 (#361)

## [5.0.0] - 2026-04-19

### Breaking Changes

- feat(api)!: standardise pagination envelope to { items, total, page, per_page, has_more } (#336) (#352)

### Features

- feat(site): document PubspecYaml / MixExs / ChartYaml / Gemspec / PackageSwift tabs (#359)
- feat(app): wire New project / New issue / New vault buttons (#323) (#355)
- feat(api,admin): add /admin/changelog feed + page (#336) (#351)
- feat(api): self-serve account deletion with grace window + purge worker (#336) (#350)
- feat(api): rotate sessions on org role change or member removal (#336) (#349)
- feat(api): add GET /auth/me/export for GDPR data export (#336) (#348)
- feat(api): async email outbox with retry/backoff (#347)
- feat(api): structured logs, Prometheus metrics, readyz/livez split (#346)
- feat(meta): first batch of grab-bag improvements (#336) (#345)

## [4.3.0] - 2026-04-19

### Features

- feat(api): env-driven bootstrap admin for self-host (#344)
- feat(api): per-IP and per-email rate limits on /auth/login (#343)

## [4.2.0] - 2026-04-19

### Features

- feat(auth): password strength meter + HIBP breach check + Argon2id tuned params (#341)
- feat(auth): forgot-password reset flow with email token (#340)
- feat(auth): user-facing active sessions list with per-device revoke (#339)
- feat(api): user profile fields (display_name, avatar, timezone, locale, preferences) (#338)
- feat(api): stable machine-readable error codes on all ApiError responses (#337)
- feat(auth): gate registration with SMTP email verification code (#324)
- feat(app): cluster + install mutations, and no-org onboarding (#321)
- feat(app): remove all mock data, wire real API everywhere (#320)
- feat(admin): Phase 2 admin panel UI + missing backend endpoints (#315)
- feat(api): GCP Cloud KMS backend for KEK and license signing (#314)
- feat(api): admin backend MVP (Phase 1 of admin panel) (#312)
- feat(api): Stripe billing integration (hosted mode) (#310)
- feat(app): self-host installs management UI (#309)
- feat(api): install license emitter (hosted mode) (#306)
- feat(api): install license token + deployment modes (#305)
- feat(app): clusters management UI (#299)
- feat(api): accept ffclust_ identities and enforce per-namespace authorization on reveal (#285)
- feat(api): cluster identities with server-side authorization (admin endpoints) (#284)

### Bug Fixes

- fix(api): copy keys directory into builder for license-pubkey include_str (#319)
- fix(app,admin): proxy /api via nginx instead of baked VITE_API_URL (#317)
- fix(api): redact secret plaintext from Debug and Display impls (#300)

## [4.1.1] - 2026-04-17

### Bug Fixes

- fix(site): restore slot in Navbar so version selector and search render (#281)

## [4.1.0] - 2026-04-17

### Features

- feat(site): document FerrFlow v3 release flow and snapshot v2 docs (#279)
- feat(api): add bulk reveal endpoint for operator integration (#277)

## [4.0.0] - 2026-04-17

### Breaking Changes

- feat(api)!: add API tokens with granular scopes (#268)

### Features

- feat(site): split nav CTA into Sign in and Sign up (#273)
- feat(api): add transactional email service skeleton (#269)

## [3.1.0] - 2026-04-16

### Features

- feat(selfhost): add bundled single-image deployment (#267)

## [3.0.0] - 2026-04-16

### Breaking Changes

- feat(api)!: add email/password auth with session cookies + frontend login/register (#265)

### Features

- feat(site): add login button in navbar linking to app.ferrflow.com (#266)
- feat(api): add issues, secrets, orgs, projects and JWT auth endpoints (#264)
- feat(app): sidebar navigation, themed pages, and new screens (#261)

### Bug Fixes

- fix(app): replace missing textLink property with text in useSection theme (#262)

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
