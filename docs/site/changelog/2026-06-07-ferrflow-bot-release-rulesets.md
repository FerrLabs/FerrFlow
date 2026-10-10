---
title: 'FerrFlow: bot releases now work under branch rulesets (v5.2.2)'
summary: The FerrFlow bot identity pushes releases correctly even when the release branch is protected by a ruleset that bypasses ferrflow[bot]. Shipped in FerrFlow v5.2.2.
date: 2026-06-07T11:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/539
---

Releases run with `bot: true` now push correctly even when your release branch is protected by a GitHub ruleset that bypasses `ferrflow[bot]`. Before this fix, the release commit could be rejected with `GH013: required status checks are expected` even though `ferrflow[bot]` was listed as a bypass actor. Shipped in **FerrFlow v5.2.2**: pinning the major (`FerrLabs/FerrFlow@v5`) picks it up automatically.

### Why it happened

`actions/checkout` persists your workflow's `GITHUB_TOKEN` as an `http.<server>.extraheader` git config entry. An HTTP extra-header outranks a credential helper, so even though FerrFlow swaps in its OIDC-minted bot token, git kept sending the original `GITHUB_TOKEN` on every push. The release commit therefore landed as `github-actions[bot]`. Which is not a bypass actor: and the ruleset rejected it.

### What changes

FerrFlow now resets that persisted extra-header on its own git invocations, so its bot token is the only credential on the wire and the push lands as `ferrflow[bot]`. This works regardless of your `actions/checkout` version and no longer requires `persist-credentials: false` on the checkout step.

```yaml
- uses: actions/checkout@v6
  with:
    fetch-depth: 0
- uses: FerrLabs/FerrFlow@v5
  with:
    bot: true
```

No config changes needed: upgrade to `@v5` (or pin `@v5.2.2` explicitly) and protected release branches just work.
