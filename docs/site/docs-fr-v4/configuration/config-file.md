---
title: Configuration
description: Référence complète du fichier de configuration FerrFlow.
---

FerrFlow supporte six formats de fichier de configuration, recherch\u00e9s dans cet ordre :

1. `ferrflow.json`
2. `ferrflow.json5`
3. `ferrflow.toml`
4. `ferrflow.ts` (n\u00e9cessite `tsx`)
5. `ferrflow.js` (n\u00e9cessite `node`)
6. `.ferrflow` (JSON)

Si aucun fichier de configuration n'est trouv\u00e9, FerrFlow d\u00e9tecte automatiquement les fichiers de version courants dans le r\u00e9pertoire actuel.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Ajoutez <code>&quot;$schema&quot;: &quot;https://ferrflow.com/schema/ferrflow.json&quot;</code> à votre configuration JSON pour l&#39;autocomplétion et la validation dans votre éditeur.</p>
</div></aside>

## Formats de configuration

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="TypeScript"><p class="ferr-tab__label">TypeScript</p><div class="ferr-tab__body"><pre><code class="language-ts">export default {
  workspace: {
    tagTemplate: &quot;v{version}&quot;,
  },
  package: [
    {
      name: &quot;my-app&quot;,
      path: &quot;.&quot;,
      changelog: &quot;CHANGELOG.md&quot;,
      versionedFiles: [
        { path: &quot;Cargo.toml&quot;, format: &quot;toml&quot; },
      ],
    },
  ],
};
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
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
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
tag_template = &quot;v{version}&quot;

[[package]]
name = &quot;my-app&quot;
path = &quot;.&quot;
changelog = &quot;CHANGELOG.md&quot;

