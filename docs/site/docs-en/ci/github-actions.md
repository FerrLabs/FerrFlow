---
title: GitHub Actions
description: Run FerrFlow releases automatically in GitHub Actions.
---

## Using the official action

The easiest way to use FerrFlow in GitHub Actions is the `FerrLabs/ferrflow@v5` action. It installs the binary and runs `ferrflow release` automatically.

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

      - uses: FerrLabs/ferrflow@v5
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

Release commits and tags use the repository's git identity. If the job sets none, FerrFlow commits as `github-actions[bot]`, the account a `GITHUB_TOKEN` push is attributed to anyway. Outside GitHub Actions, a release with no `user.name` and `user.email` stops before bumping anything.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Prefer not to manage a token? Set <code>bot: true</code> to author releases as <code>ferrflow[bot]</code> with zero secrets: see the <a href="/docs/ci/hosted-bot">Hosted bot</a> guide.</p>
</div></aside>

## Signing the release tag

GitHub signs the commits it creates through its API, which is why a release commit made with `bot: true` shows as verified. It never signs a tag object, and a GitHub App holds no key of its own, so the tag FerrFlow pushes is unsigned unless you give the action a signing key.

```yaml
- uses: FerrLabs/FerrFlow@v7
  with:
    bot: true
    tag_signing_key: ${{ secrets.TAG_SIGNING_KEY }}
    tag_signing_name: ${{ vars.TAG_SIGNING_NAME }}
    tag_signing_email: ${{ vars.TAG_SIGNING_EMAIL }}
```

The key is an unencrypted OpenSSH private key, and the tagger it signs as has to be an account GitHub can check the signature against:

```bash
ssh-keygen -t ed25519 -C "releases" -N "" -f ferrflow-tag-signing
```

Add `ferrflow-tag-signing.pub` to that account under **Settings > SSH and GPG keys > New SSH key** with the key type **Signing Key**, store the private half as the `TAG_SIGNING_KEY` secret, and set `tag_signing_email` to one of the account's verified emails. GitHub verifies a tag against the keys of the account whose verified email the tagger carries, so a mismatch there shows as unverified rather than failing the release.

The action writes the key under `RUNNER_TEMP` for the job only, and it never reaches the repository or the tag itself. Only the tag is signed: the release commit already carries GitHub's own signature in bot mode, and the release archives are signed separately with cosign.

## Accessing the release output

The action exposes the new version as an output you can use in downstream steps:

```yaml title=".github/workflows/release.yml"
- uses: FerrLabs/ferrflow@v5
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
      - uses: FerrLabs/ferrflow@v5
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
- uses: FerrLabs/ferrflow@v5
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
# Creates api@v1.3.0 and site@v0.5.1 in one step if both changed
```
