---
title: CLI commands
description: Full reference for all FerrFlow CLI commands and flags.
---

## `ferrflow release`

Run the full release pipeline: bump versions, update changelogs, commit, tag, push, and create a release.

```bash
ferrflow release [OPTIONS]
```

| Flag                        | Description                                                                                                                                                    |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--force`                   | Allow floating tags to move backward to a lower version                                                                                                        |
| `--force-version <VERSION>` | Force a specific version, skipping commit analysis. Format: `VERSION` (single repo) or `NAME@VERSION` (monorepo)                                               |
| `--channel <NAME>`          | Pre-release channel override (e.g. `beta`, `rc`, `dev`)                                                                                                        |
| `--draft`                   | Create releases as drafts (GitHub only). A later `ferrflow release` without `--draft` detects and publishes existing drafts automatically                      |
| `--force-unlock`            | Break an existing `.git/ferrflow.lock` before acquiring it. Use only when no other `ferrflow release` is running — e.g. after a crash left the lockfile behind |

**What it does:**

1. Scans commits since the last tag for each package
2. Determines the version bump from Conventional Commits
3. Updates all `versionedFiles` with the new version
4. Appends the new section to `CHANGELOG.md`
5. Creates a git commit, opens a PR, or skips (depending on `releaseCommitMode`)
6. Creates and pushes the git tag
7. Creates a GitHub/GitLab release with the changelog as notes

### Which version is bumped from

Starting with FerrFlow **v3**, the baseline for every bump is **the highest semver-valid tag** for the package (e.g. `my-pkg@v2.4.1` or `v2.4.1`), not the value in the versioned file.

The versioned file stays the canonical write target so downstream consumers (`cargo publish`, Docker builds, etc.) always see a coherent version, but it is no longer the source of truth for the bump computation. This prevents two classes of silent failure:

- **Parallel release workflows**: two pull requests merging back-to-back used to spawn two release jobs that both read the pre-release version from the file. Both computed the same next version — the second push either collided or was silently skipped. Today the second workflow sees the first workflow's freshly-pushed tag and computes the correct next version on top of it.
- **File/tag drift**: a revert, a merge from an old branch, or a manual edit could leave the file behind the tags. Bumping from a stale file produced tags that collided with history and the release got silently skipped with `tag X already exists, skipping`. The tag now wins; the file only wins when it is genuinely ahead (human pre-bump).

Resolution order, per package:

| Tag     | File    | Baseline used                  |
| ------- | ------- | ------------------------------ |
| present | present | `max(tag, file)` by semver     |
| present | absent  | tag                            |
| absent  | present | file                           |
| absent  | absent  | strategy bootstrap (see below) |

### First release on a brand-new repo

When no tag exists yet _and_ the format has no version to read (notably `go.mod`, which stores the version in tags alone), FerrFlow bootstraps from the versioning strategy's zero value:

| Strategy                 | Bootstrap baseline                       |
| ------------------------ | ---------------------------------------- |
| `semver`, `zerover`      | `0.0.0`                                  |
| `sequential`             | `0`                                      |
| `calver-seq`             | `0.0`                                    |
| `calver`, `calver-short` | ignored — bump derives from today's date |

From there the first `feat:` commit bumps to `0.1.0` / `1` / today's date / … and the release flow creates the tag itself — no `git tag foo@v0.0.0` ceremony required before the first run.

---

## `ferrflow check`

Preview what `ferrflow release` would do without making any changes.

```bash
ferrflow check [OPTIONS]
```

| Flag               | Description                                             |
| ------------------ | ------------------------------------------------------- |
| `--json`           | Output as JSON                                          |
| `--channel <NAME>` | Pre-release channel override (e.g. `beta`, `rc`, `dev`) |
| `--comment`        | Post a preview comment on the current PR/MR             |

---

## `ferrflow publish`

Run the configured [publishers](/docs/configuration/config-file/#publishers) for the currently-released version of each package — without bumping, committing, or tagging. `ferrflow release` already runs your publishers at the end of a release; `ferrflow publish` is for when you'd rather run them in a **separate CI job** that has the build toolchain and registry auth the publishers need (docker buildx, helm, a built `dist/`, …) which your release job may not.

```bash
ferrflow publish [PACKAGES...]
```

| Argument / flag | Description                                                                                                                                                                             |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `[PACKAGES...]` | Publish these packages by name (space-separated). Omit to auto-detect from the triggering tag (`GITHUB_REF` / `CI_COMMIT_TAG`), falling back to every package that declares publishers. |
| `--all`, `-a`   | Publish every package, ignoring any triggering-tag scope.                                                                                                                               |

It reads each package's current version from its `versionedFiles` (or the latest matching tag for tag-only packages), so run it **after** `ferrflow release` has cut the version. Publishers are idempotent: anything already on the registry is skipped, so a re-run is safe. Use the global `--dry-run` to preview without publishing.

**Scope resolution.** With no arguments, if the run was triggered by a package tag (e.g. `api@v2.2.1`), only that package is published — so a single tag-triggered workflow publishes each package on its own tag, with no per-package wiring. Without a matching tag (for example the release job's own branch ref), every package is published, as before. Pass package names to target a subset explicitly, or `--all` to force every package even under a tag.

The GitHub Action exposes this as `mode: publish` — it installs the binary and runs `ferrflow publish` for you, scoping to the triggering tag automatically (or pass the `package` input to override). A tag-triggered job only has to set up the toolchain its publishers need:

```yaml title=".github/workflows/publish.yml"
on:
  push:
    # `v*` for single-package repos; `*@v*` for monorepo per-package tags
    tags: ['v*', '*@v*']