[[package.versioned_files]]
path = &quot;Cargo.toml&quot;
format = &quot;toml&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  $schema: &quot;https://ferrflow.com/schema/ferrflow.json&quot;,
  workspace: {
    tagTemplate: &quot;v{version}&quot;,
  },
  package: [
    {
      name: &quot;my-app&quot;,
      path: &quot;.&quot;,
      changelog: &quot;CHANGELOG.md&quot;,
      versionedFiles: [
        { path: &quot;Cargo.toml&quot;, format: &quot;toml&quot; },
      ],
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  tagTemplate: &quot;v{version}&quot;

package:

- name: my-app
  path: &quot;.&quot;
  changelog: CHANGELOG.md
  versionedFiles:
  - path: Cargo.toml
    format: toml
    </code></pre>

</div></div>
</div>

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p>Les configurations JSON, JSON5, et TypeScript/JavaScript utilisent des cl\u00e9s en <strong>camelCase</strong> (<code>tagTemplate</code>, <code>versionedFiles</code>).
La configuration TOML utilise des cl\u00e9s en <strong>snake_case</strong> (<code>tag_template</code>, <code>versioned_files</code>).
Les configurations YAML supportent les deux, mais <strong>camelCase</strong> est recommand\u00e9 pour la coh\u00e9rence avec JSON.
Toutes les formes sont \u00e9quivalentes.</p>
</div></aside>

### Configurations TypeScript et JavaScript

Les fichiers de config TypeScript (`.ts`) et JavaScript (`.js`) utilisent un export ESM par d\u00e9faut. L'export peut \u00eatre un objet ou une fonction asynchrone.

<aside class="ferr-aside ferr-aside--warning"><div class="ferr-aside__body"><p>Les configs TypeScript n\u00e9cessitent <code>tsx</code> (<code>npm install -g tsx</code>). Les configs JavaScript n\u00e9cessitent <code>node</code> (v18+).</p>
</div></aside>

L'avantage principal des configs TS/JS : les **hooks sous forme de fonctions**. Au lieu de commandes shell, vous pouvez \u00e9crire des hooks natifs avec acc\u00e8s complet au contexte :

```ts title="ferrflow.ts"
export default {
  workspace: {
    tagTemplate: 'v{version}',
    hooks: {
      postPublish: async (ctx) => {
        await fetch('https://hooks.slack.com/services/...', {
          method: 'POST',
          body: JSON.stringify({
            text: `Released ${ctx.package}@${ctx.newVersion}`,
          }),
        });
      },
    },
  },
  package: [
    {
      name: 'my-app',
      path: '.',
      versionedFiles: [{ path: 'package.json', format: 'json' }],
    },
  ],
};
```

#### Objet de contexte des hooks

Les hooks en fonction re\u00e7oivent un objet de contexte avec ces champs :

| Champ          | Type           | Description                                                |
| -------------- | -------------- | ---------------------------------------------------------- |
| `package`      | string         | Nom du package                                             |
| `oldVersion`   | string         | Version avant le bump (vide pour la premi\u00e8re release) |
| `newVersion`   | string         | Version apr\u00e8s le bump                                 |
| `bumpType`     | string         | `major`, `minor`, `patch`, ou `none`                       |
| `tag`          | string         | Nom complet du tag git                                     |
| `dryRun`       | boolean        | Vrai si `--dry-run` est actif                              |
| `packagePath`  | string         | Chemin absolu vers la racine du package                    |
| `channel`      | string ou null | Nom du channel de pr\u00e9-release                         |
| `isPrerelease` | boolean        | Vrai si c'est une pr\u00e9-release                         |

Les hooks sous forme de commandes shell et de fonctions peuvent \u00eatre m\u00e9lang\u00e9s dans la m\u00eame config.

## `workspace`

Paramètres globaux qui s'appliquent à tous les packages.

| Champ                   | Type    | Défaut                                  | Description                                                                                                                                                                                        |
| ----------------------- | ------- | --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `remote`                | string  | `"origin"`                              | Remote git vers lequel pousser                                                                                                                                                                     |
| `branch`                | string  | auto-détecté                            | Branche vers laquelle pousser (détectée depuis le HEAD du remote)                                                                                                                                  |
| `tagTemplate`           | string  | `"v{version}"` ou `"{name}@v{version}"` | Modèle de nommage des tags. Utilise les placeholders `{version}` et `{name}`. Par défaut `v{version}` pour les repos mono-package et `{name}@v{version}` pour les monorepos.                       |
| `versioning`            | string  | `"semver"`                              | Stratégie de versionnage par défaut pour tous les packages                                                                                                                                         |
| `releaseCommitMode`     | string  | `"commit"`                              | Gestion du commit de release : `"commit"`, `"pr"` ou `"none"`                                                                                                                                      |
| `skipCi`                | boolean | dépend du mode                          | Ajouter `[skip ci]` aux commits de release. Par défaut `true` en mode `"commit"`, `false` sinon.                                                                                                   |
| `autoMergeReleases`     | boolean | `true`                                  | Activer l'auto-merge sur les PR de release (uniquement en mode `"pr"`)                                                                                                                             |
| `recoverMissedReleases` | boolean | `false`                                 | Lorsqu'activé, si FerrFlow trouve des commits non publiés couvrant plusieurs incréments de version, il crée toutes les releases intermédiaires au lieu de sauter directement à la dernière version |
| `telemetry`             | boolean | `true`                                  | Envoyer des données de télémétrie anonymes                                                                                                                                                         |

### Modèle de tag

Le champ `tagTemplate` contrôle le nommage des tags git. Placeholders disponibles :

| Placeholder | Description                        |
| ----------- | ---------------------------------- |
| `{version}` | Le numéro de version (ex. `1.2.3`) |
| `{name}`    | Le nom du package                  |

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;tagTemplate&quot;: &quot;v{version}&quot;
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
tag_template = &quot;v{version}&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    tagTemplate: &quot;v{version}&quot;,
  },
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  tagTemplate: &quot;v{version}&quot;
</code></pre>
</div></div>
</div>

Pour les monorepos, utilisez `{name}` pour namespacer les tags par package :

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

### Mode de commit de release

Contrôle la façon dont FerrFlow gère le commit qui met à jour les fichiers de version et les changelogs.

| Mode       | Comportement                                                                        |
| ---------- | ----------------------------------------------------------------------------------- |
| `"commit"` | Commit directement sur la branche courante et pousse (par défaut)                   |
| `"pr"`     | Crée une branche `release/` et ouvre une pull request                               |
| `"none"`   | Crée uniquement les tags et les releases, ne commit pas les changements de fichiers |

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;releaseCommitMode&quot;: &quot;pr&quot;,
    &quot;autoMergeReleases&quot;: true
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
release_commit_mode = &quot;pr&quot;
auto_merge_releases = true
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    releaseCommitMode: &quot;pr&quot;,
    autoMergeReleases: true,
  },
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  releaseCommitMode: pr
  autoMergeReleases: true
