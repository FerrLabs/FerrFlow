---
title: 'FerrFlow: a crash no longer leaves the release lock or a registry token behind'
summary: 'The release binary is built to abort on panic, which skips the cleanup that runs when a release ends normally. A crash left the lock file in place, blocking the repository for half an hour, and could leave a scoped npm token on disk.'
date: 2026-08-28T18:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/964
docsLink: https://ferrflow.com/docs/cli/release/
---

FerrFlow takes a lock while a release runs, so two releases cannot start at once in the same repository, and writes a temporary npm config when a registry needs a token. Both are removed when the run ends, whether it succeeded or failed.

Neither was removed when the process crashed. The release binary is compiled to abort immediately on a panic rather than unwind, a deliberate choice that cuts the binary by about a quarter, and aborting skips the cleanup step that normally runs as each value goes out of scope. The consequence had not been followed through.

So a crash mid-release left the lock file in place. The next release refused to start, and since a stale lock is only taken over after a timeout, the repository stayed blocked for half an hour with nothing explaining why. A crash during an npm publish could likewise leave a file containing a registry token in the system temporary directory until the operating system got around to clearing it.

FerrFlow now keeps a list of files that must not outlive the process and removes them from a panic handler, which does still run when the binary aborts. The normal path is unchanged: whatever was already cleaned up is taken off the list rather than deleted twice.

One gap remains, and it is worth knowing about: interrupting a local release with Ctrl-C still leaves the lock behind, for the same underlying reason. Handling that needs machinery we did not want to add without weighing it separately. If it happens, `ferrflow release --force-unlock` takes the lock back immediately.
