---
title: Supported formats
description: Version file formats that FerrFlow can read and update.
---

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><p>Used by Node.js (<code>package.json</code>).</p>
<p>FerrFlow updates the top-level <code>version</code> field.</p>
<pre><code class="language-json">{
  &quot;name&quot;: &quot;my-package&quot;,
  &quot;version&quot;: &quot;1.2.3&quot;
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><p>Used by Rust (<code>Cargo.toml</code>) and Python (<code>pyproject.toml</code>).</p>
<p>FerrFlow updates the <code>version</code> field under <code>[package]</code>, <code>[project]</code>, or <code>[tool.poetry]</code>.</p>
<pre><code class="language-toml">[package]
name = &quot;my-crate&quot;
version = &quot;1.2.3&quot;   # ← updated
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="XML"><p class="ferr-tab__label">XML</p><div class="ferr-tab__body"><p>Used by Java/Maven (<code>pom.xml</code>).</p>
<p>FerrFlow updates the first <code>&lt;version&gt;</code> element it encounters.</p>
<pre><code class="language-xml">&lt;project&gt;
  &lt;groupId&gt;com.example&lt;/groupId&gt;
  &lt;artifactId&gt;my-app&lt;/artifactId&gt;
  &lt;version&gt;1.2.3&lt;/version&gt;   &lt;!-- updated --&gt;
&lt;/project&gt;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="Gradle"><p class="ferr-tab__label">Gradle</p><div class="ferr-tab__body"><p>Used by Java/Kotlin Gradle projects (<code>build.gradle</code>, <code>build.gradle.kts</code>).</p>
<p>FerrFlow updates the <code>version = &quot;...&quot;</code> assignment.</p>
<pre><code class="language-groovy">version = &quot;1.2.3&quot;   // updated
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="Plain text"><p class="ferr-tab__label">Plain text</p><div class="ferr-tab__body"><p>Used for simple version files (<code>VERSION</code>, <code>VERSION.txt</code>).</p>
<p>FerrFlow replaces the entire file content with the version number.</p>
<pre><code>1.2.3
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="Go"><p class="ferr-tab__label">Go</p><div class="ferr-tab__body"><p>Used by Go projects (<code>go.mod</code>).</p>
<p>Go modules use git tags directly — FerrFlow does <strong>not</strong> modify <code>go.mod</code>. The version is derived entirely from the git tag (<code>v1.2.3</code> or <code>{name}@v1.2.3</code>).</p>
<p>On a brand-new repo with no matching tag yet, FerrFlow v3+ bootstraps from the strategy&#39;s zero value (<code>0.0.0</code> for <code>semver</code>, <code>0</code> for <code>sequential</code>, …) and creates the first real tag itself — you do not need to run <code>git tag … v0.0.0</code> before the first release. See <a href="/docs/reference/cli#which-version-is-bumped-from">how <code>release</code> picks the baseline</a>.</p>
</div></div>
  <div class="ferr-tab" data-label="Helm"><p class="ferr-tab__label">Helm</p><div class="ferr-tab__body"><p>Used by Kubernetes Helm charts (<code>Chart.yaml</code>).</p>
<p>FerrFlow updates the <code>version</code> field and, when present, keeps <code>appVersion</code> in sync.</p>
<pre><code class="language-yaml">apiVersion: v2
name: my-app
version: 1.2.3        # ← updated
appVersion: &quot;1.2.3&quot;   # ← updated when present
</code></pre>
</div></div>
</div>

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
