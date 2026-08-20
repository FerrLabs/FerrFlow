---
title: 'FerrFlow · control parallelism with --jobs / FERRFLOW_JOBS'
summary: FerrFlow now exposes a global --jobs <N> flag (and FERRFLOW_JOBS env var) to cap how many threads it uses for CPU-parallel work. Default stays all logical cores; --jobs 1 forces single-threaded — handy for constrained CI runners and reproducible benchmarks.
date: 2026-06-25T10:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/598
docsLink: https://ferrflow.com/docs/reference/cli/#global-flags
---

Since FerrFlow started planning packages and creating releases in parallel, two needs emerged: capping CPU use on small CI runners, and keeping benchmark numbers reproducible regardless of a machine's core count. The new global `--jobs <N>` flag — or the `FERRFLOW_JOBS` env var, with the flag taking precedence — covers both.

- **Default (unset):** all logical cores, exactly as before.
- **`--jobs N`:** caps per-package planning to N threads, and bounds the release-creation HTTP pool by N too — so `--jobs 1` makes the whole run single-threaded.

```bash
# Pin to one thread: deterministic, light on a 2-core runner
ferrflow --jobs 1 release

# Or via the environment
FERRFLOW_JOBS=4 ferrflow release
```

It works with every command, since it's a global flag.
