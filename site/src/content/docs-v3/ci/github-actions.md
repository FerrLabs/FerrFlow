---
title: GitHub Actions
description: Run FerrFlow releases automatically in GitHub Actions.
---

## Using the official action

The easiest way to use FerrFlow in GitHub Actions is the `FerrLabs/ferrflow@v3` action. It installs the binary and runs `ferrflow release` automatically.

```yaml title=".github/workflows/release.yml"
name: Release

on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write # required to push tags and create releases
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0 # full history needed for commit scanning
          token: ${{ secrets.GITHUB_TOKEN }}

      - uses: FerrLabs/ferrflow@v3
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

<aside class="ferr-aside ferr-aside--warning"><div class="ferr-aside__body"><p><code>fetch-depth: 0</code> is required. Without it, FerrFlow cannot find previous tags and will treat every commit as new.</p>
</div></aside>

## Permissions

FerrFlow needs `contents: write` to:

- Push version bump commits
- Create and push git tags
- Create GitHub Releases

If your repository has branch protection rules, create a dedicated token with the necessary permissions and pass it as `FERRFLOW_TOKEN` or configure the action's `token` input.

## Accessing the release output

The action exposes the new version as an output you can use in downstream steps:

```yaml title=".github/workflows/release.yml"
- uses: FerrLabs/ferrflow@v3
  id: ferrflow
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

- name: Build Docker image
  if: steps.ferrflow.outputs.version != ''
  run: |
    docker build -t myimage:${{ steps.ferrflow.outputs.version }} .
    docker push myimage:${{ steps.ferrflow.outputs.version }}
```

## Skip CI on release commits

FerrFlow commits version bumps with `[skip ci]` in the message by default to prevent infinite loops. No extra configuration needed.

## PR preview comments

FerrFlow can post a comment on every pull request showing what versions will be bumped when the PR is merged. The comment is automatically updated on each push.

```yaml title=".github/workflows/preview.yml"
name: FerrFlow Preview

on:
  pull_request:

permissions:
  contents: read
  pull-requests: write

jobs:
  preview:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
      - uses: FerrLabs/ferrflow@v3
        with:
          mode: preview
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

The comment looks like:

> **FerrFlow Release Preview**
>
> | Package | Current | Next    | Bump  |
> | ------- | ------- | ------- | ----- |
> | api     | `1.5.0` | `1.6.0` | minor |
> | site    | `1.8.0` | `1.8.1` | patch |
>
> Based on 3 commit(s).

If no releasable changes are detected, the comment says so.

## Monorepo example

In a monorepo, FerrFlow releases each changed package in a single run:

```yaml title=".github/workflows/release.yml"
- uses: FerrLabs/ferrflow@v3
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
# Creates api@v1.3.0 and site@v0.5.1 in one step if both changed
```
