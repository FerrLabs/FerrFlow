# Changelog

## [6.17.2] - 2026-06-26

### Bug Fixes

- fix(site): refresh FerrFlow v5.20.0 release data

## [6.17.1] - 2026-06-26

### Bug Fixes

- fix(site): point sync-ferrflow workflow at Angular benchmarks.ts (#600)

## [6.17.0] - 2026-06-26

### Features

- feat(site): document updateLockfiles + sync JSON schema (#598)

## [6.16.0] - 2026-06-26

### Features

- feat(site): list Windows arm64 and Linux armv7 release artifacts (#596)

## [6.15.1] - 2026-06-26

### Bug Fixes

- fix(site): switch docs version selector to a dropdown via ui-ng 1.1.0 (#585)

## [6.15.0] - 2026-06-26

### Features

- feat(site): document publishing one package to multiple registries (#583)

## [6.14.0] - 2026-06-25

### Features

- feat(site): document ferrflow publish tag auto-scope + multiple packages + --all (#582)

## [6.13.0] - 2026-06-25

### Features

- feat(site): document --jobs / FERRFLOW_JOBS global flag in CLI reference (#581)

## [6.12.3] - 2026-06-21

### Bug Fixes

- fix(site): resolve docs catch-all routes to populate slug param (#578)

## [6.12.2] - 2026-06-20

### Bug Fixes

- fix(site): render English docs with the layout grid (#576)

## [6.12.1] - 2026-06-18

### Refactoring

- refactor(site): consume @ferrlabs/ui-ng/docs shared docs layout (#574)

## [6.12.0] - 2026-06-18

### Features

- feat(site): migrate ferrflow.com (marketing + versioned docs) to Angular (#570)

## [6.11.1] - 2026-06-17

### Bug Fixes

- fix(site): refresh FerrFlow v5.15.2 release data

## [6.11.0] - 2026-06-17

### Features

- feat(site): redesign performance page as interactive grouped bar chart (#568)

## [6.10.0] - 2026-06-17

### Features

- feat(site): show parallel-mode perf stats with runner core count (#566)

## [6.9.3] - 2026-06-17

### Bug Fixes

- fix(site): refresh FerrFlow v5.15.1 release data

## [6.9.2] - 2026-06-17

### Bug Fixes

- fix(site): refresh FerrFlow v5.12.0 release data

## [6.9.1] - 2026-06-16

### Bug Fixes

- fix(site): refresh FerrFlow v5.11.0 release data

## [6.9.0] - 2026-06-16

### Features

- feat(docs): document workspace.deferPublish + schema (#562)

## [6.8.1] - 2026-06-16

### Bug Fixes

- fix(site): refresh FerrFlow v5.8.0 release data

## [6.8.0] - 2026-06-16

### Features

- feat(docs): document ferrflow publish command (#560)

## [6.7.0] - 2026-06-16

### Features

- feat(config): document publisher noVerify + args fields (#558)

## [6.6.6] - 2026-06-16

### Bug Fixes

- fix(ci): use FERRLABS_DISPATCH_TOKEN for sync checkout+push (real repo access) (#556)

## [6.6.5] - 2026-06-15

### Bug Fixes

- fix(ci+site): unblock benchmark sync + show competitor tools on perf page (#553)

## [6.6.4] - 2026-06-15

### Bug Fixes

- fix(deps): update dependency @astrojs/starlight to ^0.40.0 (#552)

## [6.6.3] - 2026-06-13

### Bug Fixes

- fix(docs): remove fictional self-hosting API/dashboard page (#550)

## [6.6.2] - 2026-06-12

### Bug Fixes

- fix(docs): correct config reference — drop unsupported YAML, fix package array, complete workspace table (#548)

## [6.6.1] - 2026-06-12

### Bug Fixes

- fix(schema): sync served schema with CLI source of truth (#545)

## [6.6.0] - 2026-06-09

### Features

- feat(docs): cut v4 snapshot, surface v5 features, scrub forward references (#537)

## [6.5.12] - 2026-06-08

### Bug Fixes

- fix(site): stop leaking internal :8080 port in trailing-slash redirects (#531)

## [6.5.11] - 2026-06-07

### Bug Fixes

- fix(site): refresh FerrFlow v5.2.0 release data

## [6.5.10] - 2026-06-06

### Bug Fixes

- fix(site): refresh FerrFlow v5.1.0 release data
- fix(site): refresh FerrFlow v5.0.2 release data
- fix(deps): update dependency starlight-versions to ^0.9.0 (#515)

## [6.5.9] - 2026-05-23

### Bug Fixes

- fix(ci): run Security scan on the in-cluster runner so private-repo scans no longer skip (#519)

## [6.5.8] - 2026-05-22

### Bug Fixes

- fix(site): refresh FerrFlow v5.0.1 release data

## [6.5.7] - 2026-05-22

### Bug Fixes

- fix(ci): listen for ferrflow-released dispatch from FerrFlow CI (#517)

## [6.5.6] - 2026-05-21

### Bug Fixes

- fix(site): refresh FerrFlow v5.0.0 release data

## [6.5.5] - 2026-05-21

### Bug Fixes

- fix(site): refresh FerrFlow v4.10.8 release data

## [6.5.4] - 2026-05-21

### Bug Fixes

- fix(site): refresh FerrFlow v4.10.6 release data

## [6.5.3] - 2026-05-19

### Bug Fixes

- fix(site): refresh FerrFlow v4.10.4 release data

## [6.5.2] - 2026-05-19

### Bug Fixes

- fix(docs): drop Docs link and hide Products dropdown from navbar (#512)

## [6.5.1] - 2026-05-19

### Bug Fixes

- fix(site): refresh FerrFlow v4.10.2 release data

## [6.5.0] - 2026-05-19

### Features

- feat(site): add Layout-level bold pre-footer CTA (#511)

## [6.4.4] - 2026-05-19

### Bug Fixes

- fix(docker): trigger on v* tags too (single-package repo emits v6.4.x not site@v6.4.x) (#509)

## [6.4.3] - 2026-05-19

### Bug Fixes

- fix(docs): align Starlight with landing design (light-only, brand mark) (#507)

## [6.4.2] - 2026-05-18

### Bug Fixes

- fix(ci): commit sync changes directly to main instead of opening a PR (#501)

## [6.4.1] - 2026-05-18

### Bug Fixes

- fix(ci): use sed for hero version bump (python not on Ubuntu runners) (#500)

## [6.4.0] - 2026-05-18

### Features

- feat(ci): scheduled workflow to sync FerrFlow version + benchmarks every 6h (#499)
- feat(site): add /performance page comparing FerrFlow against JS-ecosystem release tools (#498)

## [6.3.0] - 2026-05-14

### Features

- feat(site): add security / vulnerability disclosure page (EN + FR) (#496)

## [6.2.1] - 2026-05-13

### Bug Fixes

- fix(schema): sync ferrflow.json from CLI source of truth (#433) (#494)

## [6.2.0] - 2026-05-07

### Features

- feat(site): add 404 pages (EN + FR) (#492)

## [6.1.0] - 2026-05-07

### Features

- feat(site): add mentions légales + privacy + CGU pages (#489)

## [6.0.1] - 2026-05-06

### Bug Fixes

- fix(nginx): per-location Cache-Control to bust stale HTML on deploy (#481)

## [6.0.0] - 2026-05-06

### Breaking Changes

- feat(api)!: standardise pagination envelope to { items, total, page, per_page, has_more } (#336) (#352)
- feat(api)!: add API tokens with granular scopes (#268)
- feat(api)!: add email/password auth with session cookies + frontend login/register (#265)
- chore!: switch license from MIT to MPL-2.0 (#226)
- chore!: switch license from MIT to MPL-2.0 (#69)

### Features

- feat(site): add FerrFleet to footer products grid (#438)
- feat(site): add legal pages — mentions, privacy, terms, cookies (EN+FR) (#435)
- feat(site): port editorial bundle design to marketing surface (Fraunces, paper, shared Navbar/Footer) (#432)
- feat(site): rebuild after design revert (force release) (#428)
- feat(site): adopt Claude/Anthropic design language on the marketing surface (#424)
- feat(ci): i18n parity + key resolution check (closes #420) (#421)
- feat(site): Claude-style docs chrome (#419)
- feat(site): editorial scrollbar + translate=no on code blocks (#417)
- feat(site): re-add lang pill + auto-redirect FR browsers on first visit (#416)
- feat(site): finish editorial refactor — stats strip, features grid, install grid, dark CTA (#415)
- feat(site): brand-glyph favicon matching the navbar logo (#412)
- feat(site): editorial Navbar + Footer matching the design bundle (#411)
- feat(site): editorial hero on the FerrFlow landing (#405)
- feat(site): docs Header uses shared @ferrlabs/ui-astro Navbar (delete 555-line local copy) (#403)
- feat: marketing landing uses unified @ferrlabs/ui-astro chrome (#402)
- feat(site): restore 2-line structured CTA with 'Ready to ship / without the ceremony' (#396)
- feat(site): revert hero copy to release/automatically, drop stale cli.* keys (#391)
- feat(site): home page becomes cli landing, drop multi-product marketing (#390)
- feat(site): FR mirrors for /cli, /secrets, /pricing (#373)
- feat(site): FR mirror for /issues, remove landing intro, disabled FR option on untranslated pages (#372)
- feat(site): unify language selector with Starlight style + SEO content pass (#371)
- feat(api): OIDC workload-identity exchange for cluster tokens (#366) (#367)
- feat(site): snapshot v3 docs and promote current to v4 (#361)
- feat(site): document PubspecYaml / MixExs / ChartYaml / Gemspec / PackageSwift tabs (#359)
- feat(app): wire New project / New issue / New vault buttons (#323) (#355)
- feat(api,admin): add /admin/changelog feed + page (#336) (#351)
- feat(api): self-serve account deletion with grace window + purge worker (#336) (#350)
- feat(api): rotate sessions on org role change or member removal (#336) (#349)
- feat(api): add GET /auth/me/export for GDPR data export (#336) (#348)
- feat(api): async email outbox with retry/backoff (#347)
- feat(api): structured logs, Prometheus metrics, readyz/livez split (#346)
- feat(meta): first batch of grab-bag improvements (#336) (#345)
- feat(api): env-driven bootstrap admin for self-host (#344)
- feat(api): per-IP and per-email rate limits on /auth/login (#343)
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
- feat(site): document FerrFlow v3 release flow and snapshot v2 docs (#279)
- feat(api): add bulk reveal endpoint for operator integration (#277)
- feat(site): split nav CTA into Sign in and Sign up (#273)
- feat(api): add transactional email service skeleton (#269)
- feat(selfhost): add bundled single-image deployment (#267)
- feat(site): add login button in navbar linking to app.ferrflow.com (#266)
- feat(api): add issues, secrets, orgs, projects and JWT auth endpoints (#264)
- feat(app): sidebar navigation, themed pages, and new screens (#261)
- feat(site): theme navbar colors, fix logo on Chrome, widen dropdown bridge (#260)
- feat(site): redesign products section with zigzag column layout (#259)
- feat(site): localStorage-based language preference with redirect (#258)
- feat: site redesign + dashboard neutral shell + product pages + pricing (#256)
- feat(site): add Secrets and Agent cards, update features for platform (#245)
- feat(app): add Dockerfile and Docker CI workflow (#242)
- feat(app): create dashboard package with auth and issues viewer (#233)
- feat(site): redesign landing page as multi-product portal (#232)
- feat(site): document PR/MR preview comments for GitHub and GitLab (#225)
- feat(site): document ferrflow.ts and ferrflow.js config support (#224)
- feat(site): add error codes reference page (#223)
- feat(site): persist user-selected format preference across pages (#221)
- feat(site): add French translation for telemetry page (#216)
- feat(site): add telemetry page to v0 versioned docs (#213)
- feat(site): document pipeline trigger types with examples (#210)
- feat(site): document --force-version CLI flag (#208)
- feat(site): show rolling 30-day metrics with animated counters (#207)
- feat(site): document dependsOn package config for auto-bump dependents (#189)
- feat(site): unify navbar across landing page and docs (#185)
- feat(site): show config examples in all supported formats (JSON, TOML, JSON5, YAML) (#174)
- feat(playground): interactive commit/tag simulator in the playground (#150)
- feat(api): real-time analytics stats via SSE (#143)
- feat(playground): add markdown rendering toggle in changelog preview (#135)
- feat(site): document orphanedTagStrategy config option (#134)
- feat(site): use tabs for supported formats page and nudge copy button (#130)
- feat(site): show file paths as titles on code blocks (#128)
- feat(playground): hide undefined config fields with Show all toggle (#127)
- feat(site): add floating tags documentation and sync schema (#122)
- feat(playground): add telemetry preview panel (#109)
- feat: rework landing page metrics (repos tracked, commits analyzed) (#103)
- feat(playground): run simulation live on input change (#95)
- feat(site): add i18n support with French translation (#85)
- feat(site): add playground button and GitHub icon to landing navbar (#67)
- feat(playground): interactive config builder and release simulator (#63)
- feat(site): add v1 docs versioning and update documentation (#54)
- feat(api): add uptime to health endpoint (#49)
- feat: sync schema tag prefix (#36)
- feat(site): publish JSON Schema at /schema/ferrflow.json (#27)
- feat: add live metrics section on landing page
- feat: add custom code block theme matching landing page terminal
- feat: add POST /events endpoint and enhance public stats
- feat: add custom Header with logo and bicolor text in docs navbar
- feat: add SEO, sitemap, robots.txt and favicon
- feat: add versioned docs with starlight-versions, snapshot v0
- feat: merge docs into site, drop packages/docs
- feat(docs): add starlight docs site at /docs
- feat(site): redesign landing page with pastel orange theme
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

- fix(meta): correct titles, descriptions, favicons for site + playground (#470)
- fix(site): inline FR auto-redirect in <head> using shared lang-pref key (#459)
- fix(deps): bump @ferrlabs/* (triggered by astro@v0.12.0) (#447)
- fix(deps): bump @ferrlabs/* (triggered by react@v0.8.0) (#448)
- fix(ci): rename dispatch secret FERRFLOW_DISPATCH_TOKEN → FERRLABS_DISPATCH_TOKEN (#443)
- fix(site): add og:image:type meta for richer link previews (#440)
- fix(site): align docs with editorial brand + fix navbar collapse (#436)
- fix(site): hero italic 'Automatically.' was bleeding into terminal column at wide viewports (#433)
- fix(ci): pnpm/action-setup@v6 needs standalone:true on self-hosted runner (#425)
- fix(site): disable language toggle on pages without a mirror (#422)
- fix(docker): refresh lockfile + restore --frozen-lockfile (#414)
- fix(docker): switch site runtime to nginx-unprivileged (non-root + 8080) (#413)
- fix(docker): use --no-frozen-lockfile to match CI install behaviour (#410)
- fix(site): drop radial gradient from cta, keep solid primary-950 bg (#397)
- fix(ci): drop version: 10 from pnpm/action-setup, defer to package.json packageManager (#392)
- fix(ci): regenerate pnpm-lock.yaml after api/app/admin were moved out (#387)
- fix(ci): pin pnpm version for action-setup@v5 (#386)
- fix(api): mirror monorepo layout in Dockerfile for include_str CHANGELOG paths (#382)
- fix: add module-level doc headers to api and app entrypoints (#381)
- fix(ci): install build-essential on Lint API job (#380)
- fix: add per-package READMEs for api and app (#379)
- fix(ci): run buildah with --isolation=chroot (#378)
- fix(ci): rebrand GHCR + GitHub URLs from ferrflow-org to ferrlabs (#376)
- fix(api): copy keys directory into builder for license-pubkey include_str (#319)
- fix(app,admin): proxy /api via nginx instead of baked VITE_API_URL (#317)
- fix(api): redact secret plaintext from Debug and Display impls (#300)
- fix(site): restore slot in Navbar so version selector and search render (#281)
- fix(app): replace missing textLink property with text in useSection theme (#262)
- fix(config): add app package to ferrflow config (#241)
- fix(site): update docs to reference anonymous_telemetry instead of deprecated telemetry (#220)
- fix(api): add input validation for commits_count and repo_hash (#219)
- fix: remove dead package scripts and unused concurrently dependency (#218)
- fix(playground): rename telemetry to anonymous_telemetry in config types (#217)
- fix: migrate renovate config to matchCategories syntax (#212)
- fix(playground): always resolve {name} placeholder in tag template (#211)
- fix(site): disable Google auto-translate on localized pages (#205)
- fix(playground): collapse advanced workspace fields behind toggle (#204)
- fix(site): sync JSON schema with source of truth (#201)
- fix(site): add missing og.png, favicon.ico, and PNG icons (#188)
- fix: update renovate config to use recommended preset (#183)
- fix: remove references to non-existent packages from README (#182)
- fix(site): remove redundant Tools section from docs sidebar (#180)
- fix: disable skipCi in release commits (#179)
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
- fix(site): update MCP server docs for stdio-only transport (#147)
- fix(playground): Update @ferrflow/wasm to v2.9.2 (#145)
- fix(playground): Update @ferrflow/wasm to v2.9.1 (#144)
- fix(docker): patch Alpine zlib CVE and add Renovate Docker automerge (#142)
- fix(playground): Update @ferrflow/wasm to v2.8.5 (#140)
- fix(playground): Update @ferrflow/wasm to v2.8.4 (#139)
- fix(playground): Update @ferrflow/wasm to v2.8.1 (#138)
- fix(playground): Update @ferrflow/wasm to v2.7.1 (#137)
- fix: remove broken license badge on private repo (#136)
- fix(playground): Update @ferrflow/wasm to v2.7.0 (#132)
- fix(site): correct telemetry documentation inaccuracies (#131)
- fix(playground): Update @ferrflow/wasm to v2.6.1 (#126)
- fix(playground): Update @ferrflow/wasm to v2.6.0 (#125)
- fix(site): handle 404 for missing versioned pages (#124)
- fix(site): add telemetry page to v1 sidebar (#123)
- fix(playground): Update @ferrflow/wasm to v2.5.3 (#116)
- fix(playground): Update @ferrflow/wasm to v2.5.2 (#107)
- fix(deps): update rust crate sha2 to 0.11 (#104)
- fix(playground): Update @ferrflow/wasm to v2.3.0 (#105)
- fix(deps): update rust crate hmac to 0.13 (#102)
- fix(playground): Update @ferrflow/wasm to v2.2.2 (#98)
- fix(playground): add vite-plugin-wasm and renovate auto-merge rule (#91)
- fix(api): fix analytics events insert and remove PII columns (#90)
- fix(playground): add @ferrflow/wasm dependency (#89)
- fix(playground): refresh build for correct wasm
- fix(ci): remove build provenance attestation from Docker workflow (#82)
- fix(site): update documentation for v2 and correct github icon color (#81)
- fix(site): correct docs accuracy issues and add hooks documentation (#73)
- fix(deps): update dependency shiki to v4 (#65)
- fix(deps): update dependency @astrojs/react to v5 (#64)
- fix(site): add version selector to custom header (#62)
- fix(api): bind to 0.0.0.0 instead of 127.0.0.1 (#60)
- fix(ci): use release event to trigger Docker builds (#58)
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
- fix: remove shared package references from site Dockerfile
- fix: update docs links to /docs/introduction
- fix: remove eyebrow badge from hero
- fix: remove app.ferrflow.com links, replace with github ctас
- fix: add tailwind v4 css entry point and theme config
- fix: remove invalid manager keys from renovate config
- fix: update rust builder image to 1.85
- fix: use fully-qualified image names and fix API build context for buildah
- fix: resolve Docker build failures for api, site, and app
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

## [5.22.6] - 2026-05-06

### Bug Fixes

- fix(meta): correct titles, descriptions, favicons for site + playground (#470)

## [5.22.5] - 2026-05-06

### Bug Fixes

- fix(site): inline FR auto-redirect in <head> using shared lang-pref key (#459)

## [5.22.4] - 2026-05-02

### Bug Fixes

- fix(deps): bump @ferrlabs/* (triggered by astro@v0.12.0) (#447)
- fix(deps): bump @ferrlabs/* (triggered by react@v0.8.0) (#448)

## [5.22.3] - 2026-05-02

### Bug Fixes

- fix(deps): bump @ferrlabs/* (triggered by astro@v0.12.0) (#447)
- fix(deps): bump @ferrlabs/* (triggered by react@v0.8.0) (#448)

## [5.22.2] - 2026-05-02

### Bug Fixes

- fix(ci): rename dispatch secret FERRFLOW_DISPATCH_TOKEN → FERRLABS_DISPATCH_TOKEN (#443)

## [5.22.1] - 2026-05-02

### Bug Fixes

- fix(site): add og:image:type meta for richer link previews (#440)

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
