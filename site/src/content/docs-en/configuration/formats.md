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
<p>The newer <code>chartyaml</code> alias is functionally equivalent — use whichever reads more naturally in your config.</p>
<pre><code class="language-yaml">apiVersion: v2
name: my-app
version: 1.2.3        # ← updated
appVersion: &quot;1.2.3&quot;   # ← updated when present
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="Dart"><p class="ferr-tab__label">Dart</p><div class="ferr-tab__body"><p>Used by Dart and Flutter packages (<code>pubspec.yaml</code>).</p>
<p>FerrFlow updates the top-level <code>version:</code> key, leaving dependency versions, anchors, and comments intact. SemVer build suffixes (<code>1.2.3+42</code>) are supported.</p>
<pre><code class="language-yaml">name: my_app
version: 1.2.3+42     # ← updated
dependencies:
  some_pkg:
    version: 2.0.0    # untouched — this is a dep constraint
</code></pre>
<p>Config snippet:</p>
<pre><code class="language-toml">[[package.versioned_files]]
path   = &quot;pubspec.yaml&quot;
format = &quot;pubspecyaml&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="Elixir"><p class="ferr-tab__label">Elixir</p><div class="ferr-tab__body"><p>Used by Elixir / Mix projects (<code>mix.exs</code>).</p>
<p>FerrFlow updates the first <code>version: &quot;…&quot;</code> literal it finds — the canonical spot is inside <code>def project do [ ..., version: &quot;x.y.z&quot;, ... ] end</code>.</p>
<pre><code class="language-elixir">def project do
  [
    app: :my_app,
    version: &quot;1.2.3&quot;,   # ← updated
    elixir: &quot;~&gt; 1.15&quot;,
    deps: deps()
  ]
end
</code></pre>
<p>Config snippet:</p>
<pre><code class="language-toml">[[package.versioned_files]]
path   = &quot;mix.exs&quot;
format = &quot;mixexs&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="Ruby"><p class="ferr-tab__label">Ruby</p><div class="ferr-tab__body"><p>Used by Ruby gems (<code>*.gemspec</code>).</p>
<p>FerrFlow updates the <code>.version = &quot;…&quot;</code> assignment. Any receiver name works (<code>s</code>, <code>spec</code>, <code>gem</code>, …). Setting <code>version</code> from a constant (<code>s.version = MyGem::VERSION</code>) isn&#39;t covered — version the loaded <code>version.rb</code> file directly in that case.</p>
<pre><code class="language-ruby">Gem::Specification.new do |s|
  s.name    = &quot;my_gem&quot;
  s.version = &quot;1.2.3&quot;  # ← updated
end
</code></pre>
<p>Config snippet:</p>
<pre><code class="language-toml">[[package.versioned_files]]
path   = &quot;my_gem.gemspec&quot;
format = &quot;gemspec&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="Swift"><p class="ferr-tab__label">Swift</p><div class="ferr-tab__body"><p>Used by Swift packages (<code>Package.swift</code>).</p>
<p>Swift PM derives a package&#39;s version from git tags, so there&#39;s no canonical location inside <code>Package.swift</code> — FerrFlow updates the first <code>let &lt;name&gt;Version = &quot;…&quot;</code> declaration. Constant names must end with <code>Version</code> (e.g. <code>packageVersion</code>, <code>AppVersion</code>) or be literally <code>version</code>. Dependency <code>.package(url:..., from: &quot;…&quot;)</code> arguments are <strong>not</strong> touched.</p>
<pre><code class="language-swift">import PackageDescription

let packageVersion = &quot;1.2.3&quot; // ← updated

let package = Package(
name: &quot;MyPackage&quot;,
dependencies: [
.package(url: &quot;…&quot;, from: &quot;1.5.0&quot;), // untouched
]
)
</code></pre>
<p>Config snippet:</p>
<pre><code class="language-toml">[[package.versioned_files]]
path   = &quot;Package.swift&quot;
format = &quot;packageswift&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="Haskell"><p class="ferr-tab__label">Haskell</p><div class="ferr-tab__body"><p>Used by Haskell packages (<code>*.cabal</code>).</p>
<p>FerrFlow updates the top-level <code>version:</code> field. Field names are case-insensitive and top-level fields sit at column 0, so <code>cabal-version:</code> — which declares the file format, not the package version — is <strong>never</strong> touched, and neither is an indented <code>version:</code> inside a stanza.</p>
<pre><code class="language-cabal">cabal-version:      2.4     &lt;-- untouched
name:               my-package
version:            1.2.3   &lt;-- updated
</code></pre>
<p>Config snippet:</p>
<pre><code class="language-toml">[[package.versioned_files]]
path   = &quot;my-package.cabal&quot;
format = &quot;cabal&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="CMake"><p class="ferr-tab__label">CMake</p><div class="ferr-tab__body"><p>Used by C / C++ projects (<code>CMakeLists.txt</code>).</p>
<p>FerrFlow updates the <code>VERSION</code> argument of the <code>project()</code> call, including the multi-line form. <code>cmake_minimum_required(VERSION …)</code> — the CMake tool floor — and any <code>set(&lt;name&gt;_VERSION …)</code> variable are <strong>not</strong> touched.</p>
<pre><code class="language-cmake">cmake_minimum_required(VERSION 3.20)   // untouched

project(MyProject
VERSION 1.2.3 // updated
LANGUAGES CXX)
</code></pre>
<p>Config snippet:</p>
<pre><code class="language-toml">[[package.versioned_files]]
path   = &quot;CMakeLists.txt&quot;
format = &quot;cmake&quot;
</code></pre>
</div></div>
</div>

## File → format quick reference

| File                               | `format`              | Selector / behaviour                       |
| ---------------------------------- | --------------------- | ------------------------------------------ |
| `Cargo.toml`                       | `toml`                | `package.version`                          |
| `pyproject.toml`                   | `toml`                | `project.version` or `tool.poetry.version` |
| `package.json`                     | `json`                | `version`                                  |
| `composer.json`                    | `json`                | `version`                                  |
| `pom.xml`                          | `xml`                 | first `<version>` tag                      |
| `*.csproj`                         | `csproj`              | `<Version>` in `<PropertyGroup>`           |
| `build.gradle`, `build.gradle.kts` | `gradle`              | `version = "…"`                            |
| `Chart.yaml`                       | `helm` or `chartyaml` | top-level `version:`                       |
| `pubspec.yaml`                     | `pubspecyaml`         | top-level `version:`                       |
| `mix.exs`                          | `mixexs`              | `version: "…"` in project keyword list     |
| `*.gemspec`                        | `gemspec`             | `<ident>.version = "…"`                    |
| `Package.swift`                    | `packageswift`        | top-level `let <name>Version = "…"`        |
| `*.cabal`                          | `cabal`               | top-level `version:` field                 |
| `CMakeLists.txt`                   | `cmake`               | `VERSION` argument of `project()`          |
| `go.mod`                           | `gomod`               | git tag only — no file write               |
| `VERSION`, `VERSION.txt`           | `txt`                 | entire file content                        |

## Multiple files per package

A package can have as many versioned file entries as needed:

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;package&quot;: [
    {
      &quot;versionedFiles&quot;: [
        { &quot;path&quot;: &quot;Cargo.toml&quot;, &quot;format&quot;: &quot;toml&quot; },
        { &quot;path&quot;: &quot;npm/package.json&quot;, &quot;format&quot;: &quot;json&quot; }
      ]
    }
  ]
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
  package: [
    {
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

Both files will be updated to the same version before the git commit.
