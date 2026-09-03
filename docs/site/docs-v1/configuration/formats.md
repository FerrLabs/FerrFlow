---
title: Supported formats
description: Version file formats that FerrFlow can read and update.
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

A package can have as many versioned file entries as needed:

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;package&quot;: {
    &quot;versionedFiles&quot;: [
      { &quot;path&quot;: &quot;Cargo.toml&quot;, &quot;format&quot;: &quot;toml&quot; },
      { &quot;path&quot;: &quot;npm/package.json&quot;, &quot;format&quot;: &quot;json&quot; }
    ]
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[[package.versioned_files]]
path   = &quot;Cargo.toml&quot;
format = &quot;toml&quot;

[[package.versioned_files]]
path = &quot;npm/package.json&quot;
format = &quot;json&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  package: {
    versionedFiles: [
      { path: &quot;Cargo.toml&quot;, format: &quot;toml&quot; },
      { path: &quot;npm/package.json&quot;, format: &quot;json&quot; },
    ],
  },
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">package:
  versionedFiles:
    - path: Cargo.toml
      format: toml
    - path: npm/package.json
      format: json
</code></pre>
</div></div>
</div>

Both files will be updated to the same version before the git commit.
