---
title: GitHub Actions
description: Run FerrFlow releases automatically in GitHub Actions.
---

## Using the official action

The easiest way to use FerrFlow in GitHub Actions is the `FerrLabs/ferrflow@v1` action. It installs the binary and runs `ferrflow release` automatically.

```yaml
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

      - uses: FerrLabs/ferrflow@v1
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

```yaml
- uses: FerrLabs/ferrflow@v1
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

## Monorepo example

In a monorepo, FerrFlow releases each changed package in a single run:

```yaml
- uses: FerrLabs/ferrflow@v1
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
# Creates api@v1.3.0 and site@v0.5.1 in one step if both changed
```
