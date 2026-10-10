---
title: 'FerrFlow: versions read from files never carry stray whitespace'
summary: 'A version written with padding or a line break inside its quotes came back with the whitespace attached and could not become a tag. package.json, Cargo.toml, pyproject.toml and Chart.yaml read with the helm format now trim it; mix.exs, gemspec, Gradle and Package.swift reject it with a clear error.'
date: 2026-09-28T10:15:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/1218
---

Several version file readers returned the quoted value exactly as written. `"version": " 1.2.0 "` in `package.json`, or a `version:` literal in `mix.exs` that spanned a line break, produced a version with whitespace in it, and the release then failed at `git tag` with an error about invalid ref names.

Where the version sits in a delimited string (`package.json`, `Cargo.toml`, `pyproject.toml`, and `Chart.yaml` read with the `helm` format that `ferrflow init` detects), the padding is unambiguous and is now trimmed, so those files keep working. Where the version is found by pattern (`mix.exs`, `.gemspec`, Gradle, `Package.swift`), a padded literal is now reported as "no version found" instead of being guessed at, so you can fix the file before anything is tagged.
