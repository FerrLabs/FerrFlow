---
title: CLI commands
description: Full reference for all FerrFlow CLI commands and flags.
slug: v0/docs/reference/cli
---

## `ferrflow release`

Run the full release pipeline: bump versions, update changelogs, commit, tag, push, and create a release.

```bash
ferrflow release [OPTIONS]
```

| Flag              | Description                                                 |
| ----------------- | ----------------------------------------------------------- |
| `--dry-run`       | Preview all changes without writing, committing, or pushing |
| `--verbose`, `-v` | Show detailed output including commit hashes and file diffs |

**What it does:**

1. Scans commits since the last tag for each package
2. Determines the version bump from Conventional Commits
3. Updates all `versioned_files` with the new version
4. Appends the new section to `CHANGELOG.md`
5. Creates a git commit with the version bump changes
6. Creates and pushes the git tag (`{name}@v{version}`)
7. Creates a GitHub/GitLab release with the changelog as notes

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

Scaffold a `ferrflow.toml` config file for the current repository. Detects existing version files (`Cargo.toml`, `package.json`, etc.) and generates the appropriate config.

```bash
ferrflow init
```

---

## `ferrflow status`

Show the current version of each package and whether a release would be triggered.

```bash
ferrflow status [OPTIONS]
```

| Flag     | Description                         |
| -------- | ----------------------------------- |
| `--json` | Output in JSON format for scripting |

Example output:

```
api    1.2.3   minor bump pending (1 feat commit)
site   0.4.1   no release (only chore commits)
```

---

## Global flags

These flags work with all commands:

| Flag              | Description                                             |
| ----------------- | ------------------------------------------------------- |
| `--config <PATH>` | Path to a custom config file (default: `ferrflow.toml`) |
| `--version`       | Print the FerrFlow version and exit                     |
| `--help`, `-h`    | Print help                                              |
