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
</div>

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p>JSON, JSON5, and TypeScript/JavaScript configs use <strong>camelCase</strong> keys (<code>tagTemplate</code>, <code>versionedFiles</code>).
TOML configs use <strong>snake_case</strong> keys (<code>tag_template</code>, <code>versioned_files</code>).
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

| Field          | Type           | Description                                                                   |
| -------------- | -------------- | ----------------------------------------------------------------------------- |
| `package`      | string         | Package name                                                                  |
| `oldVersion`   | string         | Version before bump (empty on first release)                                  |
| `newVersion`   | string         | Version after bump                                                            |
| `bumpType`     | string         | `major`, `minor`, `patch`, or `none`                                          |
| `tag`          | string         | Full git tag name                                                             |
| `dryRun`       | boolean        | Whether `--dry-run` is set                                                    |
| `packagePath`  | string         | Absolute path to package root                                                 |
| `channel`      | string or null | Pre-release channel name                                                      |
| `isPrerelease` | boolean        | Whether this is a pre-release                                                 |
| `monorepo`     | boolean        | Whether this is a monorepo release                                            |
| `changelog`    | string         | Rendered changelog section for this bump (markdown)                           |
| `commits`      | array          | `{ hash, message, type?, scope?, breaking }` per commit in the bump           |
| `bumpedFiles`  | array          | `{ path, format }` for each file the release modified                         |
| `allPackages`  | array          | `{ name, version, bump }` for every package released in this batch            |
| `releaseUrl`   | string or null | URL of the created forge release — `postPublish` hooks only, `null` otherwise |

`commits`, `bumpedFiles` and `allPackages` arrive as real arrays (parsed from JSON), so you can iterate them directly:

```js
export default {
  workspace: {
    hooks: {
      postBump(ctx) {
        for (const c of ctx.commits) {
          if (c.breaking) console.log(`breaking: ${c.message}`);
        }
      },
    },
  },
};
```

Shell string hooks and function hooks can be mixed in the same config. Shell strings still work in TS/JS configs.

## `workspace`

Global settings that apply to all packages.

