---
title: Configuration
description: Complete reference for the FerrFlow configuration file.
---

FerrFlow supports six config file formats, searched in this order:

1. `ferrflow.json`
2. `ferrflow.json5`
3. `ferrflow.toml`
4. `ferrflow.ts` (requires `tsx`)
5. `ferrflow.js` (requires `node`)
6. `.ferrflow` (JSON)

If no config file is found, FerrFlow auto-detects common version files in the current directory.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Add <code>&quot;$schema&quot;: &quot;https://ferrflow.com/schema/ferrflow.json&quot;</code> to your JSON config for editor autocompletion and validation.</p>
</div></aside>

## Config formats

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="TypeScript"><p class="ferr-tab__label">TypeScript</p><div class="ferr-tab__body"><pre><code class="language-ts">export default {
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
};
</code></pre>
</div></div>
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

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p>JSON, JSON5, and TypeScript/JavaScript configs use <strong>camelCase</strong> keys (<code>tagTemplate</code>, <code>versionedFiles</code>).
TOML configs use <strong>snake_case</strong> keys (<code>tag_template</code>, <code>versioned_files</code>).
YAML configs support both, but <strong>camelCase</strong> is recommended for consistency with JSON.
All forms are equivalent.</p>
</div></aside>

### TypeScript and JavaScript configs

TypeScript (`.ts`) and JavaScript (`.js`) config files use a default ESM export. The export can be a plain object or an async function.

<aside class="ferr-aside ferr-aside--warning"><div class="ferr-aside__body"><p>TypeScript configs require <code>tsx</code> (<code>npm install -g tsx</code>). JavaScript configs require <code>node</code> (v18+).</p>
</div></aside>

The main advantage of TS/JS configs is **function hooks**. Instead of shell command strings, you can write hooks as native functions with full access to the hook context:

```ts title="ferrflow.ts"
export default {
  workspace: {
    tagTemplate: 'v{version}',
    hooks: {
      postPublish: async (ctx) => {
        await fetch('https://hooks.slack.com/services/...', {
          method: 'POST',
          body: JSON.stringify({
            text: `Released ${ctx.package}@${ctx.newVersion}`,
          }),
        });
      },
    },
  },
  package: [
    {
      name: 'my-app',
      path: '.',
      versionedFiles: [{ path: 'package.json', format: 'json' }],
    },
  ],
};
```

#### Hook context object

Function hooks receive a context object with these fields:

| Field          | Type           | Description                                  |
| -------------- | -------------- | -------------------------------------------- |
| `package`      | string         | Package name                                 |
| `oldVersion`   | string         | Version before bump (empty on first release) |
| `newVersion`   | string         | Version after bump                           |
| `bumpType`     | string         | `major`, `minor`, `patch`, or `none`         |
| `tag`          | string         | Full git tag name                            |
| `dryRun`       | boolean        | Whether `--dry-run` is set                   |
| `packagePath`  | string         | Absolute path to package root                |
| `channel`      | string or null | Pre-release channel name                     |
| `isPrerelease` | boolean        | Whether this is a pre-release                |

Shell string hooks and function hooks can be mixed in the same config. Shell strings still work in TS/JS configs.

## `workspace`

Global settings that apply to all packages.

| Field                   | Type    | Default                                 | Description                                                                                                                                                         |
| ----------------------- | ------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `remote`                | string  | `"origin"`                              | Git remote to push to                                                                                                                                               |
| `branch`                | string  | auto-detected                           | Branch to push to (detected from remote HEAD)                                                                                                                       |
| `tagTemplate`           | string  | `"v{version}"` or `"{name}@v{version}"` | Tag naming pattern. Uses `{version}` and `{name}` placeholders. Defaults to `v{version}` for single-package repos and `{name}@v{version}` for monorepos.            |
| `versioning`            | string  | `"semver"`                              | Default versioning strategy for all packages                                                                                                                        |
| `releaseCommitMode`     | string  | `"commit"`                              | How to handle the release commit: `"commit"`, `"pr"`, or `"none"`                                                                                                   |
| `skipCi`                | boolean | depends on mode                         | Add `[skip ci]` to release commits. Defaults to `true` when mode is `"commit"`, `false` otherwise.                                                                  |
| `autoMergeReleases`     | boolean | `true`                                  | Enable auto-merge on release PRs (only applies when mode is `"pr"`)                                                                                                 |
| `recoverMissedReleases` | boolean | `false`                                 | When enabled, if FerrFlow finds unreleased commits spanning multiple version bumps, it creates all intermediate releases instead of jumping to the latest version   |
| `floatingTags`          | array   | `[]`                                    | Floating tag levels to create on each release: `"major"`, `"minor"`. For example, `["major"]` creates a `v1` tag that always points to the latest `v1.x.y` release. |
| `telemetry`             | boolean | `true`                                  | Send anonymous usage telemetry ([details](/v3/docs/legal/telemetry))                                                                                                |

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

