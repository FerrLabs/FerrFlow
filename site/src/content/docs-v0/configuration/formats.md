---
title: Supported formats
description: Version file formats that FerrFlow can read and update.
slug: v0/docs/configuration/formats
---

## TOML

Used by Rust (`Cargo.toml`) and Python (`pyproject.toml`).

FerrFlow updates the `version` field under `[package]`, `[project]`, or `[tool.poetry]`.

```toml
[package]
name = "my-crate"
version = "1.2.3"   # ← updated
```

## JSON

Used by Node.js (`package.json`).

FerrFlow updates the top-level `version` field.

```json
{
  "name": "my-package",
  "version": "1.2.3"
}
```

## XML

Used by Java/Maven (`pom.xml`).

FerrFlow updates the first `<version>` element it encounters.

```xml
<project>
  <groupId>com.example</groupId>
  <artifactId>my-app</artifactId>
  <version>1.2.3</version>   <!-- updated -->
</project>
```

## Gradle

Used by Java/Kotlin Gradle projects (`build.gradle`, `build.gradle.kts`).

FerrFlow updates the `version = "..."` assignment.

```groovy
version = "1.2.3"   // updated
```

## Plain text

Used for simple version files (`VERSION`, `VERSION.txt`).

FerrFlow replaces the entire file content with the version number.

```
1.2.3
```

## Go modules

Used by Go projects (`go.mod`).

Go modules use git tags directly — FerrFlow does **not** modify `go.mod`. The version is derived entirely from the git tag (`v1.2.3` or `{name}@v1.2.3`).

## Multiple files per package

A package can have as many `[[package.versioned_files]]` entries as needed:

```toml
[[package.versioned_files]]
path   = "Cargo.toml"
format = "toml"

[[package.versioned_files]]
path   = "npm/package.json"
format = "json"
```

Both files will be updated to the same version before the git commit.
