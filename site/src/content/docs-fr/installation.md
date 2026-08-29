---
title: Installation
description: Comment installer FerrFlow en local ou en CI.
---

## Installation locale

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="Cargo"><p class="ferr-tab__label">Cargo</p><div class="ferr-tab__body"><pre><code class="language-bash">cargo install ferrflow
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="npm"><p class="ferr-tab__label">npm</p><div class="ferr-tab__body"><pre><code class="language-bash">npm install -g @ferrlabs/ferrflow
# ou en dépendance de développement
npm install -D @ferrlabs/ferrflow
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="WASM (navigateur)"><p class="ferr-tab__label">WASM (navigateur)</p><div class="ferr-tab__body"><pre><code class="language-bash">npm install @ferrflow/wasm
</code></pre>
<p>Utilisez FerrFlow directement dans le navigateur : parsez les commits, calculez les incréments de version et générez des changelogs côté client sans backend.</p>
</div></div>
  <div class="ferr-tab" data-label="Binaire"><p class="ferr-tab__label">Binaire</p><div class="ferr-tab__body"><p>Téléchargez un binaire pré-compilé depuis les <a href="https://github.com/FerrLabs/FerrFlow/releases/latest">Releases</a> :</p>
<pre><code class="language-bash"># Linux x86_64
curl -L https://github.com/FerrLabs/FerrFlow/releases/latest/download/ferrflow-linux-x64.tar.gz | tar xz
sudo mv ferrflow /usr/local/bin/
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="Docker"><p class="ferr-tab__label">Docker</p><div class="ferr-tab__body"><pre><code class="language-bash">docker run --rm -v $(pwd):/repo ghcr.io/ferrlabs/ferrflow:latest check
</code></pre>
</div></div>
</div>

## Installation CI

La méthode recommandée pour utiliser FerrFlow en CI est la GitHub Action : aucune étape d'installation nécessaire :

```yaml
- uses: FerrLabs/ferrflow@v5
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

Consultez [GitHub Actions](/fr/docs/ci/github-actions) et [GitLab CI](/fr/docs/ci/gitlab-ci) pour des exemples complets.

## Vérification

```bash
ferrflow --version
```

## Migration depuis la v4

Si vous suivez la configuration documentée pour GitHub Actions / GitLab CI (`GITHUB_TOKEN` / `CI_JOB_TOKEN` en variable d'environnement), aucun changement n'est nécessaire. Il suffit de bumper le pin de l'action à `FerrLabs/ferrflow@v5` et le binaire à la v5.x.

Le seul changement cassant de la v5.0 est interne : FerrFlow n'injecte plus les tokens dans l'URL distante lors des push. Il utilise désormais le protocole standard de credential helper de git (`GIT_ASKPASS`). C'est invisible pour quiconque suit la configuration recommandée, mais si vous aviez un workflow custom qui s'appuyait sur des tokens injectés dans l'URL (par exemple, un runner self-hosted avec un remote pré-amorcé `https://x-access-token:$TOKEN@github.com/...`) passez à `GITHUB_TOKEN` (ou `FERRFLOW_TOKEN`) en variable d'environnement et FerrFlow se charge du reste.

Depuis la v5.2, les releases sont signées via Sigstore et embarquent un SBOM CycloneDX : voir [Vérifier les releases](/fr/docs/verifying-releases).
