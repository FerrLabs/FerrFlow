---
title: Installation
description: How to install FerrFlow locally or in CI.
---

## Local installation

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="Cargo"><p class="ferr-tab__label">Cargo</p><div class="ferr-tab__body"><pre><code class="language-bash">cargo install ferrflow
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="npm"><p class="ferr-tab__label">npm</p><div class="ferr-tab__body"><pre><code class="language-bash">npm install -g ferrflow
# or as a dev dependency
npm install -D ferrflow
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="WASM (browser)"><p class="ferr-tab__label">WASM (browser)</p><div class="ferr-tab__body"><pre><code class="language-bash">npm install @ferrflow/wasm
</code></pre>
<p>Use FerrFlow directly in the browser — parse commits, compute version bumps, and generate changelogs client-side without a backend.</p>
</div></div>
  <div class="ferr-tab" data-label="Binary"><p class="ferr-tab__label">Binary</p><div class="ferr-tab__body"><p>Download a pre-built binary from <a href="https://github.com/FerrLabs/FerrFlow/releases/latest">Releases</a>:</p>
<pre><code class="language-bash"># Linux x86_64
curl -L https://github.com/FerrLabs/FerrFlow/releases/latest/download/ferrflow-linux-x64.tar.gz | tar xz
sudo mv ferrflow /usr/local/bin/
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="Docker"><p class="ferr-tab__label">Docker</p><div class="ferr-tab__body"><pre><code class="language-bash">docker run --rm -v $(pwd):/repo ghcr.io/ferrlabs/ferrflow:latest check
</code></pre>
</div></div>
</div>

## CI installation

The recommended way to use FerrFlow in CI is the GitHub Action — no installation step needed:

```yaml title=".github/workflows/release.yml"
- uses: FerrLabs/ferrflow@v4
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

See [GitHub Actions](/docs/ci/github-actions) and [GitLab CI](/docs/ci/gitlab-ci) for complete examples.

## Verify

```bash
ferrflow --version
```