### Floating tags

Floating tags are version aliases that always point to the latest release matching a given level. This is useful for GitHub Actions or Docker images where users reference `v1` instead of `v1.2.3`.

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;floatingTags&quot;: [&quot;major&quot;]
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
floating_tags = [&quot;major&quot;]
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    floatingTags: [&quot;major&quot;],
  },
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  floatingTags:
    - major
</code></pre>
</div></div>
</div>

When releasing `v1.2.3`, FerrFlow creates or moves a `v1` tag pointing to the same commit. With `["major", "minor"]`, both `v1` and `v1.2` tags are maintained.

If a floating tag would move backward (e.g. releasing a `v1.1.0` hotfix when `v1.2.0` already exists), FerrFlow blocks the release. Use `--force` to override this check.

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

| Field          | Required | Default                  | Description                                                                                                       |
| -------------- | -------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| `name`         | yes      | —                        | Package identifier, used in git tag prefix                                                                        |
| `path`         | yes      | —                        | Relative path to the package directory                                                                            |
| `changelog`    | no       | `{path}/CHANGELOG.md`    | Path to the changelog file                                                                                        |
| `sharedPaths`  | no       | `[]`                     | Paths that trigger this package when changed                                                                      |
| `dependsOn`    | no       | `[]`                     | Package names this package depends on. When a dependency is bumped, this package gets a patch bump automatically. |
| `versioning`   | no       | inherited from workspace | Override versioning strategy for this package                                                                     |
| `tagTemplate`  | no       | inherited from workspace | Override tag template for this package                                                                            |
| `floatingTags` | no       | inherited from workspace | Override floating tags for this package                                                                           |

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
| `helm`   | `Chart.yaml`                       | `version` and `appVersion` (when present)         |
| `gomod`  | `go.mod`                           | No file update — version comes from git tags only |
| `txt`    | `VERSION`, `VERSION.txt`           | Entire file content replaced                      |

## `hooks`

Run shell commands at key points in the release lifecycle. Hooks can be defined at workspace level (defaults for all packages) or per package (overrides workspace hooks for that package).

### Lifecycle

```
calculate bump
  ↓
pre_bump        ← validate state, check prerequisites
  ↓
write version files
  ↓
post_bump       ← modify additional files based on new version
  ↓
generate changelog
  ↓
pre_commit      ← review staged changes, run linters
  ↓
git commit + tag
  ↓
pre_publish     ← run tests against tagged commit, build artifacts
  ↓
git push + create release
  ↓
post_publish    ← push Docker images, notify Slack, publish packages
```

### Configuration

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;hooks&quot;: {
      &quot;preBump&quot;: &quot;cargo test&quot;,
      &quot;postBump&quot;: &quot;node scripts/sync-deps.js&quot;,
      &quot;preCommit&quot;: &quot;cargo fmt --check&quot;,
      &quot;prePublish&quot;: &quot;cargo build --release&quot;,
      &quot;postPublish&quot;: &quot;make docker-push &amp;&amp; ./scripts/notify.sh&quot;,
      &quot;onFailure&quot;: &quot;abort&quot;
    }
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[hooks]
pre_bump     = &quot;cargo test&quot;
post_bump    = &quot;node scripts/sync-deps.js&quot;
pre_commit   = &quot;cargo fmt --check&quot;
pre_publish  = &quot;cargo build --release&quot;
post_publish = &quot;make docker-push &amp;&amp; ./scripts/notify.sh&quot;
on_failure   = &quot;abort&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    hooks: {
      preBump: &quot;cargo test&quot;,
      postBump: &quot;node scripts/sync-deps.js&quot;,
      preCommit: &quot;cargo fmt --check&quot;,
      prePublish: &quot;cargo build --release&quot;,
      postPublish: &quot;make docker-push &amp;&amp; ./scripts/notify.sh&quot;,
      onFailure: &quot;abort&quot;,
    },
  },
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  hooks:
    preBump: cargo test
    postBump: node scripts/sync-deps.js
    preCommit: cargo fmt --check
    prePublish: cargo build --release
    postPublish: &quot;make docker-push &amp;&amp; ./scripts/notify.sh&quot;
    onFailure: abort
