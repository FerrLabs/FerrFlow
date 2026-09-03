---
title: Introduction
description: What FerrFlow is and why it exists.
---

FerrFlow is a single binary that automates semantic versioning for any repository — monorepo or classic, any language.

It reads your commit history, determines the right version bump, updates your version files, writes a changelog, creates a git tag, and publishes a release. Zero runtime dependencies.

## Why not semantic-release or changesets?

Most versioning tools are coupled to a specific ecosystem or require Node.js to be present in your CI.

| Tool             | Monorepo    | Multi-language | Runtime  |
| ---------------- | ----------- | -------------- | -------- |
| semantic-release | via plugins | JS/Node only   | Node.js  |
| changesets       | manual bump | JS only        | Node.js  |
| release-please   | limited     | partial        | Node.js  |
| cargo-release    | no          | Rust only      | Rust     |
| **FerrFlow**     | **native**  | **any**        | **none** |

FerrFlow ships as a compiled binary. Drop it in any CI environment without installing a runtime. A WASM build (`@ferrflow/wasm`) is also available for browser-side usage.

## How it works

1. **Reads commits** since the last git tag for each package
2. **Determines the bump** from [Conventional Commits](/docs/reference/conventional-commits) (`feat` → minor, `fix` → patch, breaking → major)
3. **Updates version files** — `Cargo.toml`, `package.json`, `pom.xml`, etc.
4. **Writes the changelog** in Keep a Changelog format
5. **Creates a git tag** (`api@v1.2.0`) and pushes
6. **Publishes a GitHub/GitLab release** with the changelog as release notes

In a monorepo, FerrFlow only releases packages that have changed, and understands shared dependency paths.

## Key features

- **Query commands** — `ferrflow version`, `ferrflow tag`, and `ferrflow status` for CI scripting
- **Any version file** — Cargo.toml, package.json, pom.xml, build.gradle, plain text, and more
- **Browser support** — `@ferrflow/wasm` brings commit parsing, bump computation, and changelog generation to the browser
