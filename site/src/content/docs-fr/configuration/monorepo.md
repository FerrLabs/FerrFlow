---
title: Monorepo
description: Versionner plusieurs packages indépendamment dans un seul repository.
---

FerrFlow considère un repository comme un monorepo lorsque la configuration définit plus d'un package. Chaque package est versionné indépendamment en fonction de son propre historique git.

## Isolation des packages

FerrFlow utilise les préfixes de chemin pour déterminer quels commits appartiennent à quel package. Seuls les commits qui touchent des fichiers sous `path` (ou `sharedPaths`) déclenchent une release pour ce package.

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

## Dépendances partagées

Si vous avez du code partagé entre packages (ex. une bibliothèque `packages/shared/`), déclarez-le comme entrée `sharedPaths`. Un changement dans un chemin partagé déclenche une release pour chaque package qui le référence :

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

## Dependances entre packages

Utilisez `dependsOn` pour declarer qu'un package depend d'un autre. Quand une dependance est publiee, le package dependant recoit automatiquement un bump patch — meme si aucun de ses propres fichiers n'a change. La cascade est transitive : si `app` depend de `cli` et `cli` depend de `core`, publier `core` bumpe aussi `cli` et `app`.

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

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p><code>dependsOn</code> est different de <code>sharedPaths</code>. Les chemins partages declenchent un bump quand des fichiers dans le repertoire partage changent. <code>dependsOn</code> declenche un bump quand un autre <strong>package</strong> est publie, independamment des fichiers modifies.</p>
</div></aside>

### Cycles de dépendances

`dependsOn` doit décrire un graphe orienté acyclique. Si deux packages dépendent l'un de l'autre — directement ou transitivement — il n'existe aucun ordre de release possible : FerrFlow s'arrête alors avec l'erreur `E8003` en nommant la boucle :

```
cycle detected: api → web → api
```

La vérification s'exécute avant toute écriture de version : une configuration cyclique ne produit jamais de release partielle. Cassez la boucle en supprimant l'une des arêtes `dependsOn`. Sinon, le graphe est publié dépendances d'abord : un package est toujours publié après les packages dont il dépend.

## Groupes de versions liées et fixes

Parfois, plusieurs packages doivent partager le même numéro de version, pas seulement propager un bump. `linked` et `fixed` listent des groupes de packages qui évoluent ensemble :

```toml
[workspace]
linked = [["react", "react-dom"]]
fixed  = [["@scope/a", "@scope/b", "@scope/c"]]
```

Dès qu'**un** membre d'un groupe a un commit publiable, tous les membres sont bumpés à la même version — la plus haute que le groupe atteindrait. Un `feat` sur un membre et un `fix` sur un autre publient tout le groupe sur le minor. Les noms de packages restent distincts ; seule la version est partagée, et les membres sans commit propre sont intégrés à la release à la version partagée.

- **`linked`** — les packages partagent une ligne de version lorsqu'ils sont publiés ensemble (par exemple `react` et `react-dom` passent tous deux de `1.2.3` à `1.2.4`).
- **`fixed`** — les packages sont verrouillés sur une version identique en permanence. Le comportement est celui de `linked`, et `ferrflow validate` avertit en plus lorsque les versions d'un groupe `fixed` ont déjà divergé, pour repérer une édition manuelle avant que la prochaine release ne les réaligne.

Chaque groupe doit lister au moins deux packages, et un package ne peut appartenir qu'à un seul groupe `linked` ou `fixed`. Nommer un package absent de `package[]`, ou en lister un dans deux groupes, arrête la release avec une erreur claire avant toute écriture — la même garantie amont que les [cycles de dépendances](#cycles-de-dépendances).

`linked`/`fixed` et `dependsOn` se combinent : un package qui dépend d'un package groupé reçoit toujours son bump en cascade après l'alignement du groupe.

## Format des tags git

Par défaut, les tags en monorepo utilisent le format `{name}@v{version}` :

```
api@v1.2.0
site@v0.4.1
```

Configurez cela avec le champ `tagTemplate` :

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

Pour un repo mono-package, le défaut est `v{version}` (sans préfixe de nom).

FerrFlow recherche le tag le plus récent correspondant au modèle pour déterminer quels commits sont nouveaux.

## Cadences indépendantes

Les packages sont publiés indépendamment. Dans une seule exécution de `ferrflow release` :

- `api` peut passer de `1.2.0` → `1.3.0` (nouveau commit `feat:`)
- `site` peut passer de `0.4.0` → `0.4.1` (uniquement des commits `fix:`)
- `shared` peut ne pas être publié (uniquement des commits `chore:`)

## Surcharges par package

Chaque package peut surcharger la stratégie de `versioning` et le `tagTemplate` du workspace :

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

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Utilisez <code>ferrflow check</code> pour prévisualiser exactement quels packages seraient publiés et à quelle version avant de lancer une release.</p>
</div></aside>
