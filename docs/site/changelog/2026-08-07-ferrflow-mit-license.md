---
title: 'FerrFlow: now MIT licensed'
summary: 'FerrFlow moves from MPL-2.0 to MIT. Fewer obligations for everyone using it; the change ships as v6, so Action pins move from @v5 to @v6.'
date: 2026-08-07T10:00:00Z
product: ferrflow
type: breaking
prLink: https://github.com/FerrLabs/FerrFlow/pull/813
---

FerrFlow is now distributed under the MIT licence instead of MPL-2.0. This applies to the CLI, the Rust library published on crates.io, the `ferrflow` npm package and its platform binaries, and the WASM build.

MPL-2.0 is weak copyleft: modify a FerrFlow source file and you had to publish that file. In practice that obligation almost never applied. FerrFlow is invoked, not linked: but it was enough to route the tool into legal review at organisations whose dependency policy allowlists MIT, BSD and Apache and flags everything else. That review always ended in approval, and always cost weeks. MIT removes the question.

**Nothing is taken away.** Every right you had under MPL-2.0 you still have under MIT, plus the file-level reciprocity obligation is gone. Releases up to 5.52.x remain available under MPL-2.0 in perpetuity if you have a reason to prefer them.

**What you need to do:** if you pin the GitHub Action to a floating major tag, move it to `@v6`.

```yaml
- uses: FerrLabs/FerrFlow@v6
  with:
    bot: true
```

`v5` stays where it is and will not receive further releases. Nothing else changes. No flags, no config fields, no output formats. The major bump exists so that "which licence?" has a clean answer per major line: v5 and earlier are MPL-2.0, v6 onward is MIT. Pinning a full version (`@v6.0.0`) or a commit SHA is unaffected.

If you consume FerrFlow as a Rust library, `cargo update` picks the new licence up with the version; if your build enforces a licence allowlist, MIT should already be on it.
