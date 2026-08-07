---
title: Hosted bot (ferrflow[bot])
description: Author releases as ferrflow[bot] with zero secrets, using the hosted FerrFlow GitHub App and OIDC token exchange.
---

By default, the releases and tags FerrFlow pushes are authored by whoever owns the token in your workflow — usually a personal access token, so releases show up under _your_ account. The hosted bot lets releases be authored by **`ferrflow[bot]`** instead, with a clean, consistent identity across every repo.

It works like Renovate or Dependabot:

- **Zero secrets** in your workflow — no PAT to create, store, or rotate.
- Releases authored by **`ferrflow[bot]`**.
- **Short-lived, scoped tokens** — each run gets a fresh token that expires in an hour and is scoped to the single repository.

## 1. Install the app

Go to **[github.com/apps/ferrflow](https://github.com/apps/ferrflow)** and click **Install**, then choose the organization and repositories you want FerrFlow to release. That's it — there are no secrets to create.

The app asks only for **Contents** (read & write, to push tags and create releases) and **Metadata** (read). You can review or uninstall it at any time from your organization's settings.

## 2. Enable it in your workflow

Add `bot: true` to the action and grant the workflow permission to mint an OIDC token:

```yaml title=".github/workflows/release.yml"
name: Release

on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      id-token: write # lets the runner prove the repo's identity to FerrFlow
      contents: read # for checkout; the release push uses the bot token
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0 # full history is needed for commit scanning

      - uses: FerrLabs/FerrFlow@v5
        with:
          bot: true
```

`permissions.id-token: write` is required — it's what lets the runner request the OIDC token that proves which repository is calling. Without it, FerrFlow stops with a clear error rather than falling back silently.

## How it works

No secret ever leaves your repository. Each run exchanges a proof of identity for a token:

1. The GitHub Actions runner issues a short-lived **OIDC token** describing your repository (audience `ferrflow.ferrlabs.com`).
2. FerrFlow sends that token to **`api.ferrflow.com`**, which verifies it against GitHub's public keys.
3. The service signs a **scoped installation token** for the FerrFlow app on your repo (using a private key that never leaves FerrLabs' KMS) and returns it. The token lives for one hour.
4. FerrFlow uses that token to push tags, commits, and releases — so they're authored by `ferrflow[bot]`.

## Security model

- **The app's private key never leaves FerrLabs' servers** — it lives in a KMS and is only ever used to sign installation tokens server-side.
- **Your identity is proven by OIDC, not a shared secret** — nothing sensitive is stored in your repo or transmitted from it.
- **Tokens are minimal and short-lived** — scoped to one repository, expiring after an hour.
- **You stay in control** — uninstall the app at any time to revoke all access immediately.

## Troubleshooting

FerrFlow never falls back silently: if bot mode can't get a token, it fails with a message naming the exact cause.

| Message                                                           | Cause and fix                                                                                          |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `bot mode requires permissions: id-token: write in your workflow` | The job is missing `permissions.id-token: write`. Add it (see above).                                  |
| `FerrFlow App not installed on this repository's owner`           | Install the app at [github.com/apps/ferrflow](https://github.com/apps/ferrflow) for this org / repo.   |
| `FerrFlow hosted bot rate limit hit (429)`                        | Token requests are rate-limited per repository. Retry shortly, or use a PAT via `token:` for that run. |
| `FerrFlow hosted bot service unavailable`                         | A transient service issue. Check [status.ferrlabs.com](https://status.ferrlabs.com) or retry.          |

## Alternatives

The hosted bot is the recommended path, but it isn't the only one:

- **`token:` with a PAT** — supply your own [personal access token](/docs/ci/github-actions). Releases are authored by that token's owner. Works anywhere, including non-GitHub-Actions CI.
- **`token:` with your own GitHub App** — if you'd rather run your own bot identity, pass a token minted from your own app.
- **Default `GITHUB_TOKEN`** — the simplest option, but note that pushes made with `GITHUB_TOKEN` **do not trigger downstream workflows** (so a tag push won't kick off a separate publish job). The hosted bot and PATs don't have this limitation.
