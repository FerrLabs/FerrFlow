---
title: Introduction
description: What FerrFlow is and why it exists.
---

FerrFlow is a single binary that automates semantic versioning for any repository — monorepo or classic, any language.

It reads your commit history, determines the right version bump, updates your version files, writes a changelog, creates a git tag, and publishes a release. Zero runtime dependencies.

<div class="ferr-card-group" data-cols="2">
  <div class="ferr-card"><p class="ferr-card__title">CLI-first</p><div class="ferr-card__body"><p>Everything happens from your terminal or your CI. No UI to click, no config server to babysit.</p>
</div></div>
  <div class="ferr-card"><p class="ferr-card__title">Multi-forge</p><div class="ferr-card__body"><p>GitHub, GitLab, self-hosted — FerrFlow adapts to your forge. One tool, any platform.</p>
</div></div>
  <div class="ferr-card"><p class="ferr-card__title">Conventional commits</p><div class="ferr-card__body"><p>Reads commit history to determine version bumps automatically. No manual changelog maintenance.</p>
</div></div>
  <div class="ferr-card"><p class="ferr-card__title">Zero infra</p><div class="ferr-card__body"><p>A single binary with no daemon, no server, no database. Runs wherever your CI runs.</p>
</div></div>
</div>

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

For side-by-side latency, peak memory and install size against the JS ecosystem release tools, see [Performance](/performance) — numbers refresh on every FerrFlow release.

<aside class="ferr-aside ferr-aside--note"><p class="ferr-aside__title">Heads up</p><div class="ferr-aside__body"><p>FerrFlow is versioning only. Issue tracking, secrets, and AI agents live in separate FerrLabs products.</p>
</div></aside>

## How it works

1. **Reads commits** since the last git tag for each package
2. **Determines the bump** from [Conventional Commits](/docs/reference/conventional-commits) (`feat` → minor, `fix` → patch, breaking → major)
3. **Updates version files** — `Cargo.toml`, `package.json`, `pom.xml`, etc.
4. **Writes the changelog** in Keep a Changelog format
5. **Creates a git tag** (`api@v1.2.0`) and pushes
6. **Publishes a GitHub/GitLab release** with the changelog as release notes

In a monorepo, FerrFlow only releases packages that have changed, and understands shared dependency paths.

## Key features

- **Pre/post-release hooks** — run scripts at every lifecycle stage (bump, commit, publish, failure)
- **Query commands** — `ferrflow version`, `ferrflow tag`, and `ferrflow status` for CI scripting, plus `ferrflow why` to explain a package's release decision
- **Any version file** — Cargo.toml, package.json, pom.xml, build.gradle, Chart.yaml, plain text, and more
- **Browser support** — `@ferrflow/wasm` brings commit parsing, bump computation, and changelog generation to the browser