| Field                   | Type    | Default                                                                     | Description                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ----------------------- | ------- | --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `remote`                | string  | `"origin"`                                                                  | Git remote to push to                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `branch`                | string  | auto-detected                                                               | Branch to push to (detected from remote HEAD)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `tagTemplate`           | string  | `"v{version}"` or `"{name}@v{version}"`                                     | Tag naming pattern. Uses `{version}` and `{name}` placeholders. Defaults to `v{version}` for single-package repos and `{name}@v{version}` for monorepos.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `versioning`            | string  | `"semver"`                                                                  | Default versioning strategy for all packages                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `releaseCommitMode`     | string  | `"commit"`                                                                  | How to handle the release commit: `"commit"`, `"pr"`, or `"none"`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `releaseCommitScope`    | string  | `"grouped"`                                                                 | In a monorepo where several packages are bumped at once, whether to create a single `"grouped"` commit or one commit `"per-package"`. Only matters when multiple packages bump.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `releaseCommitBody`     | string  | `"none"`                                                                    | What goes in the body of the release commit, below the subject. `"none"` keeps the single-line subject. `"summary"` lists one line per released package with its commit count. `"full"` embeds the changelog section written for each package — under `"grouped"` scope each section is headed `## <package> <version>`.                                                                                                                                                                                                                                                                                                                                                                                         |
| `forge`                 | string  | `"auto"`                                                                    | Git forge override: `"auto"` detects from the remote URL, and for an unrecognised host it probes the API over HTTPS to auto-detect a self-hosted **GitLab**, **GitHub Enterprise**, or **Gitea / Forgejo** instance (cached, ~2s, best-effort). Set `"github"`, `"gitlab"`, `"gitea"` (Gitea / Forgejo / Codeberg), or `"bitbucket"` (Bitbucket Cloud) to force a forge — needed only when the host isn't reachable over HTTPS or you want to skip probing. Gitea auth uses `GITEA_TOKEN` / `FORGEJO_TOKEN`; Bitbucket uses `BITBUCKET_TOKEN`. All cover release creation — on Bitbucket, which has no release object, the release is the annotated tag FerrFlow pushes. PR mode is GitHub/GitLab only.          |
| `skipCi`                | boolean | depends on mode                                                             | Add `[skip ci]` to release commits. Defaults to `true` when mode is `"commit"`, `false` otherwise.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `commitSkipMarkers`     | array   | `["[skip ci]", "[ci skip]", "[no ci]", "[skip actions]", "[actions skip]"]` | Markers that cause FerrFlow to skip a commit when computing the next version. Matched case-insensitively, subject line only.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `commitFormats`         | object  | permissive conventional                                                     | Which commit subjects map to which bump level. Each of `major` / `minor` / `patch` takes a pattern string, a list of patterns, or `"all"` as a catch-all; `*` matches any run of characters (including `/`) and `?` exactly one. Resolution is major → minor → patch, first match wins. `caseSensitive` (default `true`) lowercases both sides when false. Defaults also accept capitalised and slash-separated variants (`Feat:`, `feat/`, `feature:`, `Fix/`, `Perf:`, `Refactor/`, and so on), listed in full under [permissive defaults](/docs/reference/conventional-commits). Breaking markers (`feat!:`, `fix(api)!:`, a `BREAKING CHANGE:` footer) are always detected regardless of what is configured. |
| `autoMergeReleases`     | boolean | `true`                                                                      | Enable auto-merge on release PRs (only applies when mode is `"pr"`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `recoverMissedReleases` | boolean | `false`                                                                     | Compare versioned files against the last tag instead of just the last commit, recovering releases missed earlier in a monorepo.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `versionSource`         | string  | `"highest"`                                                                 | Which source wins when a package has both a git tag and a version in a versioned file. `"highest"` takes whichever is higher, so a mistake in either source ratchets the version upward and is never walked back. `"tag"` treats the tags as the record of what shipped and ignores the file. `"file"` treats the file as the source and ignores the tag, which is what a package migrated between repos usually needs. No effect when only one source is present.                                                                                                                                                                                                                                               |
| `updateLockfiles`       | boolean | `false`                                                                     | After a bump, refresh the sibling lockfile (`Cargo.lock`, `package-lock.json` / `pnpm-lock.yaml` / `yarn.lock`, `poetry.lock` / `uv.lock`, `Gemfile.lock`, `mix.lock`) via the package manager's offline / lockfile-only mode and stage it in the same release commit. A missing package manager or an unresolvable offline update is warned about, never fatal. Set per-package `updateLockfiles: false` to opt a single package out.                                                                                                                                                                                                                                                                           |
| `updateDependents`      | boolean | `false`                                                                     | After a bump, rewrite the version constraint every dependent declares for the bumped package and stage the manifest in the same release commit. Only `json` (`dependencies`, `devDependencies`, `peerDependencies`, `optionalDependencies`) and `toml` (`dependencies`, `dev-dependencies`, `build-dependencies`) manifests are rewritten, and only plain operator + version constraints — the operator is preserved (`^1.2.3` → `^2.0.0`). A `workspace:*`, `file:`/`git:` spec, `1.x` or a multi-part range is left untouched.                                                                                                                                                                                 |
| `floatingTags`          | array   | `[]`                                                                        | Floating tag levels to create on each release: `"major"`, `"minor"`. For example, `["major"]` creates a `v1` tag that always points to the latest `v1.x.y` release.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `latestTag`             | string  | none                                                                        | Template for a floating alias tag that always points at the package's newest non-prerelease release, e.g. `"latest"` or `"{name}@latest"`. Absent by default. Deliberately **not** derived from `tagTemplate`: the alias is a name, not a version, so a `tagTemplate` of `v{version}` yields `latest`, never `vlatest`. In a monorepo the template must contain `{name}`, otherwise every package overwrites the same ref and the last one released wins. Prereleases never move it, and it is exempt from the backward-movement guard that applies to `major`/`minor` floating tags.                                                                                                                            |
| `orphanedTagStrategy`   | string  | `"warn"`                                                                    | How to handle tags pointing to orphaned commits after rebase + force-push: `"warn"`, `"treeHash"`, or `"message"`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `branches`              | array   | `[]`                                                                        | Map branches to pre-release channels (see [Pre-release channels](#pre-release-channels)).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `linked`                | array   | `[]`                                                                        | Groups of packages that share a version line when co-released. When any member has a releasable commit, all members bump to the same (highest) version (see [Linked and fixed version groups](/docs/configuration/monorepo#linked-and-fixed-version-groups)).                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `fixed`                 | array   | `[]`                                                                        | Groups of packages locked to an identical version forever. Behaves like `linked`; `ferrflow validate` warns when a fixed group's versions have drifted apart.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `anonymous_telemetry`   | boolean | `true`                                                                      | Deprecated and ignored — telemetry was removed in v5.33 ([details](/v5/docs/legal/telemetry)). The key (and its `telemetry` alias) stays accepted so existing configs remain valid.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |

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
</div>

When releasing `v1.2.3`, FerrFlow creates or moves a `v1` tag pointing to the same commit. With `["major", "minor"]`, both `v1` and `v1.2` tags are maintained.

If a floating tag would move backward (e.g. releasing a `v1.1.0` hotfix when `v1.2.0` already exists), FerrFlow blocks the release. Use `--force` to override this check.

### Orphaned tag strategy

When a branch is rebased and force-pushed, existing tags may point to commits that are no longer part of the branch history. By default, FerrFlow warns about these orphaned tags and skips them. You can configure automatic recovery instead.

| Strategy     | Behavior                                                                                            |
| ------------ | --------------------------------------------------------------------------------------------------- |
| `"warn"`     | Log a warning identifying the orphaned tag and skip it (default)                                    |
| `"treeHash"` | Attempt to find a commit on the current branch with the same file tree as the orphaned tag's commit |
| `"message"`  | Attempt to find a commit on the current branch with the same commit message                         |

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;orphanedTagStrategy&quot;: &quot;treeHash&quot;
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
orphaned_tag_strategy = &quot;treeHash&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    orphanedTagStrategy: &quot;treeHash&quot;,
  },
}
</code></pre>
</div></div>
</div>

`"treeHash"` is the safest recovery option — it matches commits that have identical file contents, which is typical after a rebase that doesn't modify files. Use `"message"` when rebases also change the tree (e.g. squashing commits) but preserve the original message.

If recovery fails (no matching commit found within the last 1000 commits), FerrFlow falls back to warning and skipping the tag. In that case, re-tag manually:

```bash
git tag -f api@v1.2.0 <correct-commit>
```

### Release commit mode

Controls how FerrFlow handles the commit that updates version files and changelogs.

| Mode       | Behavior                                                                  |
| ---------- | ------------------------------------------------------------------------- |
| `"commit"` | Commits directly to the current branch and pushes (default)               |
| `"pr"`     | Opens a persistent release pull request and updates it on each new commit |
| `"none"`   | Only creates tags and releases, does not commit file changes              |

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
</div>

In `"pr"` mode FerrFlow keeps **one long-lived release PR per target branch**. It maintains a single release branch — `ferrflow/release-<target-branch>` — and on every new commit it recomputes the version and changelog and force-pushes that same branch, so the open PR updates in place instead of a new PR opening per version.

`autoMergeReleases` (default `true`) enables auto-merge on that PR; it re-applies on each update and is a no-op when disabled (the PR just waits for a human). PR mode is supported on GitHub and GitLab.

FerrFlow won't clobber work you push onto the release branch: if the branch carries a commit it didn't author — anything that isn't a `chore(release):` commit, such as a review fix you pushed — it warns and leaves the branch and PR untouched for that run.

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
</div>

### Pre-release channels

The `branches` array maps branch names (or glob patterns) to pre-release channels. When FerrFlow runs on a branch matching an entry, it releases on that channel — e.g. `1.4.0-beta.1` instead of `1.4.0`. The same mapping is what `--channel` on `ferrflow check` and `ferrflow release` overrides ad-hoc.

Each entry has:

| Field                  | Type              | Description                                                         |
| ---------------------- | ----------------- | ------------------------------------------------------------------- |
| `name`                 | string            | Branch name or glob pattern (e.g. `"main"`, `"release/*"`)          |
| `channel`              | string or `false` | Channel name (`"beta"`, `"rc"`, …), or `false` for a stable release |
| `prereleaseIdentifier` | string            | Strategy for the identifier appended after the channel name         |

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;branches&quot;: [
      { &quot;name&quot;: &quot;main&quot;, &quot;channel&quot;: false },
      { &quot;name&quot;: &quot;next&quot;, &quot;channel&quot;: &quot;beta&quot; }
    ]
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[[workspace.branches]]
name = &quot;main&quot;
channel = false

[[workspace.branches]]
name = &quot;next&quot;
channel = &quot;beta&quot;
</code></pre>

</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    branches: [
      { name: &quot;main&quot;, channel: false },
      { name: &quot;next&quot;, channel: &quot;beta&quot; },
    ],
  },
}
</code></pre>
</div></div>
</div>

