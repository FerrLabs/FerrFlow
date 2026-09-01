<div align="center">

# FerrFlow

**Universal semantic versioning for monorepos and classic repos.**

Reads your [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/), works out the right version bump,<br />
updates your version files, writes the changelog, and cuts the tagged release. Any language, any repo layout.

[![Latest release](https://img.shields.io/github/v/release/FerrLabs/FerrFlow)](https://github.com/FerrLabs/FerrFlow/releases/latest)
[![Conventional Commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-%23FE5196?logo=conventionalcommits&logoColor=white)](https://www.conventionalcommits.org/en/v1.0.0/)
[![Quality Gate](https://sonar.ferrlabs.com/api/project_badges/measure?project=ferrflow&metric=alert_status&token=sqb_53f0d93466bd01a6c6a94a15125d5aa8390c67fa)](https://sonar.ferrlabs.com/dashboard?id=ferrflow)
[![Maintainability](https://sonar.ferrlabs.com/api/project_badges/measure?project=ferrflow&metric=sqale_rating&token=sqb_53f0d93466bd01a6c6a94a15125d5aa8390c67fa)](https://sonar.ferrlabs.com/dashboard?id=ferrflow)
[![Security](https://sonar.ferrlabs.com/api/project_badges/measure?project=ferrflow&metric=security_rating&token=sqb_53f0d93466bd01a6c6a94a15125d5aa8390c67fa)](https://sonar.ferrlabs.com/dashboard?id=ferrflow)
[![License](https://img.shields.io/github/license/FerrLabs/FerrFlow)](LICENSE)
[![Socket Badge](https://badge.socket.dev/cargo/package/ferrflow/latest)](https://socket.dev/cargo/package/ferrflow)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/FerrLabs/FerrFlow/badge)](https://scorecard.dev/viewer/?uri=github.com/FerrLabs/FerrFlow)

[Documentation](https://ferrflow.com/docs) | [Changelog](https://ferrlabs.com/changelog/) | [GitHub App](https://github.com/apps/ferrflow)

</div>

## Why FerrFlow?

One compiled binary, no runtime to install. Native monorepo support, 16 version-file formats
across every ecosystem below, and it works with whatever layout your repo already has.

```bash
ferrflow release
```

That is the whole release: bump, changelog, tag, GitHub release.

## Supported version files

| Format | File | Ecosystem | Selector |
|--------|------|-----------|----------|
| `toml` | `Cargo.toml` | Rust | `package.version` |
| `toml` | `pyproject.toml` | Python | `project.version` or `tool.poetry.version` |
| `json` | `package.json` | Node.js | `version` |
| `json` | `composer.json` | PHP | `version` |
| `xml` | `pom.xml` | Java / Maven | first `<version>` that's a direct child of the root element (skips `<parent>` and dependencies) |
| `csproj` | `*.csproj` | .NET (C#, F#) | `<Version>` in `<PropertyGroup>` |
| `gradle` | `build.gradle`, `build.gradle.kts` | Java / Kotlin | `version = "…"` |
| `helm` / `chartyaml` | `Chart.yaml` | Kubernetes / Helm | top-level `version:` |
| `pubspecyaml` | `pubspec.yaml` | Dart / Flutter | top-level `version:` |
| `mixexs` | `mix.exs` | Elixir | `version: "…"` in `def project` |
| `gemspec` | `*.gemspec` | Ruby | `s.version = "…"` |
| `packageswift` | `Package.swift` | Swift | top-level `let <name>Version = "…"` |
| `cabal` | `*.cabal` | Haskell | top-level `version:` field |
| `cmake` | `CMakeLists.txt` | C / C++ | `VERSION` argument of `project()` |
| `gomod` | `go.mod` | Go | git tag only, no file write |
| `txt` | `VERSION`, `VERSION.txt` | Any | entire file content |

## Installation

**Cargo**

```bash
cargo install ferrflow
```

**npm**

```bash
npm install -D ferrflow
```

**Docker**

```bash
docker run --rm -v "$PWD:/repo" ghcr.io/ferrlabs/ferrflow:latest check
```

The image runs as a non-root user (`ferrflow`, uid 1000) and works on `/repo`.
If your checkout is owned by a different uid, pass your own:

```bash
docker run --rm -u "$(id -u):$(id -g)" -v "$PWD:/repo" ghcr.io/ferrlabs/ferrflow:latest check
```

**Pre-built binaries**

Download from [Releases](https://github.com/FerrLabs/FerrFlow/releases).

## Usage

```bash
# Preview what would be bumped
ferrflow check

# Adjust the plan, then get the command that reproduces it
ferrflow plan --interactive

# Run a release
ferrflow release

# Dry run
ferrflow release --dry-run

# Force a specific version (skips commit analysis)
ferrflow release --force-version 2.0.0          # single repo
ferrflow release --force-version api@3.0.0      # monorepo

# Repeatable in a monorepo, and packages can be left out entirely
ferrflow release --force-version core@3.0.0 --force-version api@2.5.0
ferrflow release --exclude web --exclude docs

# Pre-release
ferrflow release --channel beta

# Undo a release that failed partway
ferrflow rollback                    # prints the plan, changes nothing
ferrflow rollback --yes              # applies it
ferrflow rollback --yes api web      # only these packages

# Scaffold a config file
ferrflow init

# Scaffold a config file in a specific format
ferrflow init --format json5

# Use a specific config file
ferrflow check --config path/to/ferrflow.toml

# Or set via environment variable
FERRFLOW_CONFIG=path/to/ferrflow.toml ferrflow check

# Print current version
ferrflow version              # single repo
ferrflow version api          # monorepo, specific package

# Print last release tag
ferrflow tag
ferrflow tag api

# Print the dependency graph, the release order, and any cycle
ferrflow graph
ferrflow graph --json

# See what releasing a package would drag along with it
ferrflow graph --impact shared
ferrflow graph --impact shared --bump major

# Compare each package's public API against its last tag (Rust: needs cargo-semver-checks)
ferrflow api-check
ferrflow api-check --json

# JSON output (for scripting)
ferrflow version --json
ferrflow tag --json

# Shell completions
ferrflow completions bash >> ~/.bash_completion
ferrflow completions zsh  > ~/.zfunc/_ferrflow
ferrflow completions fish > ~/.config/fish/completions/ferrflow.fish
```

Pre-generated completion scripts are also available as `ferrflow-completions.tar.gz` in each [GitHub release](https://github.com/FerrLabs/FerrFlow/releases).

## Configuration

FerrFlow looks for a config file at the root of your repository, in this order:

1. `ferrflow.json`
2. `ferrflow.json5`
3. `ferrflow.toml`
4. `.ferrflow` (dotfile, JSON format)

If multiple config files exist, FerrFlow exits with an error listing the conflicting files. Use `--config <path>` (or `FERRFLOW_CONFIG` env var) to specify which one to use. If no config file is found, FerrFlow auto-detects common version files in the current directory.

Run `ferrflow init` to scaffold a config file interactively. Use `--format` to skip the format prompt:

```bash
ferrflow init                  # asks which format (default: json)
ferrflow init --format json5
ferrflow init --format toml
ferrflow init --format dotfile # generates .ferrflow
```

Already using semantic-release? `ferrflow migrate` reads your existing config and generates the equivalent `ferrflow.json`:

```bash
ferrflow migrate                        # auto-detects .releaserc
ferrflow migrate --from semantic-release
```

It maps `tagFormat`, `branches`, and the common plugins (`changelog`, `exec`, `github`/`gitlab`) to their FerrFlow equivalents, and prints a summary of what mapped, what was ignored, and what needs manual review. Anything without a FerrFlow equivalent is surfaced, never guessed. JSON `.releaserc` is supported today; YAML `.releaserc` and JS `release.config.js` are reported as unsupported rather than mis-parsed.

### JSON Schema

Add `$schema` to get autocompletion and validation in VS Code, WebStorm, and any JSON-aware editor:

```json
{
  "$schema": "https://ferrflow.com/schema/ferrflow.json"
}
```

### JSON (default)

```json
{
  "$schema": "https://ferrflow.com/schema/ferrflow.json",
  "workspace": {
    "remote": "origin",
    "branch": "main"
  },
  "package": [
    {
      "name": "my-app",
      "path": ".",
      "changelog": "CHANGELOG.md",
      "versionedFiles": [
        { "path": "package.json", "format": "json" }
      ]
    }
  ]
}
```

### JSON5

```json5
{
  workspace: {
    remote: "origin",
    branch: "main",
  },
  package: [
    {
      name: "my-app",
      path: ".",
      changelog: "CHANGELOG.md",
      versionedFiles: [
        { path: "package.json", format: "json" },
      ],
    },
  ],
}
```

### TOML

```toml
[workspace]
remote = "origin"
branch = "main"

[[package]]
name = "my-app"
path = "."
changelog = "CHANGELOG.md"

[[package.versioned_files]]
path = "Cargo.toml"
format = "toml"
```

### Monorepo

<details>
<summary>JSON</summary>

```json
{
  "package": [
    {
      "name": "api",
      "path": "services/api",
      "changelog": "services/api/CHANGELOG.md",
      "sharedPaths": ["services/shared/"],
      "versionedFiles": [
        { "path": "services/api/Cargo.toml", "format": "toml" }
      ]
    },
    {
      "name": "frontend",
      "path": "frontend",
      "changelog": "frontend/CHANGELOG.md",
      "versionedFiles": [
        { "path": "frontend/package.json", "format": "json" }
      ]
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[package]]
name = "api"
path = "services/api"
changelog = "services/api/CHANGELOG.md"
shared_paths = ["services/shared/"]

[[package.versioned_files]]
path = "services/api/Cargo.toml"
format = "toml"

[[package]]
name = "frontend"
path = "frontend"
changelog = "frontend/CHANGELOG.md"

[[package.versioned_files]]
path = "frontend/package.json"
format = "json"
```

</details>

#### One file per package

A large monorepo does not have to keep every package in the root config. List the per-package files
under `include`, and each project owns its own settings:

```json
{
  "workspace": { "versioning": "semver" },
  "include": ["services/*/ferrflow.json", "frontend/ferrflow.json"]
}
```

```json
{
  "name": "api",
  "changelog": "CHANGELOG.md",
  "sharedPaths": ["../shared/"],
  "versionedFiles": [{ "path": "Cargo.toml", "format": "toml" }]
}
```

Paths inside an included file are relative to that file, and `path` defaults to its directory. The
example above needs no `path`: `services/api/ferrflow.json` describes the package in
`services/api`, and moving the directory does not require editing anything.

Included files use the same keys as a `package` entry, so `dependsOn` still refers to packages by
name and works across files. They may use a different format than the root config, and they can be
mixed with an inline `package` array while you migrate.

The following are rejected rather than silently ignored: an `include` pattern matching no file, two
packages sharing a name, and an included file declaring `workspace`, `include`, or `package`.

## Versioning Strategies

Each package can use its own versioning strategy. Set a default at the workspace level and override per package:

```toml
[workspace]
versioning = "semver"  # default for all packages

[[package]]
name = "api"
path = "packages/api"
# inherits semver from workspace

[[package]]
name = "site"
path = "packages/site"
versioning = "calver"  # override: date-based
```

| Strategy | Format | Example | Description |
|----------|--------|---------|-------------|
| `semver` | `MAJOR.MINOR.PATCH` | `1.4.2` | Default, driven by conventional commits |
| `calver` | `YYYY.M.D` | `2025.3.28` | Date-based, ignores commit types |
| `calver-short` | `YY.M.D` | `25.3.28` | Compact date-based |
| `calver-seq` | `YYYY.M.SEQ` | `2025.3.3` | Date + daily sequence counter |
| `calver-short-seq` | `YY.M.SEQ` | `25.3.3` | Compact date + sequence counter |
| `sequential` | `N` | `42` | Simple incrementing build number |
| `zerover` | `0.MINOR.PATCH` | `0.15.2` | Permanently unstable, never hits 1.0 |

## Tag Template

By default, FerrFlow tags single-repo releases as `v1.2.3` and monorepo releases as `api@v1.2.3`. Customize with `tag_template` at the workspace or package level using `{name}` and `{version}` placeholders.

```toml
[workspace]
tag_template = "v{version}"  # all packages: v1.2.3

[[package]]
name = "api"
path = "packages/api"
tag_template = "{name}/v{version}"  # override: api/v1.2.3
```

| Layout | Default template | Example tag |
|--------|-----------------|-------------|
| Single repo | `v{version}` | `v1.2.3` |
| Monorepo | `{name}@v{version}` | `api@v1.2.3` |
| Custom | `release-{version}` | `release-1.2.3` |

## Pre-release Channels

Publish pre-release versions (alpha, beta, rc, dev) using the `--channel` flag or branch-based configuration. Pre-release versions follow the format `MAJOR.MINOR.PATCH-CHANNEL.IDENTIFIER`.

### CLI flag

```bash
ferrflow release --channel beta       # 2.0.0-beta.1
ferrflow check --channel rc           # preview pre-release version
```

### Branch-based configuration

Map branches to channels automatically:

```json
{
  "workspace": {
    "branches": [
      { "name": "main", "channel": false },
      { "name": "develop", "channel": "dev", "prereleaseIdentifier": "timestamp" },
      { "name": "release/*", "channel": "rc" }
    ]
  }
}
```

Branch names support glob patterns. The first match wins. Wildcards match across
`/` separators, so `*` matches branches like `fix/global` and `feature/*` matches
`feature/auth/oauth`.

### Identifier strategies

| Strategy | Example | Description |
|----------|---------|-------------|
| `increment` | `-beta.3` | Auto-incrementing counter (default) |
| `timestamp` | `-dev.20250402T1430` | UTC timestamp |
| `short-hash` | `-dev.a1b2c3d` | Git short hash |
| `timestamp-hash` | `-dev.20250402T1430-a1b2c3d` | Timestamp + hash |

### Behavior

- Floating tags (e.g. `v1`, `v1.2`) are never moved by pre-release versions
- GitHub Releases are marked as pre-release
- Stable releases include all commits since the last stable tag (skipping pre-release tags)
- Hook environment includes `FERRFLOW_CHANNEL` and `FERRFLOW_IS_PRERELEASE`

## Release Commit Mode

Controls how FerrFlow commits version bumps and changelog updates after a release.

```toml
[workspace]
release_commit_mode = "commit"  # default
```

| Mode | Description |
|------|-------------|
| `commit` | Push a release commit directly to the branch |
| `pr` | Create a pull request with the release changes |
| `none` | Skip committing entirely (useful when another tool handles it) |

When using `pr` mode, `auto_merge_releases` controls whether the PR is automatically merged:

```toml
[workspace]
release_commit_mode = "pr"
auto_merge_releases = true  # default
```

### Release Commit Scope

In monorepo mode, controls whether all package bumps go into a single commit or one commit per package:

```toml
[workspace]
release_commit_scope = "grouped"  # default
```

| Scope | Description |
|-------|-------------|
| `grouped` | Single commit for all packages (e.g. `chore(release): api v1.0.0, site v2.1.0`) |
| `per-package` | One commit per package (e.g. `chore(release): api v1.0.0`, then `chore(release): site v2.1.0`) |

Per-package commits make it easier to revert a single package bump without affecting others. This works with both `commit` and `pr` release modes.

### Skip CI

By default, release commits in `commit` mode include `[skip ci]` in the message to avoid triggering a CI loop. Override with `skip_ci`:

```toml
[workspace]
skip_ci = false  # force CI to run on release commits
```

In `pr` mode, `skip_ci` defaults to `false` since the PR merge triggers CI naturally.

### How `pr` mode releases

`pr` mode runs in two phases, so nothing is published for a release that has not been accepted.

The **proposing** run computes the bump, writes the version files and changelog onto the release
branch, and opens or updates the pull request. No tags are created and no releases are published.
Every later commit on the target branch regenerates the branch, so the open PR keeps tracking the
version that would ship now.

The **finalising** run happens after the PR merges. FerrFlow sees a `chore(release):` commit on the
target branch whose versions carry no tag, and tags exactly those versions before publishing the
releases. The versions are read from the version files, never recomputed, so merging a release PR
cannot cascade into a further bump. Squash merges and merge commits both work.

A package declared without `versionedFiles` has no version to read, so it is not finalised this way.
Use `commit` mode for tag-only packages.

## Rollback

A release that fails partway leaves tags pushed, forge releases created and a release commit on the
branch. `ferrflow rollback` undoes exactly what that run did, reading the checkpoint it left behind
rather than guessing from the log.

```bash
ferrflow rollback
```

It prints the plan and changes nothing. Add `--yes` to apply it. Name packages to narrow it to a
subset; with none, it rolls back everything the run touched.

Three things it deliberately refuses to do.

It never deletes a tag that has moved. Each tag is recorded with the commit it pointed at, and the
remote is queried at rollback time to check it still points there. One that no longer matches, or
that this checkout cannot resolve on the remote at all, is reported and skipped: a tag someone else
recreated in the meantime is not this run's to remove.

It stops on a package already published to a registry that cannot be unpublished. crates.io and PyPI
keep every version forever, and npm refuses to republish an unpublished one, so deleting the tag
would leave a version anyone can install with nothing pointing at it. The right answer there is a new
patch version, and rollback says so instead of doing half the job. Docker tags, Helm charts, release
assets and webhooks are replaceable, so they do not block anything.

It reverts the release commit only on a whole-run rollback with nothing blocked. That commit carries
every package's version bump, so reverting it while one package stays released would silently undo
that package's version too.

The revert is left uncommitted to the remote on purpose. Rollback has just deleted remote refs, and
forcing a branch update on top of that is a decision worth taking with the branch in front of you.

## Floating Tags

Move abbreviated tags (e.g. `v1`, `v1.2`) to always point at the latest matching release:

```toml
[workspace]
floating_tags = ["major"]  # creates/moves v1 when releasing v1.2.3
```

| Level | Tag | Points to |
|-------|-----|-----------|
| `major` | `v1` | Latest `v1.x.x` |
| `minor` | `v1.2` | Latest `v1.2.x` |

Floating tags are never moved by pre-release versions. Override per package:

```toml
[[package]]
name = "api"
path = "packages/api"
floating_tags = ["major", "minor"]
```

## Orphaned Tag Strategy

After a rebase + force-push, existing tags may point to commits that no longer exist on the branch. `orphaned_tag_strategy` controls how FerrFlow handles this:

```toml
[workspace]
orphaned_tag_strategy = "warn"  # default
```

| Strategy | Description |
|----------|-------------|
| `warn` | Log a warning and skip the orphaned tag |
| `treeHash` | Attempt recovery by matching the commit's tree hash |
| `message` | Attempt recovery by matching the commit message |

## Recover Missed Releases

In monorepos, a package can miss a release if its files changed but FerrFlow wasn't run. Enable `recover_missed_releases` to compare files against the last tag instead of just the last commit:

```toml
[workspace]
recover_missed_releases = true  # default: false
```

## Package Dependencies

In a monorepo, use `depends_on` to automatically patch-bump a package when one of its dependencies is released:

```json
{
  "package": [
    { "name": "core", "path": "packages/core" },
    {
      "name": "cli",
      "path": "packages/cli",
      "depends_on": ["core"]
    }
  ]
}
```

When `core` is bumped, `cli` gets a patch bump even if it had no direct commits.

## Hooks

Run shell commands at lifecycle points during a release. Hooks can be set at the workspace level (applies to all packages) or per package:

```toml
[workspace.hooks]
pre_bump = "echo 'about to bump'"
post_bump = "cargo check"
pre_commit = "npm run build"
pre_publish = "npm pack --dry-run"
post_publish = "notify-slack.sh"
on_failure = "abort"  # or "continue"
```

| Hook | When |
|------|------|
| `pre_bump` | After bump calculation, before writing version files |
| `post_bump` | After writing version files, before changelog generation |
| `pre_commit` | After changelog generation, before git commit |
| `pre_publish` | After commit and tag, before push |
| `post_publish` | After push and release creation |

If a hook exits non-zero and `on_failure` is `abort` (default), the release is cancelled. Set `on_failure` to `continue` to ignore hook failures.

Hook commands receive environment variables: `FERRFLOW_PACKAGE`, `FERRFLOW_OLD_VERSION`, `FERRFLOW_NEW_VERSION`, `FERRFLOW_BUMP_TYPE`, `FERRFLOW_TAG`, `FERRFLOW_PACKAGE_PATH`, `FERRFLOW_DRY_RUN`, `FERRFLOW_CHANNEL`, `FERRFLOW_IS_PRERELEASE`.

## Conventional Commits

FerrFlow reads [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) to decide how far to bump. The spec is the contract between your history and your version numbers: write the prefix, and the bump, the changelog section and the tag follow from it.

| Prefix | Bump |
|--------|------|
| `fix:`, `perf:`, `refactor:` | patch |
| `feat:` | minor |
| `feat!:`, `BREAKING CHANGE` | major |
| `chore:`, `docs:`, `ci:` | none |

The defaults are deliberately permissive, so a repo that never enforced the spec still gets sensible bumps: capitalised and slash-separated variants (`Feat:`, `Feat/`, `feature:`, `Fix/`, `Perf:`, `Refactor/`, and so on) are accepted alongside the canonical forms. If your history uses different conventions, remap them with [`commitFormats`](https://ferrflow.com/docs/configuration/config-file/).

Nothing maps to major by default. A major bump comes only from a structural breaking marker (`feat!:`, `fix(api)!:`, or a `BREAKING CHANGE:` footer), and those are always detected whatever you configure, so a commit cannot lose its breaking status through a custom pattern.

Full reference: [ferrflow.com/docs/reference/conventional-commits](https://ferrflow.com/docs/reference/conventional-commits).

## CI usage

**GitLab CI**

```yaml
release:
  image: ghcr.io/ferrlabs/ferrflow:latest
  script:
    - ferrflow release
  rules:
    - if: '$CI_COMMIT_BRANCH == "main"'
```

**GitHub Actions**

```yaml
- name: Release
  run: ferrflow release
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### Using the hosted bot (ferrflow[bot])

Install the [FerrFlow GitHub App](https://github.com/apps/ferrflow) on your repo or org, then opt in with `bot: true`. Release commits, tags, and GitHub Releases are authored by `ferrflow[bot]` and downstream workflows triggered by those events run normally (unlike the default `GITHUB_TOKEN`, which suppresses them).

```yaml
permissions:
  id-token: write
  contents: read

steps:
  - uses: actions/checkout@v6
    with:
      fetch-depth: 0
  - uses: FerrLabs/FerrFlow@v7
    with:
      bot: true
```

That is the whole job. No `setup-node`, no extra dependencies. FerrFlow's Rust binary handles the OIDC exchange directly, so minimal self-hosted runners work out of the box.

Three auth modes are supported: `bot: true` uses the hosted FerrFlow App (recommended); `token: <PAT>` uses a personal access token or your own GitHub App token (DIY); omitting both falls back to the workflow's `GITHUB_TOKEN` (simplest, but release events won't trigger downstream workflows).

On GitHub, the release commit is authored through the forge rather than by local git when `bot: true` is set, which is what makes it show as **verified**. A GitHub App cannot register a signing key, so letting GitHub author the commit is the only way its commits are signed. GitHub does that only when the request carries no custom author, committer or signature, so the commit is attributed to `ferrflow[bot]` and nothing else.

Everywhere else, and without `bot: true`, the commit is built by local git exactly as before. If you want verified commits under your own identity, set `commit.gpgsign` and FerrFlow will honour it, on every forge.

Annotated tags are still created and pushed by git, so they are not covered by this. Only the commit is.

## License

[MIT](LICENSE)

