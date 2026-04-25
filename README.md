# FerrFlow

[![Latest release](https://img.shields.io/github/v/release/FerrLabs/FerrFlow)](https://github.com/FerrLabs/FerrFlow/releases/latest)
[![Coverage](https://codecov.io/gh/FerrLabs/FerrFlow/graph/badge.svg)](https://codecov.io/gh/FerrLabs/FerrFlow)
[![License](https://img.shields.io/github/license/FerrLabs/FerrFlow)](LICENSE)
[![Socket Badge](https://badge.socket.dev/npm/package/ferrflow/latest)](https://badge.socket.dev/npm/package/ferrflow/latest)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/FerrLabs/FerrFlow/badge)](https://scorecard.dev/viewer/?uri=github.com/FerrLabs/FerrFlow)

Universal semantic versioning for monorepos and classic repos.

FerrFlow reads your commit history, determines the right version bump, updates your version files, generates a changelog, and creates a tagged release — for any language, any repo layout.

## Why FerrFlow?

A single compiled binary with no runtime dependencies. Native monorepo support, multi-language versioning, and works with any repo layout.

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
| `gomod` | `go.mod` | Go | git tag only — no file write |
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
docker run ghcr.io/ferrflow/ferrflow:latest check
```

**Pre-built binaries**

Download from [Releases](https://github.com/FerrLabs/FerrFlow/releases).

## Usage

```bash
# Preview what would be bumped
ferrflow check

# Run a release
ferrflow release

# Dry run
ferrflow release --dry-run

# Force a specific version (skips commit analysis)
ferrflow release --force-version 2.0.0          # single repo
ferrflow release --force-version api@3.0.0      # monorepo

# Pre-release
ferrflow release --channel beta

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
      "versioned_files": [
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
      versioned_files: [
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
      "shared_paths": ["services/shared/"],
      "versioned_files": [
        { "path": "services/api/Cargo.toml", "format": "toml" }
      ]
    },
    {
      "name": "frontend",
      "path": "frontend",
      "changelog": "frontend/CHANGELOG.md",
      "versioned_files": [
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

Hook commands receive environment variables: `FERRFLOW_PACKAGE`, `FERRFLOW_VERSION`, `FERRFLOW_PREV_VERSION`, `FERRFLOW_CHANNEL`, `FERRFLOW_IS_PRERELEASE`.

## Conventional Commits

FerrFlow follows the [Conventional Commits](https://www.conventionalcommits.org/) spec.

| Prefix | Bump |
|--------|------|
| `fix:`, `perf:`, `refactor:` | patch |
| `feat:` | minor |
| `feat!:`, `BREAKING CHANGE` | major |
| `chore:`, `docs:`, `ci:` | none |

## CI usage

**GitLab CI**

```yaml
release:
  image: ghcr.io/ferrflow/ferrflow:latest
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
  - uses: FerrLabs/FerrFlow@v4
    with:
      bot: true
```

That's it — no `setup-node`, no extra dependencies. FerrFlow's Rust binary handles the OIDC exchange directly, so minimal self-hosted runners work out of the box.

Three auth modes are supported: `bot: true` uses the hosted FerrFlow App (recommended); `token: <PAT>` uses a personal access token or your own GitHub App token (DIY); omitting both falls back to the workflow's `GITHUB_TOKEN` (simplest, but release events won't trigger downstream workflows).

## License

[MPL-2.0](LICENSE)

