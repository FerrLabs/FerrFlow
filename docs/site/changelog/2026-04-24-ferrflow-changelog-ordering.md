---
title: 'FerrFlow: changelog ordering fix'
summary: 'Changelog entries are now sorted by commit date instead of alphabetically inside each section.'
date: 2026-04-24T11:15:00Z
product: 'ferrflow'
type: 'fix'
---

Changelog entries were sorted alphabetically inside each section instead of by commit date. They are now grouped by date descending, which matches what every other release tool does and what humans actually expect.

Affects everyone running v4.x. Re-run `ferrflow changelog` to regenerate the file with the corrected ordering.
