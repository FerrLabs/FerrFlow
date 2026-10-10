---
title: 'FerrFlow · choose whether the tag or the file is the source of truth'
summary: 'When a package has both a git tag and a version in a versioned file, FerrFlow took whichever was higher. That is still the default, and versionSource now lets a repo say the tags are the record, or that the file is.'
date: 2026-08-20T12:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/873
docsLink: https://ferrflow.com/docs/configuration/config-file/
---

FerrFlow resolves the version it starts from by looking at two places: the highest tag reachable from HEAD, and the version written in the package's first versioned file. When both exist it took whichever was higher. That is a good default and remains the default, but it has a cost worth naming: a mistake in either source only ever ratchets upward. A bad merge that resurrects an old `version =` line, a hand-edit, a generator writing a future date into a manifest, and FerrFlow releases from the inflated number with nothing to walk it back.

The reverse case is just as real, which is why this is a choice rather than a fix. Moving a package between repositories carries its manifest across while the tags stay behind, and there the file is the honest source.

`versionSource` lets a repo say which one it trusts:

```jsonc
{
  "workspace": {
    "versionSource": "highest" // default, unchanged
    // "tag"  - the tags are the record of what shipped, ignore the file
    // "file" - the file is the source, ignore the tag
  }
}
```

It can be set per package too, so a monorepo migrating one package does not have to flip the policy for the rest. With only one source present the setting has no effect, and neither does it change how the version is then bumped.

Nothing changes if you upgrade without touching your config. The release and check output now says which source a version came from and whether it won on height or because you configured it, so you can tell whether your repo needs the option at all before reaching for it.
