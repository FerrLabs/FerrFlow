---
title: 'FerrFlow: calver-short-seq, and a warning when a release is held back'
summary: 'A date-only version scheme can only publish once a day, and the second attempt was indistinguishable from having nothing to release. FerrFlow now says so out loud, and calver-short-seq gives the compact year a sequence counter.'
date: 2026-08-29T10:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/967
docsLink: https://ferrflow.com/docs/configuration/versioning-strategies/
---

A version made only of a date can be produced once a day, by definition. Under `calver-short`, the second release of the day recomputes the same `YY.M.D`, the version does not move, and FerrFlow declines to publish. That part is inherent to the scheme rather than a defect.

What was a defect is that it looked exactly like having nothing to release. Both printed "Nothing to release", so a merged fix could sit unpublished until the next day with nobody told. The only way to notice was to find later that a deployed image was missing a change that had been merged hours earlier.

FerrFlow now names it:

```
! app has releasable commits but stays at 26.8.29, so nothing was published
```

That line appears whenever there is releasable work and the strategy produces the version that is already out, whatever the strategy. It is worth having even for repositories that never hit the daily limit, since the same silence covered any misconfiguration that froze the version.

The other half is a way out. `calver-seq` already pairs the full year with a counter, `YYYY.M.SEQ`, but there was no equivalent for the compact year, so a repository on `calver-short` had nowhere to go at constant format. `calver-short-seq` fills that in with `YY.M.SEQ`:

```json
{ "workspace": { "versioning": "calver-short-seq" } }
```

Switching to it costs nothing in version continuity, which is the reason to prefer it over `calver-seq` here. The counter picks up from whatever the last release was, so a repository whose most recent version was `26.8.29` gets `26.8.30` next, then `26.8.31` the same day. Moving to `calver-seq` instead would work, but it jumps the leading number from `26` to `2026`, which is irreversible and makes the year ambiguous when reading older tags alongside newer ones.

Set the strategy explicitly if you use it. `YY.M.SEQ` and `YY.M.D` are the same shape, and a value like `26.8.29` is a plausible day as well as a plausible counter, so tag detection cannot tell them apart on its own.
