---
title: Pipeline Triggers
description: Choose the right CI trigger strategy for your FerrFlow releases.
---

FerrFlow works with any CI trigger strategy. This page covers the most common patterns, when to use each one, and how they interact with `releaseCommitMode`.

## Push to main

The simplest setup: run `ferrflow release` on every push to the default branch. FerrFlow decides whether a release is needed based on the commits since the last tag.

```yaml title=".github/workflows/release.yml"
name: Release

on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    # Skip release commits to avoid infinite loops
    if: "!startsWith(github.event.head_commit.message, 'chore(release):')"
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
          token: ${{ secrets.FERRFLOW_TOKEN }}

      - uses: FerrLabs/ferrflow@v5
        env:
          GITHUB_TOKEN: ${{ secrets.FERRFLOW_TOKEN }}
```

**When to use:** Most projects. Simple, predictable, fully automated.

**Trade-offs:** Every push to main triggers a workflow run, even if no release is needed. FerrFlow exits early when there are no releasable commits, so the cost is minimal.

**Works with:** `releaseCommitMode: commit` (default) or `none`.

## Tag-triggered

Run your build/deploy pipeline when FerrFlow creates a new tag. This separates the release step (tagging) from the downstream steps (building, publishing, deploying).

```yaml title=".github/workflows/build.yml"
name: Build & Publish

on:
  push:
    tags:
      - 'v*' # single-repo: v1.2.0
      - '*@v*' # monorepo: api@v1.2.0, site@v0.5.1

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Extract version from tag
        id: version
        run: |
          TAG="${GITHUB_REF_NAME}"
          VERSION="${TAG##*v}"
          echo "tag=$TAG" >> "$GITHUB_OUTPUT"
          echo "version=$VERSION" >> "$GITHUB_OUTPUT"

      - name: Build
        run: echo "Building version ${{ steps.version.outputs.version }}"
```

**When to use:** When you want to decouple versioning from build/deploy. Common for Docker image builds, npm publishing, or binary releases.

**Trade-offs:** Requires two workflows: one for the release (push-to-main) and one for the downstream build (tag-triggered). Adds a few seconds of latency between the tag push and the build start.

### Monorepo: per-package builds

In a monorepo, use tag patterns to build only the package that was released:

```yaml title=".github/workflows/build.yml"
name: Build Package

on:
  push:
    tags:
      - 'api@v*'
      - 'site@v*'

jobs:
  build-api:
    if: startsWith(github.ref_name, 'api@v')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - run: echo "Building API ${{ github.ref_name }}"

  build-site:
    if: startsWith(github.ref_name, 'site@v')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - run: echo "Building site ${{ github.ref_name }}"
```

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p>If FerrFlow releases multiple packages in a single run (e.g. <code>api@v1.3.0</code> and <code>site@v0.5.1</code>), each tag triggers its own workflow run. The builds happen in parallel automatically.</p>
</div></aside>

## Release-triggered

Run a pipeline when a GitHub Release is published. This works well with FerrFlow's `--draft` flag: FerrFlow creates a draft release, you review it, then publishing triggers the build.

```yaml title=".github/workflows/deploy.yml"
name: Deploy

on:
  release:
    types: [published]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          ref: ${{ github.event.release.tag_name }}

      - name: Deploy
        run: echo "Deploying ${{ github.event.release.tag_name }}"
```

**When to use:** When you want a manual review gate before deploying. Create draft releases with `ferrflow release --draft`, review the changelog, then publish.

**Trade-offs:** Adds a manual step. The draft release must be published before the deploy runs.

**Works with:** All `releaseCommitMode` values.

## Manual (workflow_dispatch)

Trigger a release on demand with an optional dry-run flag. Useful for controlled release cadences or when you don't want every merge to potentially release.

```yaml title=".github/workflows/release.yml"
name: Release

on:
  workflow_dispatch:
    inputs:
      dry_run:
        description: 'Dry run (no tags, no commits, no releases)'
        type: boolean
        default: false

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
          token: ${{ secrets.FERRFLOW_TOKEN }}

      - uses: FerrLabs/ferrflow@v5
        with:
          args: ${{ inputs.dry_run == true && '--dry-run' || '' }}
        env:
          GITHUB_TOKEN: ${{ secrets.FERRFLOW_TOKEN }}
```

**When to use:** Teams that prefer explicit release decisions over automatic releases. Also useful as a secondary workflow alongside push-to-main for ad-hoc releases.

**Trade-offs:** Requires someone to click "Run workflow" in the Actions tab. Commits can pile up between releases, producing larger changelogs.

