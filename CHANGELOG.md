# Changelog

All notable changes to `ferrflow` will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [7.17.0] - 2026-09-03

### Features

- feat(docs): move the site documentation here and package it as @ferrflow/doc (#1013)

## [7.16.0] - 2026-09-03

### Features

- feat(changelog): allow any conventional commit type as a section (#1004)

## [7.15.1] - 2026-09-02

### Bug Fixes

- fix(schema): the snake_case format enum was missing cabal and cmake (#1002)

## [7.15.0] - 2026-09-02

### Features

- feat(formats): add galaxyyml for Ansible Galaxy collections (#999)

### Bug Fixes

- fix(config): accept the snake_case publisher fields the schema documents (#1000)

## [7.14.0] - 2026-09-02

### Features

- feat(publishers): support PyPI trusted publishing (#997)

## [7.13.6] - 2026-09-02

### Bug Fixes

- fix(config): drop the dead anonymousTelemetry field (#996)

## [7.13.5] - 2026-09-02

### Bug Fixes

- fix(release): do not propagate a raise that cannot be applied (#992)

## [7.13.4] - 2026-09-02

### Bug Fixes

- fix(release): raise a package's own bump when a dependency moves further (#991)

## [7.13.3] - 2026-09-02

### Bug Fixes

- fix(cascade): a dependent takes the strongest bump, not the first to arrive (#988)

## [7.13.2] - 2026-09-02

### Bug Fixes

- fix(schema): postBump runs after changelog generation, not before (#987)
- fix(validate): drop the redundant glob import from the tests (#986)

## [7.13.1] - 2026-09-01

### Bug Fixes

- fix(changelog): a postBump rewrite now reaches the tag and release body (#982)

## [7.13.0] - 2026-09-01

### Features

- feat(doctor): report lockfile drift and lockfiles left out of releases (#981)

## [7.12.0] - 2026-09-01

### Features

- feat(graph): show what releasing a package would drag along (#979)

## [7.11.2] - 2026-08-31

### Bug Fixes

- fix(monorepo): scope a package's commits to its own paths (#974)

## [7.11.1] - 2026-08-29

### Bug Fixes

- fix(git): retry an authored commit when the branch moved under it (#970)

## [7.11.0] - 2026-08-29

### Features

- feat(versioning): add calver-short-seq and surface a refused release (#967)

### Bug Fixes

- fix(release): key the commit path dedupe on the normalised path (#969)
- fix(cleanup): remove the lock and scoped npmrc on panic, which Drop cannot do under abort (#964)

## [7.10.5] - 2026-08-28

### Bug Fixes

- fix(cache): prune orphaned temp files and key on the versioned files (#963)

## [7.10.4] - 2026-08-28

### Bug Fixes

- fix(publishers): report the crate name from Cargo.toml, not the ferrflow name (#962)

## [7.10.3] - 2026-08-28

### Bug Fixes

- fix(publishers): report the npm package name from package.json, not the ferrflow name (#961)

## [7.10.2] - 2026-08-28

### Bug Fixes

- fix(git): ask ls-remote for the peeled ref so annotated tags resolve to their commit (#955)

## [7.10.1] - 2026-08-28

### Bug Fixes

- fix(release): send a path named by several versionedFiles entries once (#951)

## [7.10.0] - 2026-08-28

### Features

- feat(cli): add ferrflow rollback to undo a partially-failed release (#946)

## [7.9.1] - 2026-08-27

### Bug Fixes

- fix(config): keep workspace defaults when the workspace block is omitted (#945)

## [7.9.0] - 2026-08-27

### Features

- feat(bot): author release commits through createCommitOnBranch so they are verified (#941)

### Bug Fixes

- fix(deps): upgrade gix to 0.87 to drop the yanked bisync crate (#943)

## [7.8.1] - 2026-08-25

### Bug Fixes

- fix(git): classify push failures by cause instead of the generic push error code (#937)

## [7.8.0] - 2026-08-25

### Features

- feat(config): support per-package config files via an include key (#932)

### Bug Fixes

- fix(release): make pr mode tag on merge instead of on the pre-bump commit (#936)

## [7.7.2] - 2026-08-25

### Bug Fixes

- fix(publish): name cosign bundles .sigstore.json so they are recognised as signatures (#929)
- fix(ci): send the coverage report to SonarQube (#926)

## [7.7.1] - 2026-08-25

### Bug Fixes

- fix(docker): run as a non-root user in both images (#924)
- fix(git): honour commit.gpgsign on the commit-tree path (#922)
- fix(wasm): put api_check behind the cli feature and build the no-cli surface in CI (#919)

## [7.7.0] - 2026-08-24

### Features

- feat(cli): ferrflow plan --interactive (#917)
- feat(cli): repeatable --force-version and a new --exclude (#916)

## [7.6.0] - 2026-08-24

### Features

- feat(cli): ferrflow api-check compares the public API against the last tag (#915)
- feat(publishers): add a pypi publisher (#914)

## [7.5.0] - 2026-08-24

### Features

- feat(cli): ferrflow graph shows the dependency graph, release order and cycles (#913)

## [7.4.3] - 2026-08-22

### Bug Fixes

- fix(hooks): strip every forge token from the hook environment (#900)

## [7.4.2] - 2026-08-21

### Bug Fixes

- perf(bench): make the validate benchmark measure validation (#895)

## [7.4.1] - 2026-08-21

### Bug Fixes

- perf(bench): add an end-to-end monorepo flow benchmark (#893)

## [7.4.0] - 2026-08-20

### Features

- feat(versioning): versionTemplate, a version format built from named variables (#886)

## [7.3.2] - 2026-08-20

### Bug Fixes

- fix(ci): drop socket.yml triggerPaths (#879)

## [7.3.1] - 2026-08-20

### Bug Fixes

- fix(publish): publish to npm with trusted publishing over OIDC (#875)

## [7.3.0] - 2026-08-20

### Features

- feat(config): let a repo choose which version source wins (#873)

## [7.2.0] - 2026-08-20

### Features

- feat(release): say which source the current version came from (#870)

## [7.1.0] - 2026-08-19

### Features

- feat(tags): add workspace.latestTag for a floating alias tag (#866)

### Bug Fixes

- fix(schema): declare latestTag on packages and validate the template (#868)

## [7.0.5] - 2026-08-16

### Bug Fixes

- fix(bot): stop retrying token-exchange errors a retry cannot fix (#854)

## [7.0.4] - 2026-08-16

### Refactoring

- refactor(release): replace the seven-field tag tuple with a named struct (#850)

## [7.0.3] - 2026-08-15

### Bug Fixes

- fix(bot): retry the token exchange on transient failures (#848)

## [7.0.2] - 2026-08-15

### Bug Fixes

- fix(ci): grant the sonarqube scan job pull-requests write (#847)

## [7.0.1] - 2026-08-15

### Bug Fixes

- fix(formats): resolve the cargo crate name and retry lockfile updates online (#844)

## [7.0.0] - 2026-08-10

### Breaking Changes

- feat(commits)!: configurable commit formats via workspace.commitFormats (#824)

## [6.2.0] - 2026-08-09

### Features

- feat(release): add workspace.releaseCommitBody to embed the changelog in release commits (#822)

## [6.1.1] - 2026-08-09

### Bug Fixes

- fix(ci): retire la notification cross-repo apres release (#819)

## [6.1.0] - 2026-08-08

### Features

- feat(bot): vise l'endpoint de jeton à la racine (#818)

## [6.0.0] - 2026-08-07

### Breaking Changes

- chore!: relicense from MPL-2.0 to MIT (#813)

### Bug Fixes

- fix(action): verify the downloaded release archive against SHA256SUMS before extracting (#811)

## [5.52.1] - 2026-08-07

### Bug Fixes

- fix(action): pass caller inputs through env instead of interpolating them into run scripts (#810)
- fix(publish): point npm at the scoped .npmrc so private-registry publishes authenticate (#808)
- fix(forge): set connect, response and global HTTP timeouts on release-path agents (#807)

### Refactoring

- refactor(http): drop explanatory comments per project style (#809)

## [5.52.0] - 2026-08-05

### Features

- feat(bot): exchange tokens on api.ferrflow.com instead of api.ferrlabs.com (#785)
- feat(cli): add ferrflow why to explain a package's release decision (#784)

## [5.51.0] - 2026-08-04

### Features

- feat(monorepo): rewrite dependent version constraints on release (#783)
- feat(monorepo): propagate the upstream bump type through the dependency cascade (#782)

## [5.50.0] - 2026-08-04

### Features

- feat(diff): scope the range to the named package in a monorepo (#781)

## [5.49.0] - 2026-07-28

### Features

- feat(migrate): auto-discover workspace packages for changesets (#775)

## [5.48.1] - 2026-07-28

### Bug Fixes

- fix(ci): sign release artifacts with cosign bundles (#774)

## [5.48.0] - 2026-07-27

### Features

- feat(formats): add Cabal (.cabal) and CMake project version support (#773)

## [5.47.4] - 2026-07-27

### Bug Fixes

- fix(release): push tags before creating forge releases (#771)

## [5.47.3] - 2026-07-25

### Bug Fixes

- fix(deps): update rust crate gix to 0.86 (#762)

## [5.47.2] - 2026-07-25

### Bug Fixes

- fix(release): replan against the winning run instead of failing with E2006 on concurrent releases (#766)

## [5.47.1] - 2026-07-23

### Bug Fixes

- fix(ci): repair renovate-rebase.yml truncated by the pin sweep (#761)

## [5.47.0] - 2026-07-21

### Features

- feat: expose the crate version as ferrflow::VERSION (#758)
- feat(schema): expose the bundled JSON schema from the library (#756)
- feat(validate): expose the validation core without the cli feature (#754)

## [5.46.0] - 2026-07-21

### Features

- feat(validate): expose the validation core without the cli feature (#754)

## [5.45.0] - 2026-07-21

### Features

- feat(cli): add diff command to compare two versions (#751)
- feat(migrate): broaden release-please release-type and plugin coverage (#749)
- feat(migrate): read JS and YAML source configs (#750)

## [5.44.0] - 2026-07-21

### Features

- feat(migrate): read JS and YAML source configs (#750)

## [5.43.0] - 2026-07-21

### Features

- feat(cli): migrate from changesets, release-please, and standard-version (#745)

## [5.42.0] - 2026-07-21

### Features

- feat(release): persistent release PR — update the open PR instead of opening a new one (#744)

## [5.41.0] - 2026-07-21

### Features

- feat(hooks): expose releaseUrl to post-publish hooks (#743)
- feat(hooks): expose allPackages batch snapshot to hooks (#741)

## [5.40.0] - 2026-07-20

### Features

- feat(hooks): expose allPackages batch snapshot to hooks (#741)

## [5.39.0] - 2026-07-20

### Features

- feat(hooks): pass rich release context (changelog, commits, bumped files, monorepo) to hooks (#736)

## [5.38.0] - 2026-07-19

### Features

- feat(forge): auto-detect self-hosted GitLab/GitHub/Gitea instances (#730)
- feat(forge): add Bitbucket Cloud support (#729)

## [5.37.0] - 2026-07-18

### Features

- feat(forge): add Bitbucket Cloud support (#729)

## [5.36.1] - 2026-07-18

### Bug Fixes

- perf(git): write a commit-graph on cold runs of large repos (#728)

## [5.36.0] - 2026-07-18

### Features

- feat(commit): detect mixed-case, hyphen, and scope-internal BREAKING CHANGE variants (#727)
- feat(cli): add schema subcommand to print the bundled JSON schema (#726)
- feat(cli): add doctor diagnostic subcommand (#725)

## [5.35.0] - 2026-07-18

### Features

- feat(cli): add doctor diagnostic subcommand (#725)

## [5.34.0] - 2026-07-17

### Features

- feat(monorepo): linked and fixed package version groups (#724)

### Bug Fixes

- perf(monorepo): skip per-package tag scan when versioning strategy is set (#723)
- fix(cli): exchange the bot token before spawning threads (#722)

## [5.33.3] - 2026-07-17

### Bug Fixes

- fix(cli): exchange the bot token before spawning threads (#722)

## [5.33.2] - 2026-07-17

### Bug Fixes

- fix(schema): accept the snake_case spellings serde already accepts (#721)
- perf(build): fat LTO and panic=abort for the release profile (#720)
- fix(ci): retry cosign signing and sweep orphaned draft releases (#719)

## [5.33.1] - 2026-07-17

### Bug Fixes

- fix(ci): retry cosign signing and sweep orphaned draft releases (#719)

## [5.33.0] - 2026-07-17

### Features

- feat: remove telemetry (#716)

### Bug Fixes

- fix(ci): drop the empty env block left by the telemetry removal (#717)

## [5.32.3] - 2026-07-17

### Bug Fixes

- fix(ci): trust prolific first-party-adjacent publishers in cargo vet (#708)

## [5.32.2] - 2026-07-17

### Bug Fixes

- fix(ci): trust all BurntSushi crates in cargo vet (#705)
- fix(wasm): gate the migrate module behind the cli feature (#702)
- fix(cli): stop the error printer from burying the message under its code (#700)

### Refactoring

- refactor(monorepo): compute the per-package versioning strategy once (#707)

## [5.32.1] - 2026-07-17

### Bug Fixes

- fix(cli): stop the error printer from burying the message under its code (#700)

## [5.32.0] - 2026-07-17

### Features

- feat(cli): add migrate command to import semantic-release config (#699)

## [5.31.4] - 2026-07-16

### Bug Fixes

- fix(ci): trust all epage crates in cargo vet (#697)

## [5.31.3] - 2026-07-16

### Bug Fixes

- perf(git): resolve local tag targets in-process instead of spawning rev-list (#693)
- perf(monorepo): share one decoded commit walk across touched packages (#692)

### Refactoring

- refactor(git): compute changed files with an in-process gix tree diff (#689)

## [5.31.2] - 2026-07-16

### Bug Fixes

- perf(monorepo): memoize the recover-missed-releases diff per tag commit (#685)

## [5.31.1] - 2026-07-15

### Bug Fixes

- perf(monorepo): reuse the tag index ancestor set instead of walking twice (#683)

## [5.31.0] - 2026-07-15

### Features

- feat(ci): dispatch Renovate when the rebase box is ticked (#681)
- feat(ci): benchmark a startup floor alongside every shard (#678)

### Bug Fixes

- fix(ci): trust epage for toml_edit and toml_writer in cargo vet (#680)

## [5.30.0] - 2026-07-15

### Features

- feat(ci): benchmark a startup floor alongside every shard (#678)

## [5.29.5] - 2026-07-15

### Bug Fixes

- fix(release): drop the checkpoint when a failed attempt is cleaned up (#676)

## [5.29.4] - 2026-07-15

### Bug Fixes

- fix(ci): benchmark ferrflow cold and warm up three times (#674)

## [5.29.3] - 2026-07-15

### Bug Fixes

- perf(ci): raise benchmark runs to 30 (#672)

## [5.29.2] - 2026-07-15

### Bug Fixes

- perf(ci): shard the full benchmark per fixture (#670)

## [5.29.1] - 2026-07-15

### Bug Fixes

- fix(ci): treat ferrflow as first-party in cargo-vet (#666)

## [5.29.0] - 2026-07-14

### Features

- feat(bench): add complex dependency-graph fixture (cascade path) (#661)

## [5.28.1] - 2026-07-14

### Bug Fixes

- fix(ci): stabilize cargo-vet supply-chain job (#660)

## [5.28.0] - 2026-07-13

### Features

- feat(hooks): add pre-tag, post-tag, post-commit, pre-release, on-success, on-error hook points (#657)

## [5.27.1] - 2026-07-13

### Bug Fixes

- fix(wasm): make tracing facade non-optional so the wasm build compiles (#655)

## [5.27.0] - 2026-07-13

### Features

- feat(forge): add Gitea/Forgejo release support (#653)
- feat(obs): migrate status command output to tracing (#651)
- feat(obs): migrate misc root status output to tracing (#650)

## [5.26.0] - 2026-07-13

### Features

- feat(obs): migrate misc root status output to tracing (#650)

## [5.25.1] - 2026-07-12

### Bug Fixes

- fix(deps): bump gix to 0.85 and gix-traverse to 0.59 (#647)

## [5.25.0] - 2026-07-12

### Features

- feat(obs): migrate monorepo report output to tracing (#642)
- feat(obs): migrate monorepo run progress lines to tracing (#643)
- feat(obs): migrate validate output to tracing (#641)
- feat(obs): migrate main error printer and timing breakdown to tracing (#644)

## [5.24.0] - 2026-07-12

### Features

- feat(obs): migrate validate output to tracing (#641)
- feat(obs): migrate main error printer and timing breakdown to tracing (#644)

## [5.23.0] - 2026-07-03

### Features

- feat(obs): migrate publishers, hooks, and gitlab forge to tracing (#637)

### Bug Fixes

- perf(ci): PGO build for the x64-linux release binary (#634)

## [5.22.2] - 2026-07-03

### Refactoring

- refactor(formats): share a format-preserving splice helper across handlers (#631)

## [5.22.1] - 2026-06-28

### Bug Fixes

- fix(publish): retry cargo publish on transient registry index lag (#622)

## [5.22.0] - 2026-06-26

### Features

- feat(monorepo): detect dependency cycles and release in topological order (#618)
- feat(formats): auto-update lockfiles after version bump (#617)
- feat(obs): add tracing logging foundation (init, --log-format flag, JSON layer) (#609)

### Bug Fixes

- fix(build): sync Cargo.lock to the 5.20.0 manifest version (#616)

### Refactoring

- refactor(obs): migrate src/monorepo/ diagnostics to tracing macros (#615)

## [5.21.0] - 2026-06-26

### Features

- feat(obs): add tracing logging foundation (init, --log-format flag, JSON layer) (#609)

### Bug Fixes

- fix(build): sync Cargo.lock to the 5.20.0 manifest version (#616)

### Refactoring

- refactor(obs): migrate src/monorepo/ diagnostics to tracing macros (#615)

## [5.20.0] - 2026-06-26

### Features

- feat(obs): add tracing logging foundation (init, --log-format flag, JSON layer) (#609)

## [5.19.0] - 2026-06-26

### Features

- feat(release): add Windows arm64 and Linux armv7 build targets (#607)

### Bug Fixes

- fix(npm): add files allowlist, shim signal/error handling, node engines (#608)

## [5.18.0] - 2026-06-26

### Features

- feat(npm): publish CLI as unscoped ferrflow with @ferrflow/* platform packages (#606)
- feat(publish): auto-scope by triggering tag, accept multiple packages and --all (#602)

### Bug Fixes

- fix(deps): bump memmap2 to 0.9.11 (RUSTSEC-2026-0186) (#604)

## [5.17.0] - 2026-06-25

### Features

- feat(publish): auto-scope by triggering tag, accept multiple packages and --all (#602)

## [5.16.0] - 2026-06-17

### Features

- feat(cli): add --jobs / FERRFLOW_JOBS to control parallelism (#598)

### Bug Fixes

- perf(git): use commit-graph for revision walks (#596)

## [5.15.2] - 2026-06-17

### Bug Fixes

- perf(forge): parallelize per-tag release creation with a capped rayon pool (#595)

## [5.15.1] - 2026-06-17

### Refactoring

- refactor(monorepo): share HEAD-ancestor cache through orphan-strategy tag lookups (#594)

## [5.15.0] - 2026-06-17

### Features

- feat(release): --json output and --dry-run unified file diff (#588)

### Bug Fixes

- perf(monorepo): parallelize per-package planning with rayon (#592)
- fix(forge): validate derived API host and document GitLab draft no-op (#589)

## [5.14.0] - 2026-06-17

### Features

- feat(release): --json output and --dry-run unified file diff (#588)

## [5.13.0] - 2026-06-16

### Features

- feat(release): optional manifest mode (.ferrflow.manifest.json source of truth) (#587)

## [5.12.0] - 2026-06-16

### Features

- feat(changelog): configurable sections, scope grouping, commit + compare links (#586)

## [5.11.1] - 2026-06-16

### Bug Fixes

- perf(cache): cross-run cache for the per-package walk under .git/ferrflow-cache (#585)

## [5.11.0] - 2026-06-16

### Features

- feat(cli): add --timing flag for per-stage breakdown (#584)

## [5.10.0] - 2026-06-16

### Features

- feat(config): add workspace.deferPublish to skip publishers on release (#583)

## [5.9.0] - 2026-06-16

### Features

- feat(cli): add ferrflow publish to run publishers without releasing (#581)

## [5.8.0] - 2026-06-16

### Features

- feat(publishers): add args escape-hatch to command publishers (#579)

## [5.7.0] - 2026-06-16

### Features

- feat(publishers): add noVerify option to cargo publisher (#577)

## [5.6.0] - 2026-06-15

### Features

- feat(publishers): helm + github-release-asset + webhook executors (final 3) (#576)
- feat(publishers): docker buildx + multi-arch + optional sigstore signing (#575)

## [5.5.0] - 2026-06-15

### Features

- feat(publishers): npm executor with scoped .npmrc + idempotency (#574)
- feat(publishers): cargo executor with idempotency + token-env validation (#573)
- feat(config): declarative publishers + workspace registries (foundation) (#572)

## [5.4.0] - 2026-06-15

### Features

- feat(config): declarative publishers + workspace registries (foundation) (#572)

## [5.3.6] - 2026-06-15

### Bug Fixes

- perf(ci): cache npm globals for the bench job (#570)
- perf(ci): install cargo-tarpaulin prebuilt instead of compiling from source (#569)

## [5.3.5] - 2026-06-15

### Bug Fixes

- fix(commit): require colon delimiter in BREAKING CHANGE footer (#550) (#558)

## [5.3.4] - 2026-06-15

### Bug Fixes

- fix(deps): update rust crate gix to 0.84 (#520)
- fix(changelog): align categorization with bump parser, keep refactor section (#525) (#559)
- fix(docs): correct hook env vars, schema enum, GitLab draft warning, --recover refs (#557)
- fix(audit): zerover metadata clear, path containment, URL encoding (#553) (#562)

## [5.3.3] - 2026-06-13

### Bug Fixes

- fix(changelog): align categorization with bump parser, keep refactor section (#525) (#559)
- fix(docs): correct hook env vars, schema enum, GitLab draft warning, --recover refs (#557)
- fix(audit): zerover metadata clear, path containment, URL encoding (#553) (#562)

## [5.3.2] - 2026-06-13

### Bug Fixes

- fix(forge): paginate release & comment lookups on GitHub + GitLab (#524) (#561)
- fix(formats): read & write Cargo workspace-inherited versions (#523) (#560)

## [5.3.1] - 2026-06-12

### Bug Fixes

- fix: correct changelog classification, zerover metadata, and stale recover hint (#566)
- fix: add the 5 missing file formats to the JSON schema format enum (#565)
- fix: require colon delimiter in BREAKING CHANGE footer to stop spurious major bumps (#563)

## [5.3.0] - 2026-06-11

### Features

- feat(release): crash-resume checkpoint (#549) (#556)
- feat(commit): configurable commit-skip markers, subject-only matching (#527) (#554)

### Bug Fixes

- fix(formats): preserve formatting when bumping JSON files (#526) (#555)

## [5.2.4] - 2026-06-08

### Bug Fixes

- fix(release): wire --force-unlock CLI flag (#514) (#545)

## [5.2.3] - 2026-06-07

### Bug Fixes

- fix(ci): cargo-cyclonedx has no -p flag; select root SBOM via override-filename (#541)

## [5.2.2] - 2026-06-07

### Bug Fixes

- fix(git): reset checkout-persisted extraheader so bot token pushes the release (#539)

## [5.2.1] - 2026-06-06

### Bug Fixes

- fix(ci): discover cargo-cyclonedx output by glob, target ferrflow package (#537)

## [5.2.0] - 2026-06-05

### Features

- feat: release concurrency lock, mimalloc, cargo-deny bans, ref validation, markdown escape, ureq Agent reuse
- feat: support tag-only packages (versionedFiles really optional) (#533)

## [5.1.0] - 2026-06-05

### Features

- feat: support tag-only packages (versionedFiles really optional) (#533)

## [5.0.2] - 2026-06-03

## [5.0.1] - 2026-05-21

## [5.0.0] - 2026-05-21

### Breaking Changes

- refactor(git)!: replace URL token injection with credential helper protocol (#486)

### Bug Fixes

- perf(tags): show shared gix::Repository handle doesn't fix the TagIndex slowdown (#483)

## [4.10.8] - 2026-05-21

### Bug Fixes

- perf(tags): show shared gix::Repository handle doesn't fix the TagIndex slowdown (#483)

## [4.10.7] - 2026-05-20

### Bug Fixes

- perf(tags): document why TagIndex::build stays on libgit2 (gix is 2.4x slower) (#482)

## [4.10.6] - 2026-05-20

### Bug Fixes

- perf(tags): route collect_all_tags through gitoxide (2.7x faster) (#480)
- perf(tags): pre-collect tags into TagIndex to amortize tag_foreach across packages (#474) (#475)
- perf(tags): pre-collect tags into TagIndex to amortize tag_foreach across packages (#474)
- fix(bench): seed .changeset/initial.md so changesets/single benches like the others (#473)
- perf(tags): amortize HEAD reachability across multi-package callers (#466)
- perf(release): enable strip + thin LTO + single codegen-unit (#464)

## [4.10.5] - 2026-05-19

### Bug Fixes

- perf(tags): pre-collect tags into TagIndex to amortize tag_foreach across packages (#474)
- fix(bench): seed .changeset/initial.md so changesets/single benches like the others (#473)
- perf(tags): amortize HEAD reachability across multi-package callers (#466)
- perf(release): enable strip + thin LTO + single codegen-unit (#464)

## [4.10.4] - 2026-05-19

### Bug Fixes

- perf(tags): amortize HEAD reachability across multi-package callers (#466)
- perf(release): enable strip + thin LTO + single codegen-unit (#464)

## [4.10.3] - 2026-05-19

### Bug Fixes

- perf(release): enable strip + thin LTO + single codegen-unit (#464)

## [4.10.2] - 2026-05-19

### Bug Fixes

- fix(bench): let semantic-release fall back to origin remote (#463)
- fix(bench): set default_branch=main on all bench fixtures (#461)
- fix(release): make push_tags idempotent against pre-existing remote tags (#459)

## [4.10.1] - 2026-05-19

### Bug Fixes

- fix(release): make push_tags idempotent against pre-existing remote tags (#459)

## [4.10.0] - 2026-05-19

### Features

- feat(ci): dispatch ferrflow-released to FerrFlow-Cloud after release (#455)

## [4.9.0] - 2026-05-19

### Features

- feat(bench): add competitor tool configs to fixture definitions (#453)

## [4.8.1] - 2026-05-18

## [4.8.0] - 2026-05-18

### Features

- feat(ci): enable competitor benchmarks (semantic-release, changesets) in release run (#451)

## [4.7.12] - 2026-05-18

## [4.7.11] - 2026-05-18

## [4.7.10] - 2026-05-18

## [4.7.9] - 2026-05-17

### Bug Fixes

- fix(git): shell out to git push for tags to bypass libgit2 push revwalk bug (E2006) (#446)

## [4.7.8] - 2026-05-17

### Bug Fixes

- fix(git): refresh ODB before push_tags and classify stale-ODB errors as transient (E2006) (#445)

## [4.7.7] - 2026-05-15

### Bug Fixes

- fix(git): retry transient push errors with exponential backoff (E2003/E2006/E2008) (#444)

## [4.7.6] - 2026-05-14

### Bug Fixes

- fix(publish): warm up rustup + prepend cargo bin to PATH on macos-15-arm64 (#442)

## [4.7.5] - 2026-05-14

### Bug Fixes

- fix(release): upload + publish steps use GITHUB_TOKEN to access drafts (#441)

## [4.7.4] - 2026-05-14

### Bug Fixes

- fix(release): wait step uses GITHUB_TOKEN to list drafts (FERRFLOW_TOKEN PAT lacks scope) (#440)

## [4.7.3] - 2026-05-14

### Bug Fixes

- fix(release): wait via list+filter and self-heal missing draft (#439)

## [4.7.2] - 2026-05-13

### Bug Fixes

- fix(release): create GitHub Release before pushing tag + retry guard in Publish workflow (#438)

## [4.7.1] - 2026-05-13

### Bug Fixes

- fix: audit batch (#426, #427, #429, #430, #432) (#435)

## [4.7.0] - 2026-05-05

### Features

- feat(telemetry): honor DO_NOT_TRACK + print first-run notice (#409)

## [4.6.3] - 2026-04-29

### Bug Fixes

- fix(deny): allow-wildcard-paths is a bool, set to true for workspace path deps (#404)

## [4.6.2] - 2026-04-29

### Bug Fixes

- fix(ci): rename allow-wildcards-in-private to allow-wildcard-paths (cargo-deny) (#403)

## [4.6.1] - 2026-04-26

### Bug Fixes

- perf(ci): build micro-bench binary once, share across matrix shards (#398)

## [4.6.0] - 2026-04-26

### Features

- feat(bot): set git user.name/user.email from inside the binary (#396)

## [4.5.0] - 2026-04-25

### Features

- feat(formats): per-file selector + Maven-aware default for XML (#389)

### Bug Fixes

- fix(release): regenerate release commit on push rejection instead of rebasing (#394)
- fix(ci): release job pushes as github-actions[bot] instead of ferrflow[bot] (#391)

## [4.4.0] - 2026-04-22

### Features

- feat(ci): matrix-shard micro benchmarks across runners (#379)
- feat(cli): handle bot OIDC exchange in rust, drop node dependency from action (#375)

### Bug Fixes

- fix(action): ensure v4.3.0 action identity fix is tagged (re-release) (#381)
- fix(action): set git identity to ferrflow[bot] when bot: true (#380)
- fix(tests): use here-string in fixture runner to avoid sigpipe under pipefail (#378)

## [4.3.0] - 2026-04-22

### Features

- feat(ci): matrix-shard micro benchmarks across runners (#379)
- feat(cli): handle bot OIDC exchange in rust, drop node dependency from action (#375)

### Bug Fixes

- fix(tests): use here-string in fixture runner to avoid sigpipe under pipefail (#378)

## [4.2.0] - 2026-04-22

### Features

- feat(cli): handle bot OIDC exchange in rust, drop node dependency from action (#375)

## [4.1.0] - 2026-04-21

### Features

- feat(action): bot: true flag for hosted ferrflow[bot] identity (#373)

## Unreleased

### Added

- Action input `bot: true` opts into the hosted FerrFlow bot identity. Requires `permissions: { id-token: write }`. Releases are authored by `ferrflow[bot]`. See README for setup.

### Changed

- `bot: true` flow now performs the OIDC exchange directly in the Rust CLI. Users no longer need `actions/setup-node` (or any Node runtime) on minimal self-hosted runners. The `action.yml` step that shelled out to `node -e` is removed.

## [4.0.2] - 2026-04-21

### Bug Fixes

- fix(ci): rebrand GHCR + GitHub URLs from ferrflow-org to ferrlabs (#371)

## [4.0.1] - 2026-04-20

## [4.0.0] - 2026-04-19

### Breaking Changes

- feat(formats)!: add PubspecYaml, MixExs, ChartYaml, Gemspec, PackageSwift variants (#364) (#369)

## [3.2.1] - 2026-04-19

### Bug Fixes

- fix(git): correct merge_trees argument order in fetch_and_rebase (#368)

## [3.2.0] - 2026-04-18

### Features

- feat(config): auto-detect versioning strategy from existing tags (#361)

## [3.1.0] - 2026-04-18

### Features

- feat(action): support Windows runners in install script (#360)

## [3.0.3] - 2026-04-17

### Bug Fixes

- fix(release): bootstrap first release when no prior tag exists (#359)

## [3.0.2] - 2026-04-16

### Bug Fixes

- fix(release): use highest-semver git tag as version source of truth (#357)

## [3.0.1] - 2026-04-15

## [3.0.0] - 2026-04-15

### Breaking Changes

- chore!: switch license from MIT to MPL-2.0 (#345)

## [2.27.0] - 2026-04-15

### Features

- feat(cli): add --comment flag to post PR/MR preview comments (#344)

## [2.26.0] - 2026-04-14

### Features

- feat(cli): send error telemetry with command name and error code (#343)

## [2.25.0] - 2026-04-14

### Features

- feat(cli): assign error codes to all error sites (#342)

## [2.24.0] - 2026-04-14

### Features

- feat(cli): add ErrorCode infrastructure for structured error output (#341)

## [2.23.0] - 2026-04-11

### Features

- feat(bench): add benchmarks for git operations, validate, and full check flow (#327)

## [2.22.1] - 2026-04-10

### Bug Fixes

- fix(git): rebase release commits on non-fast-forward push failure (#322)

## [2.22.0] - 2026-04-10

### Features

- feat(cli): add --force-version flag to release command (#320)

## [2.21.0] - 2026-04-10

### Features

- feat(config): add releaseCommitScope option for per-package commits (#319)

## [2.20.3] - 2026-04-10

### Bug Fixes

- fix(versioning): strip pre-release suffix before computing next version (#318)
- fix(wasm): gate git2 usage in default_branch behind cli feature (#317)

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
