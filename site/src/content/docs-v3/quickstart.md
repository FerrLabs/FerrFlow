---
title: Quick start
description: Go from zero to your first automated release in under 5 minutes.
---

<ol>
<li><p><strong>Scaffold the config</strong></p>
<p>Run <code>ferrflow init</code> at the root of your repository. It detects your version files and writes a <code>.ferrflow</code> config:</p>
<pre><code class="language-bash">ferrflow init
</code></pre>
<p>For a Rust project this produces:</p>
<pre><code class="language-json">{
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
</li>
<li><p><strong>Preview what would happen</strong></p>
<p>Before touching anything, run a dry-run to see what FerrFlow would do:</p>
<pre><code class="language-bash">ferrflow check
</code></pre>
<p>Output:</p>
<pre><code>Scanning . ...
→ feat: add user authentication
→ fix: correct pagination offset

Bump my-app 0.1.0 → 0.2.0
Tag v0.2.0
</code></pre>
</li>
<li><p><strong>Run the release</strong></p>
<pre><code class="language-bash">ferrflow release
</code></pre>
<p>FerrFlow will:</p>
<ul>
<li>Update <code>Cargo.toml</code> to <code>0.2.0</code></li>
<li>Append to <code>CHANGELOG.md</code></li>
<li>Commit the changes</li>
<li>Create and push <code>v0.2.0</code></li>
<li>Create a GitHub release (if <code>GITHUB_TOKEN</code> is set)</li>
</ul>
</li>
</ol>

<aside class="ferr-aside ferr-aside--tip"><p class="ferr-aside__title">Starting from scratch</p><div class="ferr-aside__body"><p>No prior tag? FerrFlow v3 bootstraps from the strategy&#39;s zero value automatically — the first <code>feat:</code> lands at <code>0.1.0</code>, the first <code>fix:</code> at <code>0.0.1</code>. You don&#39;t need to create a <code>v0.0.0</code> tag by hand. See <a href="/docs/reference/cli#which-version-is-bumped-from">how the baseline is chosen</a>.</p>
</div></aside>

## Next steps

- Set up [GitHub Actions](/docs/ci/github-actions) to run releases automatically on push to `main`
- Configure a [monorepo](/docs/configuration/monorepo) if you have multiple packages
- Add [pre/post-release hooks](/docs/configuration/config-file#hooks) for custom scripts during the release lifecycle
- Use `ferrflow version` and `ferrflow tag` in CI scripts — see the [CLI reference](/docs/reference/cli)
- Review the full [config reference](/docs/configuration/config-file)