</code></pre>
</div></div>
</div>

### Stratégies de versionnage

FerrFlow supporte plusieurs stratégies de versionnage, configurables au niveau du workspace ou du package.

| Stratégie      | Format              | Progression exemple                      |
| -------------- | ------------------- | ---------------------------------------- |
| `semver`       | `MAJOR.MINOR.PATCH` | `1.2.3` → `1.3.0` → `2.0.0`              |
| `calver`       | `YYYY.MM.PATCH`     | `2026.03.0` → `2026.03.1` → `2026.04.0`  |
| `calver-short` | `YY.MM.PATCH`       | `26.03.0` → `26.03.1`                    |
| `calver-seq`   | `YYYY.MM.SEQ`       | `2026.03.1` → `2026.03.2`                |
| `sequential`   | `N`                 | `1` → `2` → `3`                          |
| `zerover`      | `0.MINOR.PATCH`     | `0.1.0` → `0.2.0` (n'atteint jamais 1.0) |

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;versioning&quot;: &quot;calver&quot;
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
versioning = &quot;calver&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    versioning: &quot;calver&quot;,
  },
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  versioning: calver
</code></pre>
</div></div>
</div>

## `package`

Définit un package à versionner. Vous pouvez en avoir un ou plusieurs.

| Champ         | Requis | Défaut                | Description                                                 |
| ------------- | ------ | --------------------- | ----------------------------------------------------------- |
| `name`        | oui    | —                     | Identifiant du package, utilisé dans le préfixe du tag git  |
| `path`        | oui    | —                     | Chemin relatif vers le répertoire du package                |
| `changelog`   | non    | `{path}/CHANGELOG.md` | Chemin vers le fichier changelog                            |
| `sharedPaths` | non    | `[]`                  | Chemins qui déclenchent ce package lorsqu'ils sont modifiés |
| `versioning`  | non    | hérité du workspace   | Surcharger la stratégie de versionnage pour ce package      |
| `tagTemplate` | non    | hérité du workspace   | Surcharger le modèle de tag pour ce package                 |

### `versionedFiles`

Fichiers dans lesquels le numéro de version doit être mis à jour.

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;my-app&quot;,
      &quot;path&quot;: &quot;.&quot;,
      &quot;versionedFiles&quot;: [
        { &quot;path&quot;: &quot;Cargo.toml&quot;, &quot;format&quot;: &quot;toml&quot; },
        { &quot;path&quot;: &quot;npm/package.json&quot;, &quot;format&quot;: &quot;json&quot; }
      ]
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[[package]]
name = &quot;my-app&quot;
path = &quot;.&quot;

[[package.versioned_files]]
path = &quot;Cargo.toml&quot;
format = &quot;toml&quot;

