---
title: 'FerrFlow: hooks can no longer hang a release'
summary: 'Every hook now has a timeout, 15 minutes by default, after which FerrFlow kills it and everything it started. Hooks also run with stdin closed, so a command that would prompt fails at once instead of waiting forever.'
date: 2026-10-10T10:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/1300
docsLink: https://ferrflow.com/docs/configuration/config-file/
---

A hook that never returned used to hold the release, and the release lock, until the CI job hit its own wall-clock limit. The usual causes are mundane: a command asking for a password on a terminal nobody is watching, a `curl` without a time limit, a test suite that deadlocks.

Hooks now run with stdin closed, which turns the prompting case into an immediate failure, and each hook gets a time limit. When it is reached, FerrFlow kills the hook along with any process it started in the background and treats it as a failed hook, so `onFailure` decides what happens next: `abort` stops the release, `continue` warns and carries on. A long hook prints that it is still running once a minute, which tells a slow build apart from a stuck one.

The default is 900 seconds. Raise or lower it for the workspace or for a single package:

```json
{
  "workspace": {
    "hooks": { "prePublish": "npm run build", "timeout": 1800 }
  }
}
```
