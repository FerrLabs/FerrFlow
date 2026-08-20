---
title: 'FerrFlow · Six new release hook points — post-commit, pre-tag, post-tag, pre-release, on-success, on-error'
summary: The release lifecycle gains six new hook points so you can run commands at the exact commit/tag/push boundaries — publish between tag and push, notify once when a release finishes, or react to a failure with the error code in hand.
date: 2026-07-13T09:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/657
docsLink: https://ferrflow.com/docs/configuration/config-file#hooks
---

FerrFlow already ran shell hooks at `pre_bump`, `post_bump`, `pre_commit`, `pre_publish`, and `post_publish`. That left gaps around the most delicate part of a release — the commit, the tag, and the push. This release fills them.

- **`post_commit`** — after the release commit, before tagging.
- **`pre_tag`** — after the commit, immediately before `git tag`. Smoke-test the bumped tree before the tag lands.
- **`post_tag`** — after tags are created, before push. This is the natural place to `cargo publish`: the tag exists locally but isn't pushed yet, so a failed publish is recoverable by dropping the local tag instead of rewriting remote history.
- **`pre_release`** — after the release pull request opens, before merge (`releaseCommitMode: pr` only). Post a review ping or run cross-repo validation.
- **`on_success`** — once, after the whole release finishes cleanly. Trigger a downstream deploy.
- **`on_error`** — once, when a release fails. It receives the failing error code in `FERRFLOW_ERROR_CODE`, so a Slack alert can link straight to the error reference.

```json
{
  "workspace": {
    "hooks": {
      "postTag": "cargo publish",
      "onSuccess": "curl -X POST $DEPLOY_WEBHOOK",
      "onError": "curl -X POST $SLACK_WEBHOOK -d \"release failed: $FERRFLOW_ERROR_CODE\""
    }
  }
}
```

Per-package hooks (`post_commit`, `pre_tag`, `post_tag`) run once per released package; the once-per-run hooks (`pre_release`, `on_success`, `on_error`) fire a single time for the whole invocation. Note that `on_error` (the reactive hook) is distinct from `on_failure`, which stays what it always was — the `abort` / `continue` strategy for how a failing hook is handled.
