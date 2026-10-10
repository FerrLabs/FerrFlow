---
title: 'FerrFlow: a failed push now says what actually went wrong'
summary: 'Every push failure was reported as "pushed onto a stale branch" and answered by regenerating the release commit three times, whatever the real cause. Transient server errors are now retried where they belong, real races are detected by their actual wording, and permanent failures surface immediately.'
date: 2026-08-25T17:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/937
docsLink: https://ferrflow.com/docs/reference/errors/
---

When a release could not push, FerrFlow said this, three times, before giving up:

```
Release attempt 1/3 pushed onto a stale 'main': E2004: Failed to push branch 'main'
Resetting working tree to remote tip and regenerating the release commit against the latest history…
```

It said that whatever had happened. The check behind the message matched `E2004`, the generic "push failed" code that rides on every push error, so a GitHub server hiccup, an expired token and a genuine race all produced the same diagnosis. Anyone reading the log went looking for a concurrent push that had never occurred, and a permanent failure such as a revoked permission burned all three attempts before revealing its real cause.

Two things now decide separately. A transient server-side failure is recognised as such and retried with backoff by the push layer, which is what that layer exists for and where a hiccup is usually resolved on the second attempt. Anything that is neither transient nor a race surfaces immediately with the message git actually produced, no retries and no misleading claim about the branch being stale.

Races themselves are now detected by what git actually says, which turned out to matter more than expected. The old wording list looked for `non-fast-forward`, but git only says that once you have fetched. Push without fetching first, which is what a release job does, and git says `fetch first` instead. That case had been reaching the retry path through the generic code rather than on its own merit, so tightening the check meant covering it properly, along with `stale info` and the divergent-remote-tag case.

Nothing to change in your configuration. Release logs get more specific, transient failures stop costing you a release, and permanent ones stop pretending to be something else.
