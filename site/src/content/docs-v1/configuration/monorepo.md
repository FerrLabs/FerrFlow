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
