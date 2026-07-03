---
title: Installation
description: How to install FerrFlow locally or in CI.
---

## Local installation

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="Cargo"><p class="ferr-tab__label">Cargo</p><div class="ferr-tab__body"><pre><code class="language-bash">cargo install ferrflow
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="npm"><p class="ferr-tab__label">npm</p><div class="ferr-tab__body"><pre><code class="language-bash">npm install -g @ferrlabs/ferrflow
# or as a dev dependency
npm install -D @ferrlabs/ferrflow
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
- uses: FerrLabs/ferrflow@v5
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

See [GitHub Actions](/docs/ci/github-actions) and [GitLab CI](/docs/ci/gitlab-ci) for complete examples.

## Verify

```bash
ferrflow --version
```

## Upgrading from v4

If you're following the documented GitHub Actions / GitLab CI setup (`GITHUB_TOKEN`/`CI_JOB_TOKEN` as an environment variable), no changes are required — just bump the action pin to `FerrLabs/ferrflow@v5` and the binary to v5.x.

The only breaking change in v5.0 is internal: FerrFlow no longer injects tokens into the remote URL when pushing. It now uses the standard git credential-helper protocol (`GIT_ASKPASS`). This is invisible to anyone using the recommended setup, but if you had a custom workflow that relied on URL-injected tokens — for example, a self-hosted runner with a pre-seeded `https://x-access-token:$TOKEN@github.com/...` remote — switch to setting `GITHUB_TOKEN` (or `FERRFLOW_TOKEN`) as an environment variable instead and FerrFlow handles the rest.

Releases since v5.2 are signed via Sigstore and ship a CycloneDX SBOM — see [Verifying releases](/docs/verifying-releases).
