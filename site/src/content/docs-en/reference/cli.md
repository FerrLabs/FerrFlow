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

## `ferrflow migrate`

Generate a FerrFlow config from an existing release tool's configuration. Point it at your repo and it writes the equivalent `ferrflow.json`.

```bash
ferrflow migrate [OPTIONS]
```

| Flag            | Description                                                                                               |
| --------------- | --------------------------------------------------------------------------------------------------------- |
| `--from <TOOL>` | Source: `semantic-release`, `changesets`, `release-please`, `standard-version`. Auto-detected if omitted. |

### Sources

| Tool               | Reads                           | Highlights of what maps                                                                                                                                                            |
| ------------------ | ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `semantic-release` | `.releaserc`, `.releaserc.json` | `tagFormat` → `tagTemplate`; `branches` → channels; `@semantic-release/exec` → `hooks`; `changelog` / `github` / `gitlab` plugins (see the plugin table below)                     |
| `release-please`   | `release-please-config.json`    | the `packages` map → FerrFlow packages (per-package `release-type` → the right version file/format); `include-component-in-tag` → `tagTemplate`; PR flow → `releaseCommitMode: pr` |
| `standard-version` | `.versionrc`, `.versionrc.json` | `tagPrefix` → `tagTemplate`; `bumpFiles` / `packageFiles` → `versionedFiles`                                                                                                       |
| `changesets`       | `.changeset/config.json`        | `baseBranch` → `branch`; `linked` / `fixed` → version groups (see note)                                                                                                            |

semantic-release plugin mapping:

| semantic-release                      | FerrFlow                                                                                                                                                  |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tagFormat: "v${version}"`            | `tagTemplate: "v{{version}}"`                                                                                                                             |
| `branches`                            | `branches` — `main`/`master` become the stable line, a `prerelease: true` (or named) branch becomes a channel                                             |
| `@semantic-release/changelog`         | the package's `changelog` path                                                                                                                            |
| `@semantic-release/exec`              | `hooks` (`prepareCmd` → `preBump`, `publishCmd` → `postPublish`, `successCmd` → `onSuccess`, `failCmd` → `onError`, `verifyConditionsCmd` → `preRelease`) |
| `@semantic-release/github` / `gitlab` | `forge`                                                                                                                                                   |

Anything without a FerrFlow equivalent is **surfaced, never guessed**. Each run prints what it mapped, what it ignored, and what needs manual review — for example `@semantic-release/npm` (configure `publishers` by hand), custom `commit-analyzer` release rules (FerrFlow's bump rules are fixed), and `repositoryUrl` (FerrFlow derives the remote from git). It won't overwrite an existing FerrFlow config.

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p><strong>changesets.</strong> changesets versions from hand-written <code>.changeset/*.md</code> files, while FerrFlow versions from conventional commits — after migrating you adopt Conventional Commits, and your existing changeset files aren't read. FerrFlow reads your workspace declaration (<code>workspaces</code> in <code>package.json</code>, or <code>pnpm-workspace.yaml</code>) and scaffolds one <code>package</code> entry per discovered package, so <code>linked</code>/<code>fixed</code> groups already reference real packages and the migrated config validates as-is. A repo with no workspace declaration gets a single root package.</p>
</div></aside>

```bash
ferrflow migrate                        # auto-detect
ferrflow migrate --from release-please
```

JSON, YAML, and JavaScript source configs all work — a JavaScript config (`.releaserc.js`, `release.config.js`, `.versionrc.js`) is evaluated with `node` (so it needs Node.js on PATH), and a YAML config (`.releaserc.yaml`, `.versionrc.yaml`) is parsed directly. After migrating, review the generated config, then run `ferrflow validate` and `ferrflow check`.

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

## `ferrflow why`

Explain the release decision for a single package: whether it counts as touched, which commits were classified and how, what its dependencies are doing, and the bump that falls out of all of it. This is the command to reach for when a package did not release and it is not obvious why.

```bash
ferrflow why [PACKAGE] [OPTIONS]
```

| Argument / flag       | Description                                                                              |
| --------------------- | ---------------------------------------------------------------------------------------- |
| `[PACKAGE]`           | Package name — required in a monorepo, optional (and inferred) in a single-package repo. |
| `--channel <CHANNEL>` | Explain the decision for a prerelease channel (`beta`, `rc`, …) instead of a stable run. |
| `--json`              | Emit the explanation as a structured JSON object instead of the human view.              |

```
Package: web
  Path:          web
  Strategy:      semver
  Version:       1.0.0