### Changelog rendering

Omit `changelog` entirely and FerrFlow writes the classic layout: **Breaking Changes**, **Features**, **Bug Fixes** and **Refactoring**, flat bullets, no links. `perf` commits fold into **Bug Fixes** there, and every other type, `docs` and `security` included, is left out.

Adding the block opts into the richer renderer.

```json title="ferrflow.json"
{
  "workspace": {
    "changelog": {
      "sections": {
        "feat": "Features",
        "fix": "Bug Fixes",
        "perf": "Perf",
        "docs": "Docs",
        "chore": "Chores",
        "ci": "CI",
        "style": false
      },
      "groupByScope": true,
      "includeCommitLinks": true,
      "includeCompareLink": true
    }
  }
}
```

#### `sections`

Maps a commit type to a heading, or to `false` to hide it. Any conventional type works, `chore`, `ci`, `build` and `test` included.

Six types carry built-in labels and render first, in this order:

| Type | Default label |
|---|---|
| `feat` | Features |
| `fix` | Bug Fixes |
| `perf` | Performance |
| `security` | Security |
| `docs` | Documentation |
| `refactor` | Code Refactoring |

Anything else you declare follows, alphabetically, and defaults to the label **Changes** if you pass `true` rather than a string.

Two rules override the type:

- A breaking marker wins. `chore!: drop python 3.8` lands under **Breaking Changes**, never under the `chore` heading.
- `security:` commits and `fix(security):` commits both land under `security`.

