---
title: 'FerrFlow: a backtick in a version no longer breaks the PR preview table'
summary: 'The release preview comment wrapped versions in a code span that a backtick could close early, scrambling the table. Version cells now use a fence long enough for whatever they contain.'
date: 2026-10-03T12:05:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/1260
---

`ferrflow check --comment` posts a table of current and next versions on the pull request. Each version sat in a single-backtick code span, and the backslash used to escape a backtick inside it is ignored by Markdown, so a version containing a backtick closed the span and broke the row.

Version cells are now fenced with one more backtick than the longest run in the value, as CommonMark specifies, so the table renders the version exactly as written.