[[package.versioned_files]]
path = &quot;npm/package.json&quot;
format = &quot;json&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  package: [
    {
      name: &quot;my-app&quot;,
      path: &quot;.&quot;,
      versionedFiles: [
        { path: &quot;Cargo.toml&quot;, format: &quot;toml&quot; },
        { path: &quot;npm/package.json&quot;, format: &quot;json&quot; },
      ],
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">package:
  - name: my-app
    path: &quot;.&quot;
    versionedFiles:
      - path: Cargo.toml
        format: toml
      - path: npm/package.json
        format: json
</code></pre>
</div></div>
</div>

| `format` | Fichier                            | Champ mis à jour                                                         |
| -------- | ---------------------------------- | ------------------------------------------------------------------------ |
| `toml`   | `Cargo.toml`, `pyproject.toml`     | `[package].version` ou `[project].version`                               |
| `json`   | `package.json`                     | `version`                                                                |
| `xml`    | `pom.xml`                          | Premier élément `<version>`                                              |
| `gradle` | `build.gradle`, `build.gradle.kts` | `version = "..."`                                                        |
| `helm`   | `Chart.yaml`                       | `version` et `appVersion` (si présent)                                   |
| `gomod`  | `go.mod`                           | Pas de mise à jour de fichier — la version vient uniquement des tags git |
| `txt`    | `VERSION`, `VERSION.txt`           | Contenu entier du fichier remplacé                                       |

## `hooks`

Exécutez des commandes shell à des points clés du cycle de release. Les hooks peuvent être définis au niveau du workspace (par défaut pour tous les packages) ou par package (surcharge les hooks du workspace pour ce package).

### Cycle de vie

```
calcul du bump
  ↓
pre_bump        ← valider l'état, vérifier les prérequis
  ↓
écriture des fichiers de version
  ↓
post_bump       ← modifier des fichiers supplémentaires avec la nouvelle version
  ↓
génération du changelog
  ↓
pre_commit      ← vérifier les changements stagés, lancer les linters
  ↓
git commit + tag
  ↓
pre_publish     ← lancer les tests sur le commit taggé, builder les artefacts
  ↓
git push + création de la release
  ↓
post_publish    ← pousser les images Docker, notifier Slack, publier les packages
```

### Configuration

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;hooks&quot;: {
      &quot;preBump&quot;: &quot;cargo test&quot;,
      &quot;postBump&quot;: &quot;node scripts/sync-deps.js&quot;,
      &quot;preCommit&quot;: &quot;cargo fmt --check&quot;,
      &quot;prePublish&quot;: &quot;cargo build --release&quot;,
      &quot;postPublish&quot;: &quot;make docker-push &amp;&amp; ./scripts/notify.sh&quot;,
      &quot;onFailure&quot;: &quot;abort&quot;
    }
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[hooks]
pre_bump     = &quot;cargo test&quot;
post_bump    = &quot;node scripts/sync-deps.js&quot;
pre_commit   = &quot;cargo fmt --check&quot;
pre_publish  = &quot;cargo build --release&quot;
post_publish = &quot;make docker-push &amp;&amp; ./scripts/notify.sh&quot;
on_failure   = &quot;abort&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    hooks: {
      preBump: &quot;cargo test&quot;,
      postBump: &quot;node scripts/sync-deps.js&quot;,
      preCommit: &quot;cargo fmt --check&quot;,
      prePublish: &quot;cargo build --release&quot;,
      postPublish: &quot;make docker-push &amp;&amp; ./scripts/notify.sh&quot;,
      onFailure: &quot;abort&quot;,
    },
  },
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  hooks:
    preBump: cargo test
    postBump: node scripts/sync-deps.js
    preCommit: cargo fmt --check
    prePublish: cargo build --release
    postPublish: &quot;make docker-push &amp;&amp; ./scripts/notify.sh&quot;
    onFailure: abort
</code></pre>
</div></div>
</div>

| Champ         | Type   | Défaut    | Description                                                                       |
| ------------- | ------ | --------- | --------------------------------------------------------------------------------- |
| `preBump`     | string | —         | Exécuté après le calcul du bump, avant l'écriture des fichiers de version         |
| `postBump`    | string | —         | Exécuté après l'écriture des fichiers de version                                  |
| `preCommit`   | string | —         | Exécuté après le changelog, avant le commit git                                   |
| `prePublish`  | string | —         | Exécuté après le commit+tag, avant le push                                        |
| `postPublish` | string | —         | Exécuté après le push et la création de la release                                |
| `onFailure`   | string | `"abort"` | `"abort"` annule la release en cas d'échec, `"continue"` affiche un avertissement |

### Variables d'environnement

Chaque hook reçoit ces variables :

| Variable                | Description                                           | Exemple                        |
| ----------------------- | ----------------------------------------------------- | ------------------------------ |
| `FERRFLOW_PACKAGE`      | Nom du package                                        | `api`                          |
| `FERRFLOW_OLD_VERSION`  | Version avant le bump (vide pour la première release) | `1.2.3`                        |
| `FERRFLOW_NEW_VERSION`  | Version après le bump                                 | `1.3.0`                        |
| `FERRFLOW_BUMP_TYPE`    | `major`, `minor`, `patch` ou `none`                   | `minor`                        |
| `FERRFLOW_TAG`          | Nom complet du tag git                                | `api@v1.3.0`                   |
| `FERRFLOW_DRY_RUN`      | `true` si `--dry-run` est activé                      | `false`                        |
| `FERRFLOW_PACKAGE_PATH` | Chemin absolu vers la racine du package               | `/home/user/repo/packages/api` |

### Hooks par package

Les hooks au niveau du package **remplacent** les hooks du workspace pour ce package (ils ne sont pas fusionnés).

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;hooks&quot;: {
      &quot;preBump&quot;: &quot;echo releasing $FERRFLOW_PACKAGE&quot;,
      &quot;postPublish&quot;: &quot;make notify&quot;
    }
  },
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;api&quot;,
      &quot;path&quot;: &quot;packages/api&quot;,
      &quot;hooks&quot;: {
        &quot;preBump&quot;: &quot;cargo test&quot;
      }
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[hooks]
pre_bump     = &quot;echo releasing $FERRFLOW_PACKAGE&quot;
post_publish = &quot;make notify&quot;