Types you do not declare are left out of the changelog.

#### `groupByScope`

Prefixes each bullet with its scope in bold, `- **api:** add events endpoint`. Scopeless commits render first. Defaults to `false`.

#### `includeCommitLinks`

Appends a commit link to each bullet, `([abc1234](<forge>/commit/abc1234))`. Silently omitted when the remote forge cannot be resolved. Defaults to `false`.

#### `includeCompareLink`

Emits a keep-a-changelog compare footer, `[1.2.3]: <forge>/compare/v1.2.2...v1.2.3`, when the forge is known and a previous tag exists. Defaults to `false`.

## `package`

Defines a package to version. You can have one or many.

| Field           | Required | Default                  | Description                                                                                                                                                                                                                                                                   |
| --------------- | -------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `name`          | yes      | —                        | Package identifier, used in git tag prefix                                                                                                                                                                                                                                    |
| `path`          | yes      | —                        | Relative path to the package directory                                                                                                                                                                                                                                        |
| `changelog`     | no       | `{path}/CHANGELOG.md`    | Path to the changelog file                                                                                                                                                                                                                                                    |
| `sharedPaths`   | no       | `[]`                     | Paths that trigger this package when changed                                                                                                                                                                                                                                  |
| `dependsOn`     | no       | `[]`                     | Packages this package depends on. When a dependency is bumped, this package is bumped too — with the same bump type by default. Each entry is a package name, or `{ "name": "core", "propagate": "patch" }` to choose the policy (`same`, `major-on-major`, `patch`, `none`). |
| `versioning`    | no       | inherited from workspace | Override versioning strategy for this package                                                                                                                                                                                                                                 |
| `tagTemplate`   | no       | inherited from workspace | Override tag template for this package                                                                                                                                                                                                                                        |
| `floatingTags`  | no       | inherited from workspace | Override floating tags for this package                                                                                                                                                                                                                                       |
| `latestTag`     | no       | inherited from workspace | Override the floating alias tag for this package                                                                                                                                                                                                                              |
| `versionSource` | no       | inherited from workspace | Override the tag-vs-file resolution for this package                                                                                                                                                                                                                          |

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

### Tag-only packages

`versionedFiles` is optional. Omit it (or set it to `[]`) for packages whose version is communicated entirely through git tags and GitHub Releases — Go modules, Docker images, GitHub Actions, infrastructure repos.

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;my-action&quot;,
      &quot;path&quot;: &quot;.&quot;,
      &quot;versionedFiles&quot;: []
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[[package]]
name = &quot;my-action&quot;
path = &quot;.&quot;
versioned_files = []
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  package: [
    {
      name: &quot;my-action&quot;,
      path: &quot;.&quot;,
      versionedFiles: [],
    },
  ],
}
</code></pre>
</div></div>
</div>

