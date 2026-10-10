---
title: 'FerrFlow: the Helm publisher reads the right chart name'
summary: 'In a Chart.yaml listing dependencies or maintainers before the chart name, the Helm publisher could take a dependency name for the chart and check or push the wrong reference. Only the top-level name key counts now.'
date: 2026-09-28T10:10:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/1231
---

The `helm` publisher reads the chart name from `Chart.yaml` to build the OCI reference it checks and pushes. It matched the first `name:` on any line, indented or not, so a chart whose `dependencies:` or `maintainers:` block came before its own `name:` was treated as the dependency: the "already published" check looked at the wrong reference, and the push went there too.

The publisher now only reads the top-level `name:` key, whatever order the file is written in. Charts that put `name:` first were never affected.
