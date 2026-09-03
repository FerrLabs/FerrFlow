---
title: ferrflow.toml
description: Complete reference for the ferrflow.toml configuration file.
slug: v0/docs/configuration/config-file
---

FerrFlow reads `ferrflow.toml` from the root of your repository. If no config file is found, it auto-detects common version files in the current directory.

## `[workspace]`

Global settings that apply to all packages.

```toml
[workspace]
remote = "origin"   # Git remote to push to (default: "origin")
branch = "main"     # Branch to push to (default: auto-detected from remote HEAD)
```

## `[[package]]`

Defines a package to version. You can have one or many.

```toml
[[package]]
name      = "api"                        # Used in git tags: api@v1.2.0
path      = "packages/api"               # Path to the package root
changelog = "packages/api/CHANGELOG.md"  # Where to write the changelog
shared_paths = ["packages/shared/"]      # Changes here also trigger this package
```

| Field          | Required | Description                                                    |
| -------------- | -------- | -------------------------------------------------------------- |
| `name`         | yes      | Package identifier, used in git tag prefix                     |
| `path`         | yes      | Relative path to the package directory                         |
| `changelog`    | no       | Path to the changelog file (defaults to `{path}/CHANGELOG.md`) |
| `shared_paths` | no       | List of paths — changes in any of them trigger this package    |

## `[[package.versioned_files]]`

Files where the version number should be updated.

```toml
[[package.versioned_files]]
path   = "packages/api/Cargo.toml"
format = "toml"
```

| `format` | File                               | Field updated                                     |
| -------- | ---------------------------------- | ------------------------------------------------- |
| `toml`   | `Cargo.toml`, `pyproject.toml`     | `[package].version`                               |
| `json`   | `package.json`                     | `version`                                         |
| `xml`    | `pom.xml`                          | First `<version>` element                         |
| `gradle` | `build.gradle`, `build.gradle.kts` | `version = "..."`                                 |
| `gomod`  | `go.mod`                           | No file update — version comes from git tags only |

A package can have multiple versioned files:

```toml
[[package.versioned_files]]
path   = "Cargo.toml"
format = "toml"

[[package.versioned_files]]
path   = "npm/package.json"
format = "json"
```

## Complete example

```toml
[workspace]
remote = "origin"
branch = "main"

[[package]]
name      = "api"
path      = "packages/api"
changelog = "packages/api/CHANGELOG.md"
shared_paths = ["packages/shared/"]

[[package.versioned_files]]
path   = "packages/api/Cargo.toml"
format = "toml"

[[package]]
name      = "site"
path      = "packages/site"
changelog = "packages/site/CHANGELOG.md"
shared_paths = ["packages/shared/"]

[[package.versioned_files]]
path   = "packages/site/package.json"
format = "json"
```

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Run <code>ferrflow init</code> to generate this file automatically based on what FerrFlow detects in your repo.</p>
</div></aside>
