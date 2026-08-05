---
title: Configuration
description: Complete reference for the FerrFlow configuration file.
---

FerrFlow supports four config file formats, searched in this order:

1. `ferrflow.json`
2. `ferrflow.json5`
3. `ferrflow.toml`
4. `.ferrflow` (JSON)

If no config file is found, FerrFlow auto-detects common version files in the current directory.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Add <code>&quot;$schema&quot;: &quot;https://ferrflow.com/schema/ferrflow.json&quot;</code> to your JSON config for editor autocompletion and validation.</p>
</div></aside>

## Config formats

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;$schema&quot;: &quot;https://ferrflow.com/schema/ferrflow.json&quot;,
  &quot;workspace&quot;: {
    &quot;tagTemplate&quot;: &quot;v{version}&quot;
  },
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;my-app&quot;,
      &quot;path&quot;: &quot;.&quot;,
      &quot;changelog&quot;: &quot;CHANGELOG.md&quot;,
      &quot;versionedFiles&quot;: [
        { &quot;path&quot;: &quot;Cargo.toml&quot;, &quot;format&quot;: &quot;toml&quot; }
      ]
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
tag_template = &quot;v{version}&quot;

[[package]]
name = &quot;my-app&quot;
path = &quot;.&quot;
changelog = &quot;CHANGELOG.md&quot;

[[package.versioned_files]]
path = &quot;Cargo.toml&quot;
format = &quot;toml&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  $schema: &quot;https://ferrflow.com/schema/ferrflow.json&quot;,
  workspace: {
    tagTemplate: &quot;v{version}&quot;,
  },
  package: [
    {
      name: &quot;my-app&quot;,
      path: &quot;.&quot;,
      changelog: &quot;CHANGELOG.md&quot;,
      versionedFiles: [
        { path: &quot;Cargo.toml&quot;, format: &quot;toml&quot; },
      ],
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  tagTemplate: &quot;v{version}&quot;

package:

- name: my-app
  path: &quot;.&quot;
  changelog: CHANGELOG.md
  versionedFiles:
  - path: Cargo.toml
    format: toml
    </code></pre>

</div></div>
</div>

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p>JSON and JSON5 configs use <strong>camelCase</strong> keys (<code>tagTemplate</code>, <code>versionedFiles</code>).
TOML configs use <strong>snake_case</strong> keys (<code>tag_template</code>, <code>versioned_files</code>).
YAML configs support both, but <strong>camelCase</strong> is recommended for consistency with JSON.
All forms are equivalent.</p>
</div></aside>

## `workspace`

Global settings that apply to all packages.

| Field                   | Type    | Default                                 | Description                                                                                                                                                       |
| ----------------------- | ------- | --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `remote`                | string  | `"origin"`                              | Git remote to push to                                                                                                                                             |
| `branch`                | string  | auto-detected                           | Branch to push to (detected from remote HEAD)                                                                                                                     |
| `tagTemplate`           | string  | `"v{version}"` or `"{name}@v{version}"` | Tag naming pattern. Uses `{version}` and `{name}` placeholders. Defaults to `v{version}` for single-package repos and `{name}@v{version}` for monorepos.          |
| `versioning`            | string  | `"semver"`                              | Default versioning strategy for all packages                                                                                                                      |
| `releaseCommitMode`     | string  | `"commit"`                              | How to handle the release commit: `"commit"`, `"pr"`, or `"none"`                                                                                                 |
| `skipCi`                | boolean | depends on mode                         | Add `[skip ci]` to release commits. Defaults to `true` when mode is `"commit"`, `false` otherwise.                                                                |
| `autoMergeReleases`     | boolean | `true`                                  | Enable auto-merge on release PRs (only applies when mode is `"pr"`)                                                                                               |
| `recoverMissedReleases` | boolean | `false`                                 | When enabled, if FerrFlow finds unreleased commits spanning multiple version bumps, it creates all intermediate releases instead of jumping to the latest version |
| `telemetry`             | boolean | `true`                                  | Send anonymous usage telemetry                                                                                                                                    |

### Tag template

The `tagTemplate` field controls how git tags are named. Available placeholders:

| Placeholder | Description                       |
| ----------- | --------------------------------- |
| `{version}` | The version number (e.g. `1.2.3`) |
| `{name}`    | The package name                  |

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;tagTemplate&quot;: &quot;v{version}&quot;
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
tag_template = &quot;v{version}&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    tagTemplate: &quot;v{version}&quot;,
  },
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  tagTemplate: &quot;v{version}&quot;
</code></pre>
</div></div>
</div>

For monorepos, use `{name}` to namespace tags per package:

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;tagTemplate&quot;: &quot;{name}@v{version}&quot;
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
tag_template = &quot;{name}@v{version}&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    tagTemplate: &quot;{name}@v{version}&quot;,
  },
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  tagTemplate: &quot;{name}@v{version}&quot;
</code></pre>
</div></div>
</div>