</code></pre>
</div></div>
</div>

| Field         | Type   | Default   | Description                                                             |
| ------------- | ------ | --------- | ----------------------------------------------------------------------- |
| `preBump`     | string | —         | Run after bump calculation, before writing version files                |
| `postBump`    | string | —         | Run after version files are written                                     |
| `preCommit`   | string | —         | Run after changelog, before git commit                                  |
| `prePublish`  | string | —         | Run after commit+tag, before push                                       |
| `postPublish` | string | —         | Run after push and release creation                                     |
| `onFailure`   | string | `"abort"` | `"abort"` cancels the release on failure, `"continue"` prints a warning |

### Environment variables

Every hook receives these variables:

| Variable                | Description                                  | Example                        |
| ----------------------- | -------------------------------------------- | ------------------------------ |
| `FERRFLOW_PACKAGE`      | Package name                                 | `api`                          |
| `FERRFLOW_OLD_VERSION`  | Version before bump (empty on first release) | `1.2.3`                        |
| `FERRFLOW_NEW_VERSION`  | Version after bump                           | `1.3.0`                        |
| `FERRFLOW_BUMP_TYPE`    | `major`, `minor`, `patch`, or `none`         | `minor`                        |
| `FERRFLOW_TAG`          | Full git tag name                            | `api@v1.3.0`                   |
| `FERRFLOW_DRY_RUN`      | `true` if `--dry-run` is set                 | `false`                        |
| `FERRFLOW_PACKAGE_PATH` | Absolute path to package root                | `/home/user/repo/packages/api` |

### Per-package hooks

Package-level hooks **replace** workspace-level hooks for that package (they are not merged).

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;hooks&quot;: {
      &quot;preBump&quot;: &quot;echo releasing $FERRFLOW_PACKAGE&quot;,
      &quot;postPublish&quot;: &quot;make notify&quot;
    }
  },
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;api&quot;,
      &quot;path&quot;: &quot;packages/api&quot;,
      &quot;hooks&quot;: {
        &quot;preBump&quot;: &quot;cargo test&quot;
      }
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[hooks]
pre_bump     = &quot;echo releasing $FERRFLOW_PACKAGE&quot;
post_publish = &quot;make notify&quot;

[[package]]
name = &quot;api&quot;
path = &quot;packages/api&quot;

[package.hooks]
pre_bump = &quot;cargo test&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    hooks: {
      preBump: &quot;echo releasing $FERRFLOW_PACKAGE&quot;,
      postPublish: &quot;make notify&quot;,
    },
  },
  package: [
    {
      name: &quot;api&quot;,
      path: &quot;packages/api&quot;,
      hooks: {
        preBump: &quot;cargo test&quot;,
      },
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  hooks:
    preBump: &quot;echo releasing $FERRFLOW_PACKAGE&quot;
    postPublish: make notify

package:

- name: api
  path: packages/api
  hooks:
  preBump: cargo test
  </code></pre>

</div></div>
</div>

In this example, the `api` package runs `cargo test` for `preBump` (overriding the workspace echo) but inherits the workspace `postPublish` hook.

### Behavior

- **`--dry-run`**: hooks are printed but not executed.
- **`--verbose`**: hook stdout/stderr is streamed live. Otherwise output is only shown on failure.
- Files modified by `postBump` or `preCommit` hooks are automatically included in the release commit.

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
