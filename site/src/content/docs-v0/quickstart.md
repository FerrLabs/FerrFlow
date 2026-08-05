---
title: Quick start
description: Go from zero to your first automated release in under 5 minutes.
slug: v0/docs/quickstart
---

<ol>
<li><p><strong>Scaffold the config</strong></p>
<p>Run <code>ferrflow init</code> at the root of your repository. It detects your version files and writes a <code>ferrflow.toml</code>:</p>
<pre><code class="language-bash">ferrflow init
</code></pre>
<p>For a Rust project this produces:</p>
<pre><code class="language-toml">[workspace]
remote = &quot;origin&quot;
branch = &quot;main&quot;

[[package]]
name = &quot;my-app&quot;
path = &quot;.&quot;
changelog = &quot;CHANGELOG.md&quot;

[[package.versioned_files]]
path = &quot;Cargo.toml&quot;
format = &quot;toml&quot;
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
Tag my-app@v0.2.0
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
<li>Create and push <code>my-app@v0.2.0</code></li>
<li>Create a GitHub release (if <code>GITHUB_TOKEN</code> is set)</li>
</ul>
</li>
</ol>

## Next steps

- Set up [GitHub Actions](/v0/docs/ci/github-actions) to run releases automatically on push to `main`
- Configure a [monorepo](/v0/docs/configuration/monorepo) if you have multiple packages
- Review the full [config reference](/v0/docs/configuration/config-file)