### Release commit mode

Controls how FerrFlow handles the commit that updates version files and changelogs.

| Mode       | Behavior                                                     |
| ---------- | ------------------------------------------------------------ |
| `"commit"` | Commits directly to the current branch and pushes (default)  |
| `"pr"`     | Creates a `release/` branch and opens a pull request         |
| `"none"`   | Only creates tags and releases, does not commit file changes |

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;releaseCommitMode&quot;: &quot;pr&quot;,
    &quot;autoMergeReleases&quot;: true
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
release_commit_mode = &quot;pr&quot;
auto_merge_releases = true
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    releaseCommitMode: &quot;pr&quot;,
    autoMergeReleases: true,
  },
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  releaseCommitMode: pr
  autoMergeReleases: true
</code></pre>
</div></div>
</div>

### Versioning strategies

FerrFlow supports multiple versioning strategies, configurable at workspace or package level.

| Strategy       | Format              | Example progression                     |
| -------------- | ------------------- | --------------------------------------- |
| `semver`       | `MAJOR.MINOR.PATCH` | `1.2.3` → `1.3.0` → `2.0.0`             |
| `calver`       | `YYYY.MM.PATCH`     | `2026.03.0` → `2026.03.1` → `2026.04.0` |
| `calver-short` | `YY.MM.PATCH`       | `26.03.0` → `26.03.1`                   |
| `calver-seq`   | `YYYY.MM.SEQ`       | `2026.03.1` → `2026.03.2`               |
| `sequential`   | `N`                 | `1` → `2` → `3`                         |
| `zerover`      | `0.MINOR.PATCH`     | `0.1.0` → `0.2.0` (never reaches 1.0)   |

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;versioning&quot;: &quot;calver&quot;
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
versioning = &quot;calver&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    versioning: &quot;calver&quot;,
  },
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  versioning: calver
</code></pre>
</div></div>
</div>

## `package`

Defines a package to version. You can have one or many.

| Field         | Required | Default                  | Description                                   |
| ------------- | -------- | ------------------------ | --------------------------------------------- |
| `name`        | yes      | —                        | Package identifier, used in git tag prefix    |
| `path`        | yes      | —                        | Relative path to the package directory        |
| `changelog`   | no       | `{path}/CHANGELOG.md`    | Path to the changelog file                    |
| `sharedPaths` | no       | `[]`                     | Paths that trigger this package when changed  |
| `versioning`  | no       | inherited from workspace | Override versioning strategy for this package |
| `tagTemplate` | no       | inherited from workspace | Override tag template for this package        |

### `versionedFiles`

Files where the version number should be updated.

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;my-app&quot;,
      &quot;path&quot;: &quot;.&quot;,
      &quot;versionedFiles&quot;: [
        { &quot;path&quot;: &quot;Cargo.toml&quot;, &quot;format&quot;: &quot;toml&quot; },
        { &quot;path&quot;: &quot;npm/package.json&quot;, &quot;format&quot;: &quot;json&quot; }
      ]
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[[package]]
name = &quot;my-app&quot;
path = &quot;.&quot;

[[package.versioned_files]]
path = &quot;Cargo.toml&quot;
format = &quot;toml&quot;