FerrFlow reads the current version from the latest matching git tag, computes the next bump from conventional commits, then creates the tag, the GitHub Release, the changelog and any floating tags — without touching any source file. Hooks still run normally, so you can `docker build`, `docker push`, or `gh release upload` from `postPublish` against `FERRFLOW_NEW_VERSION`.

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p>Before v5.1, packages without <code>versionedFiles</code> were silently skipped. If you depended on that behavior to exclude a package from a release, remove it from the config instead.</p>
</div></aside>

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
generate changelog
  ↓
post_bump       ← modify additional files, or rewrite the changelog just written
  ↓
pre_commit      ← review staged changes, run linters
  ↓
git commit
  ↓
post_commit     ← react to the release commit
  ↓
pre_tag         ← smoke-test the bumped tree before the tag lands
  ↓
git tag
  ↓
post_tag        ← cargo publish before push (recoverable if it fails)
  ↓
pre_publish     ← run tests against tagged commit, build artifacts
  ↓
git push + create release
  ↓
post_publish    ← push Docker images, notify Slack, publish packages

pre_release     ← (PR mode) after the release PR opens, before merge
on_success      ← once, after the whole release completes cleanly
on_error        ← once, when the release fails ($FERRFLOW_ERROR_CODE)
```

### Rewriting the changelog from a hook

`post_bump` runs after the changelog section is generated and written, and receives it in `FERRFLOW_CHANGELOG`. A hook can rewrite `CHANGELOG.md` and FerrFlow picks the change up: the rewritten file is committed, and it is also what reaches the git tag, the forge release body and the release commit.

That is enough to turn commit subjects into prose without FerrFlow needing to know anything about how you do it:

```bash
#!/bin/sh
# Read the generated section from $FERRFLOW_CHANGELOG, write prose back into
# CHANGELOG.md. Any tool works here, including none.
your-rewriter --input "$FERRFLOW_CHANGELOG" --write CHANGELOG.md
```

```json
{ "workspace": { "hooks": { "postBump": "sh ./scripts/prose.sh" } } }
```

Two things worth knowing. If the rewrite loses the `## [version]` heading, FerrFlow falls back to the generated text rather than publishing empty release notes. And nothing here runs under `--dry-run`, where no changelog is written, so preview the result with a real release on a branch rather than expecting `--dry-run` to show it.

Reproducibility is yours to manage. The changelog is committed and tagged, so whatever the hook produces is permanent. A rewriter that gives a different answer each run makes releases that cannot be reproduced.

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
</div>

| Field         | Type   | Default   | Description                                                                             |
| ------------- | ------ | --------- | --------------------------------------------------------------------------------------- |
| `preBump`     | string | —         | Run after bump calculation, before writing version files                                |
| `postBump`    | string | —         | Run after version files are written                                                     |
| `preCommit`   | string | —         | Run after changelog, before git commit                                                  |
| `postCommit`  | string | —         | Run after the release commit, before tagging                                            |
| `preTag`      | string | —         | Run after the commit, immediately before `git tag`                                      |
| `postTag`     | string | —         | Run after tags are created, before push                                                 |
| `prePublish`  | string | —         | Run after commit+tag, before push                                                       |
| `postPublish` | string | —         | Run after push and release creation                                                     |
| `preRelease`  | string | —         | PR mode only: after the release PR opens, before merge (once per run)                   |
| `onSuccess`   | string | —         | Run once after the whole release completes cleanly                                      |
| `onError`     | string | —         | Run once when the release fails; sets `FERRFLOW_ERROR_CODE` (once per run)              |
| `onFailure`   | string | `"abort"` | Strategy — `"abort"` cancels the release on hook failure, `"continue"` prints a warning |

### Environment variables

Every hook receives these variables:

| Variable                     | Description                                                 | Example                                                                |
| ---------------------------- | ----------------------------------------------------------- | ---------------------------------------------------------------------- |
| `FERRFLOW_PACKAGE`           | Package name                                                | `api`                                                                  |
| `FERRFLOW_OLD_VERSION`       | Version before bump (empty on first release)                | `1.2.3`                                                                |
| `FERRFLOW_NEW_VERSION`       | Version after bump                                          | `1.3.0`                                                                |
| `FERRFLOW_BUMP_TYPE`         | `major`, `minor`, `patch`, or `none`                        | `minor`                                                                |
| `FERRFLOW_TAG`               | Full git tag name                                           | `api@v1.3.0`                                                           |
| `FERRFLOW_DRY_RUN`           | `true` if `--dry-run` is set                                | `false`                                                                |
| `FERRFLOW_PACKAGE_PATH`      | Absolute path to package root                               | `/home/user/repo/packages/api`                                         |
| `FERRFLOW_IS_PRERELEASE`     | `true` on a pre-release channel                             | `false`                                                                |
| `FERRFLOW_MONOREPO`          | `true` on a monorepo release                                | `false`                                                                |
| `FERRFLOW_CHANGELOG`         | Rendered changelog section for this bump                    | `### Features\n- ...`                                                  |
| `FERRFLOW_COMMITS_JSON`      | JSON array of `{ hash, message, type?, scope?, breaking }`  | `[{"hash":"a1b2","message":"feat: x","type":"feat","breaking":false}]` |
| `FERRFLOW_BUMPED_FILES_JSON` | JSON array of `{ path, format }` the release modified       | `[{"path":"package.json","format":"json"}]`                            |
| `FERRFLOW_ALL_PACKAGES_JSON` | JSON array of `{ name, version, bump }` released this batch | `[{"name":"api","version":"1.3.0","bump":"minor"}]`                    |
| `FERRFLOW_RELEASE_URL`       | URL of the created forge release (`postPublish` only)       | `https://github.com/acme/api/releases/tag/v1.3.0`                      |
| `FERRFLOW_ERROR_CODE`        | Error code, set only for `onError`                          | `E2005`                                                                |

`FERRFLOW_COMMITS_JSON`, `FERRFLOW_BUMPED_FILES_JSON` and `FERRFLOW_ALL_PACKAGES_JSON` are JSON strings — pipe them through `jq` from shell hooks.

For the once-per-run hooks (`preRelease`, `onSuccess`, `onError`) the per-package variables are empty and `FERRFLOW_TAG` holds every released tag joined by commas.

`onFailure` is the failure **strategy** (`abort` / `continue`), not a command. The command that runs _when_ a release fails is `onError`, which receives the failing `FERRFLOW_ERROR_CODE`.

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
</div>

In this example, the `api` package runs `cargo test` for `preBump` (overriding the workspace echo) but inherits the workspace `postPublish` hook.

### Behavior

- **`--dry-run`**: hooks are printed but not executed.
- **`--verbose`**: hook stdout/stderr is streamed live. Otherwise output is only shown on failure.
- Files modified by `postBump` or `preCommit` hooks are automatically included in the release commit.

## `publishers`

Declarative replacement for the shell-script-in-`postPublish`-hook pattern. Each entry says "after the GitHub Release is created, push this package to that target." Available since v5.4.

Six built-in kinds cover the common publishing targets:

| `kind`                 | What it does                                                                               | Idempotent on                                           |
| ---------------------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------- |
| `cargo`                | `cargo publish` to crates.io or a custom registry                                          | "already uploaded" registry response                    |
| `npm`                  | `npm publish` to npmjs.org, GitHub Packages, or a custom registry                          | "cannot publish over the previously published versions" |
| `docker`               | `docker buildx build --push` with multi-arch + optional Sigstore                           | `docker manifest inspect` on each requested tag         |
| `helm`                 | `helm package` + `helm push` to an OCI registry                                            | `helm show chart` on the new version                    |
| `github-release-asset` | `gh release upload --clobber` of a sidecar file                                            | always re-uploads (clobber semantics)                   |
| `webhook`              | Generic `POST` notifier with `{name}` / `{version}` / `{tag}` / `{env:NAME}` interpolation | none — webhooks are fire-and-forget                     |

All kinds honor `--dry-run` (print the plan, do nothing) and the crash-resume checkpoint (a re-run after a partial failure picks up where it left off).

### Registries

Token credentials are declared once at the workspace level. Each publisher references a registry by name; FerrFlow validates the token env var is exported _before_ invoking the underlying tool.

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;registries&quot;: {
      &quot;kellnr&quot;: {
        &quot;url&quot;: &quot;https://kellnr.example.com&quot;,
        &quot;tokenEnv&quot;: &quot;CARGO_REGISTRIES_KELLNR_TOKEN&quot;
      },
      &quot;gh-packages&quot;: {
        &quot;url&quot;: &quot;https://npm.pkg.github.com&quot;,
        &quot;tokenEnv&quot;: &quot;NODE_AUTH_TOKEN&quot;
      }
    }
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace.registries.kellnr]
url = &quot;https://kellnr.example.com&quot;
token_env = &quot;CARGO_REGISTRIES_KELLNR_TOKEN&quot;

