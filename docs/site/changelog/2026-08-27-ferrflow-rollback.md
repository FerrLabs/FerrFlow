---
title: 'FerrFlow: undo a release that failed partway'
summary: 'A release that dies after pushing tags used to leave you reconstructing what it did from a job log, then deleting refs by hand. ferrflow rollback reads the checkpoint that run left behind and undoes exactly what it did, refusing the parts that cannot be undone.'
date: 2026-08-27T12:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/946
docsLink: https://ferrflow.com/docs/cli/rollback/
---

Releases fail in the middle. A registry answers 401, a docker build breaks, a hook aborts. By then the tags are pushed and the release commit is on the branch, and FerrFlow could only go forward: a checkpoint let you resume, never rewind. Getting back to a clean state meant working out which tags the run had pushed, deleting each one, removing the forge releases by hand, reverting the commit and repairing the manifest. On a monorepo with a cascade that is a lot of state to reconstruct from a job log, under time pressure.

`ferrflow rollback` does it from the checkpoint instead of from memory:

```bash
ferrflow rollback          # prints the plan, changes nothing
ferrflow rollback --yes    # applies it
ferrflow rollback --yes api web
```

The dry run is the default, since the destructive form deletes remote refs.

What makes it safe is mostly what it refuses to do. It never deletes a tag that has moved: every tag is recorded with the commit it pointed at, and one that no longer matches is reported and skipped, because a rollback can happen long after the failure and a tag someone else recreated is not that run's to remove.

It also stops on any package already published to a registry that cannot be unpublished. crates.io and PyPI keep every version forever, and npm refuses to republish an unpublished one, so deleting the tag would strand a version anyone can still install. Rollback says so and leaves that package alone; the right answer is a new patch version. Docker tags, Helm charts, release assets and webhooks are replaceable and block nothing. In a monorepo where one package published and three did not, the three roll back and the one is refused, rather than half of each being undone.

The release commit is reverted only when the whole run rolls back with nothing blocked, because that commit carries every package's version bump and reverting it while one package stays released would quietly undo that package's version too. The revert is left for you to push: rollback has just deleted remote refs, and forcing a branch update on top of that is a decision worth taking with the branch in front of you.
