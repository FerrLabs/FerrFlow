---
title: 'FerrFlow: ferrflow doctor'
summary: 'A new `ferrflow doctor` subcommand runs read-only diagnostics on your repo, config, and forge setup and prints a categorised report: so "is my setup sane?" has a first-class answer instead of grepping `--verbose` logs.'
date: 2026-07-18T09:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/725
docsLink: https://ferrflow.com/docs/reference/cli#ferrflow-doctor
---

When a release behaved unexpectedly: a wrong bump, a tag that didn't move, no commits detected: the only recourse was `--verbose` and staring at logs. `ferrflow doctor` gives you a first-class health check instead.

```bash
ferrflow doctor                 # human report
ferrflow doctor --format json   # machine-readable, stable for CI
ferrflow doctor --online        # also check the GitHub API rate limit
```

It runs a series of read-only checks and groups them into five sections: **Repo** (git repository, commit history, clean working tree, remote, tags), **Config** (which config file wins, whether it parses, plus the full `ferrflow validate` suite), **Versioning** (strategy and each package's version), **Forge** (detected forge and whether an auth token is present), and **CI** (workflow files, and whether a workflow pins the FerrFlow action). Every check is green, a warning, or an error.

Run it on a fresh checkout and it explains exactly what's missing before your first release; run it on a misconfigured repo and it pinpoints the file or value at fault. The exit code is scriptable: `0` all green, `1` warnings, `2` errors: and `--format json` has a stable shape you can assert on in CI.
