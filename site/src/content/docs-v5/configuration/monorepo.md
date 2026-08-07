---
title: Monorepo
description: Version multiple packages independently in a single repository.
---

FerrFlow treats a repository as a monorepo when the config defines more than one package. Each package is versioned independently based on its own git history.

## Package isolation

FerrFlow uses path prefixes to determine which commits belong to which package. Only commits that touch files under `path` (or `sharedPaths`) trigger a release for that package.

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;api&quot;,
      &quot;path&quot;: &quot;packages/api&quot;
    },
    {
      &quot;name&quot;: &quot;site&quot;,
      &quot;path&quot;: &quot;packages/site&quot;
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[[package]]
name = &quot;api&quot;
path = &quot;packages/api&quot;

[[package]]
name = &quot;site&quot;
path = &quot;packages/site&quot;
</code></pre>

</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  package: [
    {
      name: &quot;api&quot;,
      path: &quot;packages/api&quot;,
    },
    {
      name: &quot;site&quot;,
      path: &quot;packages/site&quot;,
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">package:
  - name: api
    path: packages/api
  - name: site
    path: packages/site
</code></pre>
</div></div>
</div>

## Shared dependencies

If you have code shared between packages (e.g., a `packages/shared/` library), declare it as a `sharedPaths` entry. A change to any shared path triggers a release for every package that lists it:

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;api&quot;,
      &quot;path&quot;: &quot;packages/api&quot;,
      &quot;sharedPaths&quot;: [&quot;packages/shared/&quot;]
    },
    {
      &quot;name&quot;: &quot;site&quot;,
      &quot;path&quot;: &quot;packages/site&quot;,
      &quot;sharedPaths&quot;: [&quot;packages/shared/&quot;]
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[[package]]
name = &quot;api&quot;
path = &quot;packages/api&quot;
shared_paths = [&quot;packages/shared/&quot;]

[[package]]
name = &quot;site&quot;
path = &quot;packages/site&quot;
shared_paths = [&quot;packages/shared/&quot;]
</code></pre>

</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  package: [
    {
      name: &quot;api&quot;,
      path: &quot;packages/api&quot;,
      sharedPaths: [&quot;packages/shared/&quot;],
    },
    {
      name: &quot;site&quot;,
      path: &quot;packages/site&quot;,
      sharedPaths: [&quot;packages/shared/&quot;],
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">package:
  - name: api
    path: packages/api
    sharedPaths:
      - packages/shared/
  - name: site
    path: packages/site
    sharedPaths:
      - packages/shared/
</code></pre>
</div></div>
</div>

## Package dependencies

Use `dependsOn` to declare that a package depends on another. When a dependency is released, the dependent package is bumped too — even if none of its own files changed — with the same bump type by default. This cascades transitively: if `app` depends on `cli` and `cli` depends on `core`, bumping `core` bumps both `cli` and `app`.

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;core&quot;,
      &quot;path&quot;: &quot;packages/core&quot;
    },
    {
      &quot;name&quot;: &quot;cli&quot;,
      &quot;path&quot;: &quot;packages/cli&quot;,
      &quot;dependsOn&quot;: [&quot;core&quot;]
    },
    {
      &quot;name&quot;: &quot;app&quot;,
      &quot;path&quot;: &quot;packages/app&quot;,
      &quot;dependsOn&quot;: [&quot;cli&quot;]
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[[package]]
name = &quot;core&quot;
path = &quot;packages/core&quot;

[[package]]
name = &quot;cli&quot;
path = &quot;packages/cli&quot;
depends_on = [&quot;core&quot;]

[[package]]
name = &quot;app&quot;
path = &quot;packages/app&quot;
depends_on = [&quot;cli&quot;]
</code></pre>

</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  package: [
    {
      name: &quot;core&quot;,
      path: &quot;packages/core&quot;,
    },
    {
      name: &quot;cli&quot;,
      path: &quot;packages/cli&quot;,
      dependsOn: [&quot;core&quot;],
    },
    {
      name: &quot;app&quot;,
      path: &quot;packages/app&quot;,
      dependsOn: [&quot;cli&quot;],
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">package:
  - name: core
    path: packages/core
  - name: cli
    path: packages/cli
    dependsOn:
      - core
  - name: app
    path: packages/app
    dependsOn:
      - cli
</code></pre>
</div></div>
</div>

### Propagation policy

Write an entry as an object to choose how the upstream bump translates:

```json
{
  "name": "cli",
  "path": "packages/cli",
  "dependsOn": [{ "name": "core", "propagate": "patch" }]
}
```

| Policy           | A major in the dependency becomes     | A minor becomes | A patch becomes |
| ---------------- | ------------------------------------- | --------------- | --------------- |
| `same` (default) | major                                 | minor           | patch           |
| `major-on-major` | major                                 | patch           | patch           |
| `patch`          | patch                                 | patch           | patch           |
| `none`           | nothing — the dependent is not bumped | nothing         | nothing         |

A bare string is shorthand for `same`, so `"dependsOn": ["core"]` and `{ "name": "core", "propagate": "same" }` are identical. When several dependencies move at once under different policies, the strongest resulting bump wins.

### Updating dependents' manifests

Set `workspace.updateDependents` to `true` to rewrite the version constraint each dependent declares for the bumped package, staged in the same release commit:

```
● core  1.0.0 → 1.1.0  (minor)
● cli   2.3.0 → 2.4.0  (minor, dependency: core)
  ↳ core → 1.1.0 in cli/package.json
```

`cli/package.json` goes from `"core": "^1.0.0"` to `"core": "^1.1.0"`. Only `json` and `toml` manifests are rewritten, and only plain operator + version constraints — the operator is preserved. A `workspace:*`, `file:`/`git:` spec, `1.x` or a multi-part range carries intent a version pin would destroy, so it is left for you to handle.

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p><code>dependsOn</code> differs from <code>sharedPaths</code>. Shared paths trigger a bump when files in the shared directory change. <code>dependsOn</code> triggers a bump when another <strong>package</strong> is released, regardless of which files changed.</p>
</div></aside>

### Dependency cycles

`dependsOn` must describe a directed acyclic graph. If two packages depend on each other — directly or transitively — there is no order in which to release them, so FerrFlow stops with error `E8003` and names the loop:

```
cycle detected: api → web → api
```

The check runs before any version is written, so a cyclic configuration never produces a partial release. Break the loop by removing one of the `dependsOn` edges. Otherwise the graph is released dependencies-first: a package is always released after the packages it depends on.

## Linked and fixed version groups

Sometimes packages must share a version number, not just cascade a bump. `linked` and `fixed` list groups of packages that move in lockstep:

```toml
[workspace]
linked = [["react", "react-dom"]]
fixed  = [["@scope/a", "@scope/b", "@scope/c"]]
```

When **any** member of a group has a releasable commit, every member is bumped to the same version — the highest version the group would reach. A `feat` on one member and a `fix` on another release the whole group on the minor. Package names stay distinct; only the version is shared, and members with no commits of their own are pulled into the release at the shared version.

- **`linked`** — packages share a version line when they are released together (e.g. `react` and `react-dom` both go `1.2.3 → 1.2.4`).
- **`fixed`** — packages are locked to an identical version forever. It behaves like `linked`, and `ferrflow validate` additionally warns when a fixed group's versions have already drifted apart, so you catch a manual edit before the next release realigns them.

Each group must list at least two packages, and a package may appear in only one `linked` or `fixed` group. Naming a package that isn't defined in `package[]`, or listing one in two groups, stops the release with a clear error before anything is written — the same pre-flight guarantee as [dependency cycles](#dependency-cycles).

`linked`/`fixed` and `dependsOn` compose: a package that depends on a grouped package still receives its cascade bump after the group is aligned.

## Git tag format

By default, monorepo tags use the `{name}@v{version}` format:

```
api@v1.2.0
site@v0.4.1
```

Configure this with the `tagTemplate` field:

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

For a single-package repo, the default is `v{version}` (no name prefix).

FerrFlow looks for the most recent tag matching the template to determine what commits are new.

## Independent cadences

Packages release independently. In a single `ferrflow release` run:

- `api` may bump from `1.2.0` → `1.3.0` (new `feat:` commit)
- `site` may bump from `0.4.0` → `0.4.1` (only `fix:` commits)
- `shared` may not release at all (only `chore:` commits)

## Per-package overrides

Each package can override the workspace-level `versioning` strategy and `tagTemplate`:

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;versioning&quot;: &quot;semver&quot;,
    &quot;tagTemplate&quot;: &quot;{name}@v{version}&quot;
  },
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;api&quot;,
      &quot;path&quot;: &quot;packages/api&quot;,
      &quot;versioning&quot;: &quot;calver&quot;
    },
    {
      &quot;name&quot;: &quot;site&quot;,
      &quot;path&quot;: &quot;packages/site&quot;,
      &quot;tagTemplate&quot;: &quot;site-v{version}&quot;
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
versioning = &quot;semver&quot;
tag_template = &quot;{name}@v{version}&quot;

[[package]]
name = &quot;api&quot;
path = &quot;packages/api&quot;
versioning = &quot;calver&quot;

[[package]]
name = &quot;site&quot;
path = &quot;packages/site&quot;
tag_template = &quot;site-v{version}&quot;
</code></pre>

</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    versioning: &quot;semver&quot;,
    tagTemplate: &quot;{name}@v{version}&quot;,
  },
  package: [
    {
      name: &quot;api&quot;,
      path: &quot;packages/api&quot;,
      versioning: &quot;calver&quot;,
    },
    {
      name: &quot;site&quot;,
      path: &quot;packages/site&quot;,
      tagTemplate: &quot;site-v{version}&quot;,
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  versioning: semver
  tagTemplate: &quot;{name}@v{version}&quot;

package:

- name: api
  path: packages/api
  versioning: calver
- name: site
  path: packages/site
  tagTemplate: &quot;site-v{version}&quot;
  </code></pre>

</div></div>
</div>

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Use <code>ferrflow check</code> to preview exactly which packages would be released and at what version before committing to a release.</p>
</div></aside>