## PR-based

Use `releaseCommitMode: pr` to have FerrFlow open a pull request with the version bump instead of committing directly. The release completes when the PR is merged.

```yaml title="ferrflow.json"
{ 'workspace': { 'releaseCommitMode': 'pr' } }
```

```yaml title=".github/workflows/release.yml"
name: Release

on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    if: "!startsWith(github.event.head_commit.message, 'chore(release):')"
    permissions:
      contents: write
      pull-requests: write
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
          token: ${{ secrets.FERRFLOW_TOKEN }}

      - uses: FerrLabs/ferrflow@v5
        env:
          GITHUB_TOKEN: ${{ secrets.FERRFLOW_TOKEN }}
```

**When to use:** When you want to review version bumps before they land, or when branch protection prevents direct pushes to main.

**Trade-offs:** Adds an extra merge step. The PR must be merged before tags are created.

**Works with:** `releaseCommitMode: pr` only. Requires `pull-requests: write` permission.

## Combining strategies

A common production setup combines push-to-main for versioning with tag-triggered builds:

```yaml title=".github/workflows/release.yml"
# Workflow 1: Version and tag on every push to main
name: Release
on:
  push:
    branches: [main]
jobs:
  release:
    if: "!startsWith(github.event.head_commit.message, 'chore(release):')"
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
          token: ${{ secrets.FERRFLOW_TOKEN }}
      - uses: FerrLabs/ferrflow@v5
        env:
          GITHUB_TOKEN: ${{ secrets.FERRFLOW_TOKEN }}
```

```yaml title=".github/workflows/build.yml"
# Workflow 2: Build and deploy when a tag is pushed
name: Build
on:
  push:
    tags: ['v*', '*@v*']
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - run: echo "Building ${{ github.ref_name }}"
```

## Concurrency safety

Since v5.2, `ferrflow release` acquires `.git/ferrflow.lock` atomically (`O_CREAT|O_EXCL`) at the start of every mutating run. A second concurrent invocation on the same repo fails fast with a clear error rather than racing on git refs — the classic failure mode is a manually-triggered release firing at the same time as a cron-driven `auto-release`, producing half-pushed tag sets, non-fast-forward rejects, or duplicate draft releases.

You don't need to wire anything up — the lock is automatic on every `release` invocation. Read-only commands (`check`, `status`, `version`, `tag`) skip it.

If a previous run crashed without releasing the lock, the next invocation takes it over automatically after 30 minutes (the host + PID stamped inside the lockfile lets FerrFlow detect stale locks). To take it over sooner, delete `.git/ferrflow.lock` manually.

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p>The lock is per-repo, scoped to <code>.git/</code>. It does not protect across separate clones of the same repo — if you run releases concurrently from two different runners against two different checkouts of the same remote, the lock won&#39;t see the other side. Use a single release runner, or serialize at the CI level (<code>concurrency:</code> in GitHub Actions, <code>interruptible: false</code> in GitLab).</p>
</div></aside>

## Crash-resume

A release does several side-effecting steps in sequence: write versioned files → commit → tag → push commit → push tags → create GitHub Releases → run `post_publish` hooks. If the process dies between any of those (network blip, runner termination, SIGKILL), the repo is in a half-released state.

Since v5.3, FerrFlow writes a `.git/ferrflow.checkpoint.json` after each phase succeeds. The next `ferrflow release` invocation on the same commit resumes from the recorded phase and skips the work that already happened — useful at CI scale where transient failures are routine. The checkpoint is deleted automatically when a release finishes cleanly.

Two safety rails:

- HEAD-pinning: the checkpoint records the commit SHA the release was operating on. If you advance HEAD between runs, FerrFlow refuses to resume and tells you to either reset HEAD back to the recorded commit or delete the checkpoint manually. This avoids replaying old tags onto a different commit graph.
- Idempotency: the underlying git and forge operations already no-op when their target exists (a tag that's already pushed is a no-op, a release that already exists is reused), so a mid-phase crash still recovers cleanly on the next run.

To start fresh after a crash you don't want to resume from, delete `.git/ferrflow.checkpoint.json` manually.

## Summary

| Trigger           | Automatic | Review gate | Best for                   |
| ----------------- | --------- | ----------- | -------------------------- |
| Push to main      | Yes       | No          | Most projects              |
| Tag-triggered     | Yes       | No          | Decoupled build/deploy     |
| Release-triggered | No        | Yes         | Draft → review → publish   |
| Manual            | No        | Yes         | Controlled release cadence |
| PR-based          | Partial   | Yes         | Branch protection / review |
