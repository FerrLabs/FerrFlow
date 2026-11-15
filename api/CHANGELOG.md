# Changelog

## [2026.11.7] - 2026-11-15

### Features

- feat: benchmark tool_configs, reset version, disable skipCi (#279)
- Feat/status command (#41)

## [2026.11.6] - 2026-11-15

### Breaking Changes

- feat(api)!: retire les montages /v1 et /api/v1 (#788)

## [2026.11.5] - 2026-11-14

### Bug Fixes

- fix(deps): update rust crate jsonwebtoken to v12 (#1163)

## [2026.10.15] - 2026-10-31

### Bug Fixes

- fix(api): link the published ferrflow 7 crate instead of a v5 git rev (#1123)

## [2026.10.13] - 2026-10-25

### Bug Fixes

- fix(api): restrict oidc token exchange to default branch, tags and non-PR events (#1106)

## [2026.10.12] - 2026-10-24

### Bug Fixes

- fix(deps): update crates for RUSTSEC-2026-0381..0385 and ignore 0387 (#1105)

## [2026.10.10] - 2026-10-21

### Bug Fixes

- fix(deps): update dependency @ferrflow/doc to v7.33.0 (#1088)

## [2026.10.8] - 2026-10-16

### Bug Fixes

- fix(deps): update rustls to 0.23.46 for RUSTSEC-2026-0346 to 0349 (#1076)

## [2026.9.4] - 2026-09-17

### Bug Fixes

- fix(deps): update rustls to 0.23.45 (#965)

## [2026.9.1] - 2026-09-02

### Features

- feat(api): add the marketplace webhook route (#893)

## [2026.8.2] - 2026-08-31

### Bug Fixes

- fix(release): sync api/Cargo.lock and enable updateLockfiles (#875)
- fix(site): sync the served schema with the CLI source (#873)
- fix(ci): repin the .github reusables to current main and resync api/Cargo.lock (#880)
- fix(site): refresh FerrFlow v7.11.1 release data
- fix(site): refresh FerrFlow v7.11.0 release data and JSON schema
- fix(site): drop the telemetry page from v6 and v7 and repoint its links (#866)
- fix(site): refresh FerrFlow v7.10.5 release data
- fix(deps): pin esbuild to the version that verifies its binary (#861)
- fix(site): refresh FerrFlow v7.10.2 release data
- fix(site): drop the numbered section chapôs (#857)
- fix(site): refresh FerrFlow v7.9.1 release data
- fix(site): refresh FerrFlow v7.9.0 release data
- fix(site): refresh FerrFlow v7.8.1 release data
- fix(site): refresh FerrFlow v7.7.2 release data and JSON schema
- fix(site): refresh FerrFlow v7.7.1 release data
- fix(site): refresh FerrFlow v7.6.0 release data and JSON schema

## [2026.8.1] - 2026-08-24

### Features

- feat(docs): document versionSource and sync the schema (#833)
- feat(site): add the changelog to ferrflow.com (#835)
- feat(docs): document the permissive commit-format defaults (#832)
- feat(docs): why names the source of the current version (#831)
- feat(docs): document workspace.latestTag (#830)
- feat(docs): document workspace.commitFormats (#798)

### Bug Fixes

- fix(site): refresh FerrFlow v7.4.3 release data
- fix(site): refresh FerrFlow v7.4.2 release data
- fix(site): refresh FerrFlow v7.4.0 release data and JSON schema
- fix(site): refresh FerrFlow v7.3.1 release data and JSON schema
- fix(site): refresh FerrFlow v7.1.0 release data
- fix(schema): sync served schema with the FerrFlow CLI source
- fix(site): refresh FerrFlow v7.0.5 release data
- fix(site): refresh FerrFlow v7.0.3 release data
- fix(site): refresh FerrFlow v7.0.2 release data

## [6.1.0] - 2026-08-12

### Features

- feat(docs): document workspace.releaseCommitBody (#795)

### Bug Fixes

- fix(site): refresh FerrFlow v7.0.0 release data
- fix(ci): accorde pull-requests write au job appelant les reusables (#799)
- fix(schema): sync served schema with the FerrFlow CLI source
- fix(site): refresh FerrFlow v6.2.0 release data
- fix(site): refresh FerrFlow v6.1.1 release data

## [6.0.2] - 2026-08-09

### Bug Fixes

- fix(deps): update rust crate jsonwebtoken to v11 (#783)
- fix(ci): retire le déclencheur cross-repo, garde le cycle de 6 h et le manuel (#789)
- fix(site): refresh FerrFlow v6.0.0 release data

## [6.0.1] - 2026-08-08

### Bug Fixes

- fix(docker): câble le registre Kellnr dans le build de l'API (#787)

## [6.0.0] - 2026-08-08

### Breaking Changes

- feat(api)!: négocie le contrat par en-tête, routes servies à la racine (#784)

### Bug Fixes

- fix(site): refresh FerrFlow v5.52.1 release data

## [5.3.1] - 2026-08-07

### Bug Fixes

- fix(site): refresh FerrFlow v5.52.0 release data
- fix(deps): update rust crate jsonwebtoken to v10 [security] (#775)

## [5.3.0] - 2026-08-05

### Features

- feat(api): add the standalone FerrFlow API (#774)
- feat(docs): point the FerrFlow API docs at api.ferrflow.com (#772)
- feat(docs): document the ferrflow why command (#771)
- feat(docs): document dependency propagation policies and updateDependents (#769)
- feat(docs): changesets migration now discovers workspace packages (#749)
- feat(docs): document cabal and cmake version file formats (#746)
- feat(docs): document all migrate sources in the CLI reference (#728)
- feat(docs): add FerrFlow API reference page (EN + FR) (#731)
- feat(docs): document the ferrflow diff command (#730)
- feat(docs): document the persistent release PR behaviour (releaseCommitMode pr) (#726)
- feat(docs): document allPackages and releaseUrl hook context fields (#724)
- feat(docs): document rich hook context fields (changelog, commits, bumpedFiles, monorepo) (#719)
- feat(site): note self-hosted forge auto-detection in the config reference (#711)
- feat(site): add hosted bot (ferrflow[bot]) setup guide (#706)
- feat(site): document forge = bitbucket in the config reference (#709)
- feat(site): document BREAKING CHANGE parsing rules and variants (#705)
- feat(site): document the ferrflow doctor subcommand (#699)
- feat(site): document the ferrflow schema subcommand (#701)
- feat(site): document linked/fixed package version groups (#697)
- feat(site): document the telemetry removal (#694)
- feat(site): document the migrate command (#690)
- feat(ci): dispatch Renovate when the rebase box is ticked (#683)
- feat(site): split each benchmark bar into startup and work (#682)
- feat(site): label the complex dependency-graph benchmark fixture (#671)
- feat(site): add /why comparison page vs semantic-release, release-please, Changesets (#667)
- feat(site): document expanded release hook points in config reference (#664)
- feat(site): document Gitea/Forgejo forge in config reference (#661)
- feat(site): document --log-format flag and logging behavior (#659)
- feat(i18n): translate the performance page to French (#646)
- feat(site): consume shared chrome + migrate to @angular/localize runtime i18n (#614)
- feat(site): document monorepo dependency cycle detection (#604)
- feat(site): document updateLockfiles + sync JSON schema (#598)
- feat(site): list Windows arm64 and Linux armv7 release artifacts (#596)
- feat(site): document publishing one package to multiple registries (#583)
- feat(site): document ferrflow publish tag auto-scope + multiple packages + --all (#582)
- feat(site): document --jobs / FERRFLOW_JOBS global flag in CLI reference (#581)
- feat(site): migrate ferrflow.com (marketing + versioned docs) to Angular (#570)
- feat(site): redesign performance page as interactive grouped bar chart (#568)
- feat(site): show parallel-mode perf stats with runner core count (#566)
- feat(docs): document workspace.deferPublish + schema (#562)
- feat(docs): document ferrflow publish command (#560)
- feat(config): document publisher noVerify + args fields (#558)
- feat(docs): cut v4 snapshot, surface v5 features, scrub forward references (#537)
- feat(site): add Layout-level bold pre-footer CTA (#511)
- feat(perf): add install footprint section to compare release-please (#503)
- feat(ci): scheduled workflow to sync FerrFlow version + benchmarks every 6h (#499)
- feat(site): add /performance page comparing FerrFlow against JS-ecosystem release tools (#498)
- feat(site): add security / vulnerability disclosure page (EN + FR) (#496)
- feat(site): add 404 pages (EN + FR) (#492)
- feat(site): add mentions légales + privacy + CGU pages (#489)
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

### Bug Fixes

- fix(site): refresh FerrFlow v5.51.0 release data
- fix(site): refresh FerrFlow v5.50.0 release data and JSON schema
- fix(site): serve the styled 404 for docs directories instead of a bare 403 (#766)
- fix(site): hide docs nav entries that a version never shipped (#761)
- fix(deps): declare tinyglobby for @analogjs/vite-plugin-nitro (#760)
- fix(site): refresh FerrFlow v5.49.0 release data
- fix(site): refresh FerrFlow v5.48.1 release data
- fix(site): refresh FerrFlow v5.48.0 release data
- fix(docs): document cosign bundle verification for releases (#748)
- fix(site): refresh FerrFlow v5.47.4 release data
- fix(site): refresh FerrFlow v5.47.3 release data
- fix(site): refresh FerrFlow v5.47.2 release data
- fix(site): refresh FerrFlow v5.47.1 release data
- fix(ci): repair renovate-rebase.yml truncated by the pin sweep (#736)
- fix(site): refresh FerrFlow v5.47.0 release data
- fix(site): refresh FerrFlow v5.46.0 release data
- fix(site): refresh FerrFlow v5.45.0 release data
- fix(site): refresh FerrFlow v5.44.0 release data
- fix(site): refresh FerrFlow v5.42.0 release data
- fix(site): refresh FerrFlow v5.40.0 release data
- fix(site): describe the benchmark as like-for-like release --dry-run for every tool (#721)
- fix(site): refresh FerrFlow v5.39.0 release data
- fix(site): refresh FerrFlow v5.37.0 release data
- fix(site): refresh FerrFlow v5.36.1 release data and JSON schema
- fix(site): refresh FerrFlow v5.36.0 release data
- fix(site): refresh FerrFlow v5.35.0 release data
- fix(site): refresh FerrFlow v5.34.0 release data
- fix(site): refresh FerrFlow v5.33.3 release data
- fix(ci): cancel superseded benchmark syncs, keep only the latest (#692)
- fix(site): refresh FerrFlow v5.32.2 release data
- fix(site): refresh FerrFlow v5.32.0 release data
- fix(site): refresh FerrFlow v5.31.4 release data
- fix(site): refresh FerrFlow v5.31.2 release data
- fix(site): refresh FerrFlow v5.31.1 release data
- fix(site): drop startup from the chart legend (#686)
- fix(site): refresh FerrFlow v5.31.0 release data
- fix(site): match the /performance container on /why (#685)
- fix(site): refresh FerrFlow v5.30.0 release data
- fix(site): refresh FerrFlow v5.29.5 release data
- fix(site): correct the changesets claim in the benchmark methodology (#681)
- fix(site): plot cold benchmark data on a linear scale (#678)
- fix(site): refresh FerrFlow v5.29.4 release data
- fix(site): refresh FerrFlow v5.29.3 release data
- fix(ci): rebase and retry when the benchmark sync races a merge to main (#680)
- fix(site): refresh FerrFlow v5.29.2 release data
- fix(site): refresh FerrFlow v5.29.1 release data
- fix(site): refresh FerrFlow v5.29.0 release data
- fix(site): refresh FerrFlow v5.28.1 release data
- fix(site): align favicon and app icons with navbar commit-graph logo (#669)
- fix(site): refresh FerrFlow v5.28.0 release data
- fix(site): refresh FerrFlow v5.27.1 release data
- fix(site): dedupe tailwindcss @import — foundation already imports it (#662)
- fix(site): refresh FerrFlow v5.27.0 release data
- fix(site): refresh FerrFlow v5.26.0 release data
- fix(site): refresh FerrFlow v5.25.1 release data
- fix(site): refresh FerrFlow v5.25.0 release data
- fix(site): refresh FerrFlow v5.24.0 release data
- fix(deps): update dependency marked to v18 (#572)
- fix(site): refresh FerrFlow v5.23.0 release data
- fix(site): refresh FerrFlow v5.22.2 release data
- fix(site): update WASM package name to @ferrflow/wasm in legacy docs (#634)
- fix(site): update WASM package name to @ferrflow/wasm (#633)
- fix(site): show accurate packages × commits in perf fixture labels (#629)
- fix(site): restrict perf comparison chart to the shared command (check) (#627)
- fix(site): emit chrome design tokens with @theme static so Tailwind keeps them (#625)
- fix(site): refresh FerrFlow v5.22.1 release data
- fix(site): label hero size stat as Download to disambiguate from on-disk size (#617)
- fix(site): keep the navbar sticky at the top on scroll (#611)
- fix(docker): read packages-read secret from file (buildah lacks env= mount support) (#609)
- fix(site): refresh FerrFlow v5.22.0 release data
- fix(site): refresh FerrFlow v5.21.0 release data
- fix(site): refresh FerrFlow v5.20.0 release data
- fix(site): point sync-ferrflow workflow at Angular benchmarks.ts (#600)
- fix(site): switch docs version selector to a dropdown via ui-ng 1.1.0 (#585)
- fix(site): resolve docs catch-all routes to populate slug param (#578)
- fix(site): render English docs with the layout grid (#576)
- fix(site): refresh FerrFlow v5.15.2 release data
- fix(site): refresh FerrFlow v5.15.1 release data
- fix(site): refresh FerrFlow v5.12.0 release data
- fix(site): refresh FerrFlow v5.11.0 release data
- fix(site): refresh FerrFlow v5.8.0 release data
- fix(site): refresh FerrFlow v5.5.0 release data
- fix(ci): use FERRLABS_DISPATCH_TOKEN for sync checkout+push (real repo access) (#556)
- fix(ci+site): unblock benchmark sync + show competitor tools on perf page (#553)
- fix(deps): update dependency @astrojs/starlight to ^0.40.0 (#552)
- fix(deps): update dependency @astrojs/starlight to ^0.39.0 (#514)
- fix(docs): remove fictional self-hosting API/dashboard page (#550)
- fix(site): drop nonexistent playground from security scope (#551)
- fix(docs): correct config reference — drop unsupported YAML, fix package array, complete workspace table (#548)
- fix(docs): reconcile CLI reference with FerrFlow v5 surface (#547)
- fix(docs): correct published npm package names in install docs (#546)
- fix(schema): sync served schema with CLI source of truth (#545)
- fix(site): stop leaking internal :8080 port in trailing-slash redirects (#531)
- fix(site): refresh FerrFlow v5.2.0 release data
- fix(site): refresh FerrFlow v5.1.0 release data
- fix(site): refresh FerrFlow v5.0.2 release data
- fix(deps): update dependency starlight-versions to ^0.9.0 (#515)
- fix(ci): run Security scan on the in-cluster runner so private-repo scans no longer skip (#519)
- fix(site): refresh FerrFlow v5.0.1 release data
- fix(ci): listen for ferrflow-released dispatch from FerrFlow CI (#517)
- fix(site): refresh FerrFlow v5.0.0 release data
- fix(site): refresh FerrFlow v4.10.8 release data
- fix(site): refresh FerrFlow v4.10.6 release data
- fix(site): refresh FerrFlow v4.10.4 release data
- fix(docs): drop Docs link and hide Products dropdown from navbar (#512)
- fix(site): refresh FerrFlow v4.10.2 release data
- fix(docker): trigger on v* tags too (single-package repo emits v6.4.x not site@v6.4.x) (#509)
- fix(site): auto-redeploy on new FerrFlow release with fresh data (#505)
- fix(docs): align Starlight with landing design (light-only, brand mark) (#507)
- fix(ci): commit sync changes directly to main instead of opening a PR (#501)
- fix(ci): use sed for hero version bump (python not on Ubuntu runners) (#500)
- fix(schema): sync ferrflow.json from CLI source of truth (#433) (#494)
- fix(nginx): per-location Cache-Control to bust stale HTML on deploy (#481)
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

### Refactoring

- refactor: move packages/site to the repo root (#773)
- refactor(site): use the shared comparison header on /why (#674)
- refactor(site): consume @ferrlabs/ui-foundation fonts instead of local @fontsource (#636)
- refactor(site): store benchmarks as JSON asset instead of regenerated TS module (#602)
- refactor(site): consume @ferrlabs/ui-ng/docs shared docs layout (#574)
- refactor: scope FerrFlow-Cloud to marketing site + CLI playground only (#384)
