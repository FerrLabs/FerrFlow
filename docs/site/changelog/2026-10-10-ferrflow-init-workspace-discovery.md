---
title: 'FerrFlow: init lists the packages of your workspace'
summary: 'ferrflow init now reads Cargo, Go and Gradle workspaces as well as npm and pnpm ones, and offers to scaffold one package per member instead of asking you to type them.'
date: 2026-10-10T15:00:00Z
product: ferrflow
type: new
docsLink: https://ferrflow.com/docs/reference/cli
---

Setting up a monorepo meant typing every package name, path and versioned file by hand, even though the workspace file already listed them. `ferrflow init` now reads that file and shows what it found before asking anything else.

It understands `workspaces` in `package.json` and `pnpm-workspace.yaml`, `[workspace] members` and `exclude` in `Cargo.toml`, `use` entries in `go.work`, and `include` calls in `settings.gradle` or `settings.gradle.kts`. Each member becomes a package with its manifest as the versioned file and a changelog beside it.

A Cargo workspace whose members use `version.workspace = true` is scaffolded as a single package at the root, since that is where the version lives.

Answer `n` to the prompt to fall back to entering packages by hand, as before.
