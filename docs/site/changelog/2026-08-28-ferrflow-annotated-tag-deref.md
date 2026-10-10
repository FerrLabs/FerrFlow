---
title: 'FerrFlow: an annotated tag already on the remote no longer blocks the push'
summary: 'FerrFlow compared the commit a tag points at against the tag object itself, so any annotated tag already on the remote looked like it had moved. The push failed with a message about a difference that was not there, telling you to delete a perfectly healthy tag.'
date: 2026-08-28T10:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/955
docsLink: https://ferrflow.com/docs/reference/errors/
---

FerrFlow checks the remote before pushing tags, so that a tag someone else already put there at a different commit stops the release instead of being silently overwritten. The check compared the wrong pair of values.

An annotated tag is two objects: the tag itself, carrying its message, and the commit it points at. FerrFlow read the commit locally and the tag object remotely, then compared them. They are never equal, so any annotated tag already on the remote was reported as diverged:

```
E2006: Tag(s) already exist on remote pointing to a different commit:
v1.0.0 (local ebf8a82 != remote ffed861)
```

Both refer to the same tag at the same commit. The message names a commit difference that does not exist, and advises deleting a tag that is fine.

FerrFlow creates annotated tags, so this was the ordinary case rather than an edge one. It stayed hidden because the release planner normally skips a package whose tag already exists, so the push is never attempted. The gap opened when the local view of tags was stale while the remote had the tag, most plausibly after a failed tag fetch at the start of a release, whose error was not surfaced.

The cause was one refspec. Asking `git ls-remote` for `refs/tags/<tag>` does not return the `refs/tags/<tag>^{}` entry that carries the commit, so the dereferencing step had nothing to work with. FerrFlow now asks for both. Lightweight tags were never affected, since for them the tag and the commit are the same object, which is also why every existing test in this area missed it.

Nothing to change on your side. A genuinely divergent tag still stops the release, with the same message, which now means what it says.