[workspace.registries.gh-packages]
url = &quot;https://npm.pkg.github.com&quot;
token_env = &quot;NODE_AUTH_TOKEN&quot;
</code></pre>

</div></div>
</div>

The token value itself never lives in the config file — only the env-var name does. This keeps `ferrflow.json` checked-in safely.

### Multiple registries per package

`publishers` is a list and every entry runs on its own, so to publish one package to several registries you add one entry per target — each with its own `registry` (and therefore its own credentials), each idempotency-checked against that registry.

```json title="ferrflow.json"
{
  "package": [
    {
      "name": "mylib",
      "path": "crates/mylib",
      "versionedFiles": [{ "path": "Cargo.toml", "format": "toml" }],
      "publishers": [{ "kind": "cargo" }, { "kind": "cargo", "registry": "kellnr" }]
    }
  ]
}
```

This publishes `mylib` to crates.io (public, default token) **and** the private `kellnr` registry (its own `tokenEnv`). The same fan-out works for every kind — `npm` to npmjs + GitHub Packages, `helm` to two OCI registries, `docker` to two `image` targets (one `docker login` per host).

### Cargo publisher

```json title="ferrflow.json"
{
  "package": [
    {
      "name": "ferrlabs-auth",
      "path": "crates/auth",
      "versionedFiles": [{ "path": "Cargo.toml", "format": "toml" }],
      "publishers": [{ "kind": "cargo", "registry": "kellnr" }]
    }
  ]
}
```

