---
title: Démarrage rapide
description: De zéro à votre première release automatisée en moins de 5 minutes.
---

<ol>
<li><p><strong>Générer la configuration</strong></p>
<p>Exécutez <code>ferrflow init</code> à la racine de votre repository. Il détecte vos fichiers de version et génère un fichier <code>.ferrflow</code> :</p>
<pre><code class="language-bash">ferrflow init
</code></pre>
<p>Pour un projet Rust, cela produit :</p>
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
<li><p><strong>Prévisualiser le résultat</strong></p>
<p>Avant de toucher à quoi que ce soit, lancez un dry-run pour voir ce que FerrFlow ferait :</p>
<pre><code class="language-bash">ferrflow check
</code></pre>
<p>Sortie :</p>
<pre><code>Scanning . ...
→ feat: add user authentication
→ fix: correct pagination offset

Bump my-app 0.1.0 → 0.2.0
Tag v0.2.0
</code></pre>
</li>
<li><p><strong>Lancer la release</strong></p>
<pre><code class="language-bash">ferrflow release
</code></pre>
<p>FerrFlow va :</p>
<ul>
<li>Mettre à jour <code>Cargo.toml</code> à <code>0.2.0</code></li>
<li>Compléter <code>CHANGELOG.md</code></li>
<li>Committer les changements</li>
<li>Créer et pousser <code>v0.2.0</code></li>
<li>Créer une release GitHub (si <code>GITHUB_TOKEN</code> est défini)</li>
</ul>
</li>
</ol>

## Étapes suivantes

- Configurez [GitHub Actions](/fr/docs/ci/github-actions) pour lancer les releases automatiquement sur push vers `main`
- Configurez un [monorepo](/fr/docs/configuration/monorepo) si vous avez plusieurs packages
- Ajoutez des [hooks pre/post-release](/fr/docs/configuration/config-file#hooks) pour des scripts personnalisés pendant le cycle de release
- Utilisez `ferrflow version` et `ferrflow tag` dans vos scripts CI — voir la [référence CLI](/fr/docs/reference/cli)
- Consultez la [référence de configuration](/fr/docs/configuration/config-file) complète