jobs:
  publish:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v6
      - uses: docker/setup-buildx-action@v4
      - uses: docker/login-action@v4
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: FerrLabs/FerrFlow@v5
        with:
          mode: publish
```

---

## `ferrflow changelog`

Generate or update `CHANGELOG.md` only, without bumping versions or creating tags.

```bash
ferrflow changelog
```

Takes no command-specific flags. Use the global `--dry-run` to print the entry without writing it.

---

## `ferrflow init`

Scaffold a config file for the current repository. Detects existing version files (`Cargo.toml`, `package.json`, etc.) and generates the appropriate config.

```bash
ferrflow init [OPTIONS]
```

| Flag                | Description                                    |
| ------------------- | ---------------------------------------------- |
| `--format <FORMAT>` | Config file format: `json`, `json5`, or `toml` |

---

## `ferrflow status`

Show the current version of each package and whether a release would be triggered.

```bash
ferrflow status [OPTIONS]
```

| Flag                | Description                               |
| ------------------- | ----------------------------------------- |
| `--output <FORMAT>` | Output format: `text` (default) or `json` |

Example output:

```
api    1.2.3   minor bump pending (1 feat commit)
site   0.4.1   no release (only chore commits)
```

---

## `ferrflow version`

Print the current version of one or all packages. Useful in CI scripts.

```bash
ferrflow version [PACKAGE] [OPTIONS]
```

| Flag     | Description    |
| -------- | -------------- |
| `--json` | Output as JSON |

Returns the version from the latest git tag matching the package's tag template.

---

## `ferrflow tag`

Print the latest tag for one or all packages.

```bash
ferrflow tag [PACKAGE] [OPTIONS]
```

| Flag     | Description    |
| -------- | -------------- |
| `--json` | Output as JSON |

---

## `ferrflow validate`

Validate the config and the versioned files it points at, without bumping anything. Pass `--repo` to validate a remote repository instead of the working tree.

```bash
ferrflow validate [OPTIONS]
```

| Flag            | Description                                                                             |
| --------------- | --------------------------------------------------------------------------------------- |
| `--json`        | Output as JSON                                                                          |
| `--repo <REPO>` | Remote repository to validate (e.g. `owner/repo` for GitHub, or `gitlab:group/project`) |
| `--ref <REF>`   | Git ref for remote validation (branch, tag, or commit)                                  |

---

## `ferrflow completions`

Generate a shell completion script and print it to stdout.

```bash
ferrflow completions <SHELL>
```

`<SHELL>` is one of `bash`, `elvish`, `fish`, `powershell`, or `zsh`.

---

## Global flags

These flags work with all commands:

| Flag              | Description                                                                                                                                                      |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--dry-run`       | Show what would happen without making any changes                                                                                                                |
| `--verbose`, `-v` | Verbose output, including commit hashes and file diffs                                                                                                           |
| `--config <PATH>` | Path to a custom config file (default: auto-detected). Also accepts the `FERRFLOW_CONFIG` env variable.                                                          |
| `--jobs <N>`      | Max threads for CPU-parallel work (per-package planning). Default: all logical cores; `1` forces single-threaded. Also accepts the `FERRFLOW_JOBS` env variable. |
| `--version`       | Print the FerrFlow version and exit                                                                                                                              |
| `--help`, `-h`    | Print help                                                                                                                                                       |