Omit `registry` to publish to crates.io. `allowDirty: true` adds `--allow-dirty` (mirrors `cargo publish`'s own flag). `noVerify: true` adds `--no-verify` — set it for multi-crate batch releases so inter-dependent crates don't fail on registry-index propagation timing.

### npm publisher

```json title="ferrflow.json"
{
  "package": [
    {
      "name": "@ferrlabs/ui-react",
      "path": "packages/react",
      "publishers": [
        { "kind": "npm", "registry": "gh-packages", "tag": "next", "access": "public" }
      ]
    }
  ]
}
```

Omit `registry` to publish to npmjs.org. `tag` defaults to `"latest"`. A scoped `.npmrc` is written into the package dir for the duration of the publish and removed on exit — your project's `.npmrc` is never modified.

### Docker publisher

```json title="ferrflow.json"
{
  "package": [
    {
      "name": "ferrlabs-auth-api",
      "path": "crates/auth-api",
      "publishers": [
        {
          "kind": "docker",
          "image": "ghcr.io/ferrlabs/auth-api",
          "tags": ["{version}", "{major}", "{minor}", "latest"],
          "platforms": ["linux/amd64", "linux/arm64"],
          "sign": "sigstore"
        }
      ]
    }
  ]
}
```

`{version}`, `{major}`, `{minor}` expand from the release version. `sign: "sigstore"` runs `cosign sign --yes <image>@<digest>` against the produced manifest after push. Auth assumes you've already run `docker login` (FerrFlow doesn't manage docker credentials so it doesn't conflict with the rest of the CI).

### Helm publisher

```json title="ferrflow.json"
{
  "package": [
    {
      "name": "ferrvault-operator",
      "path": "operator",
      "publishers": [
        {
          "kind": "helm",
          "chart": "chart",
          "registry": "oci://ghcr.io/ferrlabs/charts"
        }
      ]
    }
  ]
}
```

`chart` is the directory containing `Chart.yaml`, relative to the package path. Auth assumes `helm registry login` was done in CI.

### PyPI publisher

```json title="ferrflow.json"
{
  "package": [
    {
      "name": "ferrlabs-sdk",
      "path": "sdk",
      "publishers": [{ "kind": "pypi" }]
    }
  ]
}
```

Runs `python -m build` in the package directory, then `twine upload dist/*`. Set `build` to `false` when the distributions are produced earlier in the pipeline. Omit `registry` to publish to pypi.org.

#### Trusted publishing

```json title="ferrflow.json"
{
  "publishers": [{ "kind": "pypi", "trustedPublishing": true }]
}
```

Mints a short-lived upload token from PyPI instead of reading a stored one, so no PyPI secret lives in the repository. The job has to grant the OIDC permission, and the repository and workflow have to be registered as a trusted publisher for the project:

```yaml title=".github/workflows/release.yml"
permissions:
  contents: write
  id-token: write
```

The token endpoint is derived from the registry url, so TestPyPI and a private index work unchanged. A non-https registry url is refused: the exchange must not happen in the clear.

`trustedPublishing` and a registry `tokenEnv` are mutually exclusive. Setting both is a configuration error, reported before anything is published, rather than a silent choice between two credentials.

### GitHub release asset

```json title="ferrflow.json"
{
  "publishers": [
    { "kind": "github-release-asset", "path": "sbom.cdx.json" },
    {
      "kind": "github-release-asset",
      "path": "ferrflow-linux-x64.tar.gz.bundle",
      "displayName": "linux-x64.bundle"
    }
  ]
}
```

Re-uploads use `--clobber` so a retry replaces the previous file rather than failing. Picks up `GITHUB_TOKEN` from the environment, same as `gh release upload`.

### Webhook

```json title="ferrflow.json"
{
  "publishers": [
    {
      "kind": "webhook",
      "url": "https://hooks.slack.com/services/...",
      "body": { "text": "Released {name}@{version} :rocket:" },
      "headers": { "Authorization": "Bearer {env:SLACK_TOKEN}" }
    }
  ]
}
```

`{name}`, `{version}`, `{tag}` and `{env:NAME}` are interpolated in the URL, body, and header values. Missing `{env:NAME}` is an error (a webhook with an unset bearer token must NOT send anonymously).

### Extra flags (`args`)

Every command-based publisher — `cargo`, `npm`, `docker`, `helm`, `github-release-asset` — accepts an `args` array. The strings are appended verbatim to the underlying tool invocation, so you can pass options FerrFlow doesn't model natively without waiting for a new field.

```json title="ferrflow.json"
{
  "publishers": [
    { "kind": "cargo", "registry": "kellnr", "args": ["--locked"] },
    { "kind": "npm", "args": ["--provenance"] },
    {
      "kind": "docker",
      "image": "ghcr.io/ferrlabs/api",
      "args": ["--build-arg", "PROFILE=release"]
    }
  ]
}
```

For the docker publisher, `args` are inserted before the build-context positional (buildx rejects flags that follow the context argument). The `webhook` publisher has no `args` — it isn't a shelled-out command.

### Deferring publishing to a separate job (`deferPublish`)

By default publishers run inline at the end of `ferrflow release`. That's fine when the release job already has what the publishers need — but `docker`, `helm`, and `npm` publishers need a build toolchain (buildx, helm, a built `dist/`) that a minimal release job often doesn't carry.

Set `workspace.deferPublish: true` and `ferrflow release` will **skip** the publishers; a separate [`ferrflow publish`](/docs/reference/cli/#ferrflow-publish) run executes them. You keep **one** config file:

```json title="ferrflow.json"
{
  "workspace": { "deferPublish": true },
  "package": [
    {
      "name": "my-operator",
      "path": ".",
      "versionedFiles": [{ "path": "go.mod", "format": "gomod" }],
      "publishers": [
        { "kind": "docker", "image": "ghcr.io/acme/my-operator", "tags": ["{version}", "latest"] },
        { "kind": "helm", "chart": "charts/my-operator", "registry": "oci://ghcr.io/acme/charts" }
      ]
    }
  ]
}
```

`release` versions + tags as usual; a tag-triggered job that sets up the toolchain runs `ferrflow publish` (which always runs the publishers, ignoring `deferPublish`). See the [`ferrflow publish`](/docs/reference/cli/#ferrflow-publish) reference for the matching workflow.

### Migrating from postPublish hooks

The publishers block is additive — your existing `postPublish` hooks keep working. Recommended order:

1. Add a `publishers` block alongside your hook with `--dry-run` enabled in CI to preview what would be published.
2. Once the dry-run output matches what your hook does, switch CI to a real `ferrflow release` and verify the live publish.
3. Delete the `postPublish` hook.

The crash-resume checkpoint (see [pipeline triggers](/docs/ci/pipeline-triggers#crash-resume)) means a partially-failed publish is safely re-runnable — the publishers that already succeeded skip themselves on the second pass.

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
</div>

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Run <code>ferrflow init</code> to generate a config file automatically based on what FerrFlow detects in your repo.</p>
</div></aside>