Last tag: web@v1.0.0 (7dcc20b, 3 days ago, reachable from HEAD)

Touch check (changed files at HEAD):
  ✗ core/src/api.rs                                no match
→ not touched

Dependencies:
  core                     bumping (major)      propagate: patch → patch

Decision: patch bump from the dependency cascade — 1.0.0 → 1.0.1, tag web@v1.0.1
```

The report reads top to bottom as the decision is made:

- **Last tag** — the tag the range starts from, when it was cut, and whether its commit is still reachable from `HEAD`. A tag that is _not_ reachable (rebased away, force pushed) is why a package can look released and still replay its whole history.
- **Touch check** — the file set that decided whether the package is in scope, with the `path` or `sharedPaths` prefix each file matched. The set is `HEAD`'s changed files, or everything since the last tag when [`recoverMissedReleases`](/docs/configuration/config-file/) pulled the package back in.
- **Commits considered** — every commit back to the package's last tag with its individual bump. Bump classification is _not_ path-scoped, so a `feat!:` on a sibling package still counts here; this section is usually where an unexpected major comes from.
- **Dependencies** — each `dependsOn` entry, whether that upstream is moving this run, its `propagate` policy, and what the policy resolves to.
- **Decision** — the bump, the version range, the tag, and whether it came from the package's own commits or from the dependency cascade.

The verdict comes from the same planning code `ferrflow release` runs, so `why` and the next release cannot disagree.

---

## `ferrflow diff`

Compare two versions of a package: the commits that went in, each commit's bump, the files changed, and the changelog FerrFlow would generate for the range. Handy for auditing a release, checking why a version bumped the way it did, or writing release notes for a range after the fact.

```bash
ferrflow diff [PACKAGE] <FROM>..<TO> [--json]
```

| Argument / flag | Description                                                                                   |
| --------------- | --------------------------------------------------------------------------------------------- |
| `<FROM>..<TO>`  | The version range. Each side is a tag or version — `v1.4.0`, or a full tag like `api@v1.6.0`. |
| `[PACKAGE]`     | Package name — required in a monorepo, optional (and inferred) in a single-package repo.      |
| `--json`        | Emit the comparison as a structured JSON object instead of the human view.                    |

Each endpoint resolves by trying the string as a tag first (a real tag, or `v1.4.0` in a single-package repo), then as the package's tag for that version (`api@v1.4.0`).

```bash
ferrflow diff v1.4.0..v1.6.0            # single-package repo
ferrflow diff api v1.4.0..v1.6.0        # monorepo — name the package
```

The output lists every commit in the range with its individual bump (`major` / `minor` / `patch` / `none`), highlights breaking changes, summarises the changed files, and renders the changelog section for the range. In a monorepo the range is scoped to the named package: only commits touching its `path` or `sharedPaths` are considered.

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

## `ferrflow doctor`

Run read-only diagnostics on the repo, config, and forge setup and print a categorised report — the "is my setup sane?" command. Use it on a fresh checkout to see what's missing before the first release, or when a run behaves unexpectedly and you'd otherwise be staring at `--verbose` logs.

```bash
ferrflow doctor [OPTIONS]
```

| Flag             | Description                                                           |
| ---------------- | --------------------------------------------------------------------- |
| `--format <FMT>` | `human` (default) or `json`                                           |
| `--online`       | Also probe the forge API (GitHub rate limit / auth); requires a token |

The report groups checks into five sections — **Repo** (git repository, commit history, clean working tree, remote, tags), **Config** (which config file wins, whether it parses, plus the full `ferrflow validate` check suite), **Versioning** (strategy and each package's on-disk version), **Forge** (detected forge and whether an auth token is present in the environment), and **CI** (workflow files, and whether a workflow pins the `FerrLabs/FerrFlow` action). Every check reports green, a warning, or an error.

The exit code is scriptable: `0` when everything is green, `1` if there are only warnings, `2` if any check errors. `--format json` has a stable shape — `{ status, exit_code, sections: [{ title, checks: [{ name, status, detail }] }] }` — so CI can assert on it.

```bash
ferrflow doctor                 # human report
ferrflow doctor --format json   # machine-readable, stable for CI
ferrflow doctor --online        # also check the GitHub API rate limit
```

---

## `ferrflow completions`

Generate a shell completion script and print it to stdout.

```bash
ferrflow completions <SHELL>
```

`<SHELL>` is one of `bash`, `elvish`, `fish`, `powershell`, or `zsh`.

---

## `ferrflow schema`

Print the JSON schema for the ferrflow config file. The schema is bundled into the binary, so this works offline — no network call to `ferrflow.com/schema/ferrflow.json`.

```bash
ferrflow schema [OPTIONS]
```

| Flag              | Description                                           |
| ----------------- | ----------------------------------------------------- |
| `--pretty`        | Format the output instead of compact single-line JSON |
| `--output <FILE>` | Write to a file instead of stdout                     |

Use it to point an editor at a local copy, or to validate `.ferrflow.json` in a pre-commit hook with no internet access:

```bash
ferrflow schema --pretty --output ferrflow.schema.json
```

Then set `"$schema": "./ferrflow.schema.json"` in your config. The command parses the bundled schema before printing, so it exits non-zero if the build artefact is somehow corrupt.

---

## Global flags

These flags work with all commands:

| Flag                    | Description                                                                                                                                                                             |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--dry-run`             | Show what would happen without making any changes                                                                                                                                       |
| `--verbose`, `-v`       | Verbose output, including commit hashes and file diffs                                                                                                                                  |
| `--log-format <FORMAT>` | Diagnostic output format on stderr: `human` (default, colored) or `json` (one structured event per line). Command **data** (`--json`, `version` / `tag` values) always stays on stdout. |
| `--config <PATH>`       | Path to a custom config file (default: auto-detected). Also accepts the `FERRFLOW_CONFIG` env variable.                                                                                 |
| `--jobs <N>`            | Max threads for CPU-parallel work (per-package planning). Default: all logical cores; `1` forces single-threaded. Also accepts the `FERRFLOW_JOBS` env variable.                        |
| `--version`             | Print the FerrFlow version and exit                                                                                                                                                     |
| `--help`, `-h`          | Print help                                                                                                                                                                              |

## Logging & output

FerrFlow separates **data** from **logs** across the two output streams:

- **stdout** carries data — the `--json` output of `check` / `release` / `status` / `validate`, and the value printed by `version` and `tag`. Capture it in scripts: `V=$(ferrflow version)`.
- **stderr** carries the human status report and every diagnostic event.

So you can capture the machine result and the run log independently:

```bash
ferrflow check --json > result.json 2> run.log
```

`--log-format json` renders each diagnostic as one structured JSON event per line on stderr, ready for Datadog / Loki / CloudWatch:

```json
{
  "timestamp": "2026-01-01T00:00:00Z",
  "level": "INFO",
  "fields": { "message": "✓ Updated CHANGELOG.md" },
  "target": "ferrflow::changelog"
}
```

`--verbose` (or a `RUST_LOG` filter such as `RUST_LOG=ferrflow::git=trace`) controls which levels are shown.
