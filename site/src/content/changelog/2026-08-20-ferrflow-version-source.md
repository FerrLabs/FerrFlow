---
title: 'FerrFlow · release output says where the current version came from'
summary: 'The version FerrFlow starts from is resolved from a tag, a versioned file, or neither. The release and check output now names the source, so a package whose tag was never pushed is distinguishable from one that has no tags yet.'
date: 2026-08-20T10:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/870
docsLink: https://ferrflow.com/docs/reference/cli
---

FerrFlow resolves the version it starts from in one of four ways: the highest tag reachable from HEAD, the version written in the package's first versioned file, the higher of the two when both exist, or the strategy's starting point when neither does. Until now all four printed the same bare number, so four different situations were indistinguishable from the output.

That hides a real failure: a version that looks right for the wrong reason. A package whose tag was never pushed reads its version from the file and releases as if nothing were wrong, and a first release looks identical to a package that has simply lost its tags. Answering "where did that number come from" meant reading the source.

The release and check lines now name the source:

```
● discord  2026.8.18 → 2026.8.19  (calverseq, from discord/Cargo.toml)
● discord  2026.8.18 → 2026.8.19  (calverseq, from discord/Cargo.toml, over tag v2026.8.1)
● api      0.0.0     → 0.1.0      (minor, bootstrapped)
```

The same information is in `release --json` and `check --json` as a `version_source` object, tagged with a `kind` of `tag`, `file`, `tag_over_file`, `file_over_tag`, or `bootstrap`, so CI can branch on it without parsing prose. The field is additive and optional, and nothing about how versions are resolved has changed: only how much of that resolution you can see.