[[package.versioned_files]]
path = &quot;npm/package.json&quot;
format = &quot;json&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  package: [
    {
      name: &quot;my-app&quot;,
      path: &quot;.&quot;,
      versionedFiles: [
        { path: &quot;Cargo.toml&quot;, format: &quot;toml&quot; },
        { path: &quot;npm/package.json&quot;, format: &quot;json&quot; },
      ],
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">package:
  - name: my-app
    path: &quot;.&quot;
    versionedFiles:
      - path: Cargo.toml
        format: toml
      - path: npm/package.json
        format: json
</code></pre>
</div></div>
</div>

| `format` | File                               | Field updated                                     |
| -------- | ---------------------------------- | ------------------------------------------------- |
| `toml`   | `Cargo.toml`, `pyproject.toml`     | `[package].version` or `[project].version`        |
| `json`   | `package.json`                     | `version`                                         |
| `xml`    | `pom.xml`                          | First `<version>` element                         |
| `gradle` | `build.gradle`, `build.gradle.kts` | `version = "..."`                                 |
| `gomod`  | `go.mod`                           | No file update — version comes from git tags only |
| `txt`    | `VERSION`, `VERSION.txt`           | Entire file content replaced                      |

## Complete examples

### Single repo

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;$schema&quot;: &quot;https://ferrflow.com/schema/ferrflow.json&quot;,
  &quot;workspace&quot;: {
    &quot;tagTemplate&quot;: &quot;v{version}&quot;
  },
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;ferrflow&quot;,
      &quot;path&quot;: &quot;.&quot;,
      &quot;changelog&quot;: &quot;CHANGELOG.md&quot;,
      &quot;versionedFiles&quot;: [
        { &quot;path&quot;: &quot;Cargo.toml&quot;, &quot;format&quot;: &quot;toml&quot; },
        { &quot;path&quot;: &quot;npm/package.json&quot;, &quot;format&quot;: &quot;json&quot; }
      ]
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
tag_template = &quot;v{version}&quot;

[[package]]
name = &quot;ferrflow&quot;
path = &quot;.&quot;
changelog = &quot;CHANGELOG.md&quot;

[[package.versioned_files]]
path = &quot;Cargo.toml&quot;
format = &quot;toml&quot;

[[package.versioned_files]]
path = &quot;npm/package.json&quot;
format = &quot;json&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  $schema: &quot;https://ferrflow.com/schema/ferrflow.json&quot;,
  workspace: {
    tagTemplate: &quot;v{version}&quot;,
  },
  package: [
    {
      name: &quot;ferrflow&quot;,
      path: &quot;.&quot;,
      changelog: &quot;CHANGELOG.md&quot;,
      versionedFiles: [
        { path: &quot;Cargo.toml&quot;, format: &quot;toml&quot; },
        { path: &quot;npm/package.json&quot;, format: &quot;json&quot; },
      ],
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  tagTemplate: &quot;v{version}&quot;

package:

- name: ferrflow
  path: &quot;.&quot;
  changelog: CHANGELOG.md
  versionedFiles:
  - path: Cargo.toml
    format: toml
  - path: npm/package.json
    format: json
    </code></pre>

</div></div>
</div>

### Monorepo

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;$schema&quot;: &quot;https://ferrflow.com/schema/ferrflow.json&quot;,
  &quot;workspace&quot;: {
    &quot;tagTemplate&quot;: &quot;{name}@v{version}&quot;
  },
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;api&quot;,
      &quot;path&quot;: &quot;packages/api&quot;,
      &quot;changelog&quot;: &quot;packages/api/CHANGELOG.md&quot;,
      &quot;sharedPaths&quot;: [&quot;packages/shared/&quot;],
      &quot;versionedFiles&quot;: [
        { &quot;path&quot;: &quot;packages/api/Cargo.toml&quot;, &quot;format&quot;: &quot;toml&quot; }
      ]
    },
    {
      &quot;name&quot;: &quot;site&quot;,
      &quot;path&quot;: &quot;packages/site&quot;,
      &quot;changelog&quot;: &quot;packages/site/CHANGELOG.md&quot;,
      &quot;sharedPaths&quot;: [&quot;packages/shared/&quot;],
      &quot;versionedFiles&quot;: [
        { &quot;path&quot;: &quot;packages/site/package.json&quot;, &quot;format&quot;: &quot;json&quot; }
      ]
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
tag_template = &quot;{name}@v{version}&quot;

[[package]]
name = &quot;api&quot;
path = &quot;packages/api&quot;
changelog = &quot;packages/api/CHANGELOG.md&quot;
shared_paths = [&quot;packages/shared/&quot;]

[[package.versioned_files]]
path = &quot;packages/api/Cargo.toml&quot;
format = &quot;toml&quot;

[[package]]
name = &quot;site&quot;
path = &quot;packages/site&quot;
changelog = &quot;packages/site/CHANGELOG.md&quot;
shared_paths = [&quot;packages/shared/&quot;]

[[package.versioned_files]]
path = &quot;packages/site/package.json&quot;
format = &quot;json&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  $schema: &quot;https://ferrflow.com/schema/ferrflow.json&quot;,
  workspace: {
    tagTemplate: &quot;{name}@v{version}&quot;,
  },
  package: [
    {
      name: &quot;api&quot;,
      path: &quot;packages/api&quot;,
      changelog: &quot;packages/api/CHANGELOG.md&quot;,
      sharedPaths: [&quot;packages/shared/&quot;],
      versionedFiles: [
        { path: &quot;packages/api/Cargo.toml&quot;, format: &quot;toml&quot; },
      ],
    },
    {
      name: &quot;site&quot;,
      path: &quot;packages/site&quot;,
      changelog: &quot;packages/site/CHANGELOG.md&quot;,
      sharedPaths: [&quot;packages/shared/&quot;],
      versionedFiles: [
        { path: &quot;packages/site/package.json&quot;, format: &quot;json&quot; },
      ],
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  tagTemplate: &quot;{name}@v{version}&quot;

package:

- name: api
  path: packages/api
  changelog: packages/api/CHANGELOG.md
  sharedPaths:
  - packages/shared/
    versionedFiles:
  - path: packages/api/Cargo.toml
    format: toml

- name: site
  path: packages/site
  changelog: packages/site/CHANGELOG.md
  sharedPaths:
  - packages/shared/
    versionedFiles:
  - path: packages/site/package.json
    format: json
    </code></pre>

</div></div>
</div>

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Run <code>ferrflow init</code> to generate a config file automatically based on what FerrFlow detects in your repo.</p>
</div></aside>