[[package]]
name = &quot;api&quot;
path = &quot;packages/api&quot;

[package.hooks]
pre_bump = &quot;cargo test&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    hooks: {
      preBump: &quot;echo releasing $FERRFLOW_PACKAGE&quot;,
      postPublish: &quot;make notify&quot;,
    },
  },
  package: [
    {
      name: &quot;api&quot;,
      path: &quot;packages/api&quot;,
      hooks: {
        preBump: &quot;cargo test&quot;,
      },
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  hooks:
    preBump: &quot;echo releasing $FERRFLOW_PACKAGE&quot;
    postPublish: make notify

package:

- name: api
  path: packages/api
  hooks:
  preBump: cargo test
  </code></pre>

</div></div>
</div>

Dans cet exemple, le package `api` exécute `cargo test` pour `preBump` (surchargeant l'echo du workspace) mais hérite du hook `postPublish` du workspace.

### Comportement

- **`--dry-run`** : les hooks sont affichés mais non exécutés.
- **`--verbose`** : la sortie stdout/stderr des hooks est diffusée en direct. Sinon, la sortie n'est affichée qu'en cas d'échec.
- Les fichiers modifiés par les hooks `postBump` ou `preCommit` sont automatiquement inclus dans le commit de release.

## Exemples complets

### Repo unique

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;$schema&quot;: &quot;https://ferrflow.com/schema/ferrflow.json&quot;,
  &quot;workspace&quot;: {
    &quot;tagTemplate&quot;: &quot;v{version}&quot;
  },
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;ferrflow&quot;,
      &quot;path&quot;: &quot;.&quot;,
      &quot;changelog&quot;: &quot;CHANGELOG.md&quot;,
      &quot;versionedFiles&quot;: [
        { &quot;path&quot;: &quot;Cargo.toml&quot;, &quot;format&quot;: &quot;toml&quot; },
        { &quot;path&quot;: &quot;npm/package.json&quot;, &quot;format&quot;: &quot;json&quot; }
      ]
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
tag_template = &quot;v{version}&quot;

[[package]]
name = &quot;ferrflow&quot;
path = &quot;.&quot;
changelog = &quot;CHANGELOG.md&quot;

[[package.versioned_files]]
path = &quot;Cargo.toml&quot;
format = &quot;toml&quot;

[[package.versioned_files]]
path = &quot;npm/package.json&quot;
format = &quot;json&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  $schema: &quot;https://ferrflow.com/schema/ferrflow.json&quot;,
  workspace: {
    tagTemplate: &quot;v{version}&quot;,
  },
  package: [
    {
      name: &quot;ferrflow&quot;,
      path: &quot;.&quot;,
      changelog: &quot;CHANGELOG.md&quot;,
      versionedFiles: [
        { path: &quot;Cargo.toml&quot;, format: &quot;toml&quot; },
        { path: &quot;npm/package.json&quot;, format: &quot;json&quot; },
      ],
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  tagTemplate: &quot;v{version}&quot;

package:

- name: ferrflow
  path: &quot;.&quot;
  changelog: CHANGELOG.md
  versionedFiles:
  - path: Cargo.toml
    format: toml
  - path: npm/package.json
    format: json
    </code></pre>

</div></div>
</div>

### Monorepo

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;$schema&quot;: &quot;https://ferrflow.com/schema/ferrflow.json&quot;,
  &quot;workspace&quot;: {
    &quot;tagTemplate&quot;: &quot;{name}@v{version}&quot;
  },
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;api&quot;,
      &quot;path&quot;: &quot;packages/api&quot;,
      &quot;changelog&quot;: &quot;packages/api/CHANGELOG.md&quot;,
      &quot;sharedPaths&quot;: [&quot;packages/shared/&quot;],
      &quot;versionedFiles&quot;: [
        { &quot;path&quot;: &quot;packages/api/Cargo.toml&quot;, &quot;format&quot;: &quot;toml&quot; }
      ]
    },
    {
      &quot;name&quot;: &quot;site&quot;,
      &quot;path&quot;: &quot;packages/site&quot;,
      &quot;changelog&quot;: &quot;packages/site/CHANGELOG.md&quot;,
      &quot;sharedPaths&quot;: [&quot;packages/shared/&quot;],
      &quot;versionedFiles&quot;: [
        { &quot;path&quot;: &quot;packages/site/package.json&quot;, &quot;format&quot;: &quot;json&quot; }
      ]
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
tag_template = &quot;{name}@v{version}&quot;

[[package]]
name = &quot;api&quot;
path = &quot;packages/api&quot;
changelog = &quot;packages/api/CHANGELOG.md&quot;
shared_paths = [&quot;packages/shared/&quot;]

[[package.versioned_files]]
path = &quot;packages/api/Cargo.toml&quot;
format = &quot;toml&quot;

[[package]]
name = &quot;site&quot;
path = &quot;packages/site&quot;
changelog = &quot;packages/site/CHANGELOG.md&quot;
shared_paths = [&quot;packages/shared/&quot;]

[[package.versioned_files]]
path = &quot;packages/site/package.json&quot;
format = &quot;json&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  $schema: &quot;https://ferrflow.com/schema/ferrflow.json&quot;,
  workspace: {
    tagTemplate: &quot;{name}@v{version}&quot;,
  },
  package: [
    {
      name: &quot;api&quot;,
      path: &quot;packages/api&quot;,
      changelog: &quot;packages/api/CHANGELOG.md&quot;,
      sharedPaths: [&quot;packages/shared/&quot;],
      versionedFiles: [
        { path: &quot;packages/api/Cargo.toml&quot;, format: &quot;toml&quot; },
      ],
    },
    {
      name: &quot;site&quot;,
      path: &quot;packages/site&quot;,
      changelog: &quot;packages/site/CHANGELOG.md&quot;,
      sharedPaths: [&quot;packages/shared/&quot;],
      versionedFiles: [
        { path: &quot;packages/site/package.json&quot;, format: &quot;json&quot; },
      ],
    },
  ],
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="YAML"><p class="ferr-tab__label">YAML</p><div class="ferr-tab__body"><pre><code class="language-yaml">workspace:
  tagTemplate: &quot;{name}@v{version}&quot;

package:

- name: api
  path: packages/api
  changelog: packages/api/CHANGELOG.md
  sharedPaths:
  - packages/shared/
    versionedFiles:
  - path: packages/api/Cargo.toml
    format: toml

- name: site
  path: packages/site
  changelog: packages/site/CHANGELOG.md
  sharedPaths:
  - packages/shared/
    versionedFiles:
  - path: packages/site/package.json
    format: json
    </code></pre>

</div></div>
</div>

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Exécutez <code>ferrflow init</code> pour générer automatiquement un fichier de configuration basé sur ce que FerrFlow détecte dans votre repo.</p>
</div></aside>
