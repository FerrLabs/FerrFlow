---
title: 'FerrFlow: a hotfix no longer moves the latest alias backward'
summary: 'With latestTag set, releasing 1.2.4 after 2.0.0 pointed latest at 1.2.4. latest now stays on the highest version; the hotfix release still goes through, and --force moves the alias back if you want it to.'
date: 2026-10-03T12:15:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/1262
docsLink: https://ferrflow.com/docs/configuration/config-file/
---

The `latestTag` alias followed whichever stable release ran last, so cutting a hotfix on a maintenance line pointed `latest` at an older version than the one most people should install. It now behaves like the floating `v1` and `v1.5` tags and never moves to a lower version on its own.

Unlike those tags, a backward `latest` does not stop the release: shipping a hotfix on an older line is a normal thing to do, so the release completes, `latest` stays put, and the output says `Kept latest on 2.0.0: 1.2.4 is older`. If you relied on `latest` meaning "most recently released", pass `--force` to that release and the alias moves back as before, with a warning.
