# Diagrams

| Diagram | Covers |
|---|---|
| [release-algorithm.md](release-algorithm.md) | Commit analysis → bump → manifest writes → commit/tag/push/publish, the crash-resume checkpoint, and why the step order is what it is |

Mermaid, so GitHub renders inline.

**Keep it current.** Read it before changing the release path, and update it in the same PR.

The ordering claims are the load-bearing part — "git lands before the forge", the lock taken before
any mutating step, the checkpoint refusing a stale HEAD. Those read as incidental and are not: each
one is what keeps a failed release recoverable instead of leaving a half-published mess.
