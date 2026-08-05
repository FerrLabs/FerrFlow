---
title: CLI commands
description: Full reference for all FerrFlow CLI commands and flags.
---

## `ferrflow release`

Run the full release pipeline: bump versions, update changelogs, commit, tag, push, and create a release.

```bash
ferrflow release [OPTIONS]
```

| Flag                        | Description                                                                                                      |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `--dry-run`                 | Preview all changes without writing, committing, or pushing                                                      |
| `--force`                   | Allow floating tags to move backward to a lower version                                                          |
| `--force-version <VERSION>` | Force a specific version, skipping commit analysis. Format: `VERSION` (single repo) or `NAME@VERSION` (monorepo) |
| `--verbose`, `-v`           | Show detailed output including commit hashes and file diffs                                                      |

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

Preview what `ferrflow release` would do without making any changes. Equivalent to `ferrflow release --dry-run`.

```bash
ferrflow check
```

---

## `ferrflow changelog`

Generate or update `CHANGELOG.md` only, without bumping versions or creating tags.

```bash
ferrflow changelog [OPTIONS]
```

| Flag        | Description                                       |
| ----------- | ------------------------------------------------- |
| `--dry-run` | Print the changelog entry without writing to disk |

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

## Global flags

These flags work with all commands:

| Flag              | Description                                                                                         |
| ----------------- | --------------------------------------------------------------------------------------------------- |
| `--config <PATH>` | Path to a custom config file (default: auto-detected). Also accepts `FERRFLOW_CONFIG` env variable. |
| `--version`       | Print the FerrFlow version and exit                                                                 |
| `--help`, `-h`    | Print help                                                                                          |
