---
title: 'FerrFlow: a txt selector no longer eats the spaces around your version'
summary: 'A versioned file read through a txt selector whose capture included padding lost all of that padding on the first release, because the write replaced the whole capture. It now replaces only what the read returned.'
date: 2026-09-24T12:05:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/1190
docsLink: https://ferrflow.com/schema/ferrflow.json
---

A `txt` versioned file can name a `selector`, a regex with one capture group around the version. The write replaced the whole of that capture, so any padding the regex swallowed went with it: `version = 1.2.3` with `(?m)^version =(.+)$` came back as `version =2.0.0`.

It goes all at once, on the first release, and every release after that writes into a line that has already been stripped. The selector still matches a line that lost its space, so nothing ever reported it.

Both sides now work from the same trimmed range: the write replaces exactly what the read returned and leaves the rest of the line alone. If your selector captures padding, the padding stays where you put it.

Two smaller things come with it. A selector whose capture is only whitespace is now an error instead of an empty version on its way to a tag name, and a selector whose group never participates, which an optional group like `(?m)^version(?: =(.*))?$` allows, now fails with `E4702` and a link to the docs rather than a bare message.

This came out of the review of the read-side trim rather than a report, so you are unlikely to have chased it: until that trim landed, a padded capture also produced a padded version, and the version is what you would have noticed first.
