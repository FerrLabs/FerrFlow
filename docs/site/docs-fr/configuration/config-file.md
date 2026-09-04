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
</div>

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p>Les configurations JSON, JSON5, et TypeScript/JavaScript utilisent des cl\u00e9s en <strong>camelCase</strong> (<code>tagTemplate</code>, <code>versionedFiles</code>).
La configuration TOML utilise des cl\u00e9s en <strong>snake_case</strong> (<code>tag_template</code>, <code>versioned_files</code>).
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

| Champ          | Type           | Description                                                                  |
| -------------- | -------------- | ---------------------------------------------------------------------------- |
| `package`      | string         | Nom du package                                                               |
| `oldVersion`   | string         | Version avant le bump (vide pour la premi\u00e8re release)                   |
| `newVersion`   | string         | Version apr\u00e8s le bump                                                   |
| `bumpType`     | string         | `major`, `minor`, `patch`, ou `none`                                         |
| `tag`          | string         | Nom complet du tag git                                                       |
| `dryRun`       | boolean        | Vrai si `--dry-run` est actif                                                |
| `packagePath`  | string         | Chemin absolu vers la racine du package                                      |
| `channel`      | string ou null | Nom du channel de pr\u00e9-release                                           |
| `isPrerelease` | boolean        | Vrai si c'est une pr\u00e9-release                                           |
| `monorepo`     | boolean        | Vrai si c'est une release monorepo                                           |
| `changelog`    | string         | Section de changelog rendue pour ce bump (markdown)                          |
| `commits`      | array          | `{ hash, message, type?, scope?, breaking }` par commit du bump              |
| `bumpedFiles`  | array          | `{ path, format }` pour chaque fichier modifié par la release                |
| `allPackages`  | array          | `{ name, version, bump }` pour chaque package publié dans ce batch           |
| `releaseUrl`   | string ou null | URL de la release forge créée — hooks `postPublish` uniquement, `null` sinon |

`commits`, `bumpedFiles` et `allPackages` arrivent comme de vrais tableaux (parsés depuis du JSON), vous pouvez donc les itérer directement :

```js
export default {
  workspace: {
    hooks: {
      postBump(ctx) {
        for (const c of ctx.commits) {
          if (c.breaking) console.log(`breaking: ${c.message}`);
        }
      },
    },
  },
};
```

Les hooks sous forme de commandes shell et de fonctions peuvent \u00eatre m\u00e9lang\u00e9s dans la m\u00eame config.

## `workspace`

Paramètres globaux qui s'appliquent à tous les packages.

| Champ                   | Type    | Défaut                                                                      | Description                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ----------------------- | ------- | --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `remote`                | string  | `"origin"`                                                                  | Remote git vers lequel pousser                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `branch`                | string  | auto-détecté                                                                | Branche vers laquelle pousser (détectée depuis le HEAD du remote)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `tagTemplate`           | string  | `"v{version}"` ou `"{name}@v{version}"`                                     | Modèle de nommage des tags. Utilise les placeholders `{version}` et `{name}`. Par défaut `v{version}` pour les repos mono-package et `{name}@v{version}` pour les monorepos.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `latestTag`             | string  | aucun                                                                       | Modèle d'un tag alias flottant qui pointe toujours vers la dernière version non-préversion du package, par exemple `"latest"` ou `"{name}@latest"`. Absent par défaut. Volontairement **non** dérivé de `tagTemplate` : l'alias est un nom, pas une version, donc un `tagTemplate` en `v{version}` donne `latest`, jamais `vlatest`. En monorepo le modèle doit contenir `{name}`, sinon chaque package écrase le même ref et le dernier publié gagne. Les préversions ne le déplacent jamais, et il échappe au garde-fou anti-recul qui s'applique aux tags flottants `major`/`minor`.                                                                                                                                                                                                                                    |
| `versioning`            | string  | `"semver"`                                                                  | Stratégie de versionnage par défaut pour tous les packages                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `releaseCommitMode`     | string  | `"commit"`                                                                  | Gestion du commit de release : `"commit"`, `"pr"` ou `"none"`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `releaseCommitScope`    | string  | `"grouped"`                                                                 | Dans un monorepo où plusieurs packages sont bumpés en même temps, créer un seul commit `"grouped"` ou un commit `"per-package"`. N'a d'effet que quand plusieurs packages bumpent.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `releaseCommitBody`     | string  | `"none"`                                                                    | Ce que contient le corps du commit de release, sous la ligne de sujet. `"none"` conserve le sujet sur une seule ligne. `"summary"` liste une ligne par package publié avec son nombre de commits. `"full"` intègre la section de changelog écrite pour chaque package — en scope `"grouped"`, chaque section est titrée `## <package> <version>`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `forge`                 | string  | `"auto"`                                                                    | Forçage du forge git : `"auto"` détecte depuis l'URL du remote, et pour un hôte non reconnu il sonde l'API en HTTPS pour auto-détecter une instance auto-hébergée de **GitLab**, **GitHub Enterprise** ou **Gitea / Forgejo** (mis en cache, ~2s, best-effort). Renseignez `"github"`, `"gitlab"`, `"gitea"` (Gitea / Forgejo / Codeberg) ou `"bitbucket"` (Bitbucket Cloud) pour forcer un forge — nécessaire uniquement si l'hôte n'est pas joignable en HTTPS ou pour éviter le sondage. L'auth Gitea utilise `GITEA_TOKEN` / `FORGEJO_TOKEN` ; Bitbucket utilise `BITBUCKET_TOKEN`. Tous couvrent la création de release — sur Bitbucket, qui n'a pas d'objet release, la release est le tag annoté que FerrFlow pousse. Le mode PR reste GitHub/GitLab.                                                               |
| `skipCi`                | boolean | dépend du mode                                                              | Ajouter `[skip ci]` aux commits de release. Par défaut `true` en mode `"commit"`, `false` sinon.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `commitSkipMarkers`     | array   | `["[skip ci]", "[ci skip]", "[no ci]", "[skip actions]", "[actions skip]"]` | Marqueurs qui font ignorer un commit par FerrFlow lors du calcul de la prochaine version. Comparaison insensible à la casse, sur la ligne de sujet uniquement.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `commitFormats`         | object  | conventionnel permissif                                                     | Quels sujets de commit correspondent à quel niveau de bump. Chacun de `major` / `minor` / `patch` accepte un motif, une liste de motifs, ou `"all"` comme fourre-tout ; `*` correspond à n'importe quelle suite de caractères (y compris `/`) et `?` à exactement un. Résolution : major → minor → patch, premier motif gagnant. `caseSensitive` (défaut `true`) met les deux côtés en minuscules quand il vaut `false`. Les défauts acceptent aussi les variantes capitalisées et séparées par une barre oblique (`Feat:`, `feat/`, `feature:`, `Fix/`, `Perf:`, `Refactor/`, etc.), listées en entier dans [défauts permissifs](/fr/docs/reference/conventional-commits). Les marqueurs de rupture (`feat!:`, `fix(api)!:`, un pied de page `BREAKING CHANGE:`) sont toujours détectés quelle que soit la configuration. |
| `autoMergeReleases`     | boolean | `true`                                                                      | Activer l'auto-merge sur les PR de release (uniquement en mode `"pr"`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `recoverMissedReleases` | boolean | `false`                                                                     | Comparer les fichiers versionnés au dernier tag plutôt qu'au seul dernier commit, pour rattraper des releases manquées plus tôt dans un monorepo.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `versionSource`         | string  | `"highest"`                                                                 | Quelle source l'emporte quand un package a à la fois un tag git et une version dans un fichier versionné. `"highest"` prend la plus haute, donc une erreur dans l'une ou l'autre fait monter la version sans jamais redescendre. `"tag"` traite les tags comme le registre de ce qui a été publié et ignore le fichier. `"file"` traite le fichier comme la source et ignore le tag, ce dont a besoin un package migré d'un dépôt à un autre. Sans effet si une seule source est présente.                                                                                                                                                                                                                                                                                                                                 |
| `branches`              | array   | `[]`                                                                        | Associe des branches à des canaux de pré-release (voir [Canaux de pré-release](#canaux-de-pré-release)).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `linked`                | array   | `[]`                                                                        | Groupes de packages qui partagent une ligne de version lorsqu'ils sont publiés ensemble. Dès qu'un membre a un commit publiable, tous passent à la même version (la plus haute) (voir [Groupes de versions liées et fixes](/fr/docs/configuration/monorepo#groupes-de-versions-liées-et-fixes)).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `fixed`                 | array   | `[]`                                                                        | Groupes de packages verrouillés sur une version identique en permanence. Comportement de `linked` ; `ferrflow validate` avertit lorsque les versions d'un groupe `fixed` ont divergé.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `anonymous_telemetry`   | boolean | `true`                                                                      | Dépréciée et ignorée — la télémétrie a été retirée en v5.33 ([détails](/fr/v5/docs/legal/telemetry)). La clé (et son alias `telemetry`) reste acceptée pour que les configurations existantes restent valides.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |

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
</div>

### Mode de commit de release

Contrôle la façon dont FerrFlow gère le commit qui met à jour les fichiers de version et les changelogs.

| Mode       | Comportement                                                                           |
| ---------- | -------------------------------------------------------------------------------------- |
| `"commit"` | Commit directement sur la branche courante et pousse (par défaut)                      |
| `"pr"`     | Ouvre une pull request de release persistante et la met à jour à chaque nouveau commit |
| `"none"`   | Crée uniquement les tags et les releases, ne commit pas les changements de fichiers    |

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
</div>

En mode `"pr"`, FerrFlow maintient **une seule PR de release au long cours par branche cible**. Il conserve une branche de release unique — `ferrflow/release-<branche-cible>` — et à chaque nouveau commit il recalcule la version et le changelog puis force-push cette même branche : la PR ouverte est mise à jour sur place au lieu d'en ouvrir une nouvelle par version.

`autoMergeReleases` (par défaut `true`) active l'auto-merge sur cette PR ; il est réappliqué à chaque mise à jour et sans effet quand il est désactivé (la PR attend simplement un humain). Le mode PR est supporté sur GitHub et GitLab.

FerrFlow n'écrase pas le travail que vous poussez sur la branche de release : si la branche porte un commit qu'il n'a pas créé — tout ce qui n'est pas un commit `chore(release):`, comme un correctif de revue que vous avez poussé — il avertit et laisse la branche et la PR intactes pour ce run.

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
</div>

### Canaux de pré-release

Le tableau `branches` associe des noms de branches (ou des motifs glob) à des canaux de pré-release. Quand FerrFlow s'exécute sur une branche correspondant à une entrée, il release sur ce canal — par exemple `1.4.0-beta.1` au lieu de `1.4.0`. C'est cette même association que l'option `--channel` de `ferrflow check` et `ferrflow release` surcharge ponctuellement.

Chaque entrée comporte :

| Champ                  | Type              | Description                                                            |
| ---------------------- | ----------------- | ---------------------------------------------------------------------- |
| `name`                 | string            | Nom de branche ou motif glob (ex. `"main"`, `"release/*"`)             |
| `channel`              | string ou `false` | Nom de canal (`"beta"`, `"rc"`, …), ou `false` pour une release stable |
| `prereleaseIdentifier` | string            | Stratégie de l'identifiant ajouté après le nom du canal                |

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;branches&quot;: [
      { &quot;name&quot;: &quot;main&quot;, &quot;channel&quot;: false },
      { &quot;name&quot;: &quot;next&quot;, &quot;channel&quot;: &quot;beta&quot; }
    ]
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[[workspace.branches]]
name = &quot;main&quot;
channel = false

[[workspace.branches]]
name = &quot;next&quot;
channel = &quot;beta&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    branches: [
      { name: &quot;main&quot;, channel: false },
      { name: &quot;next&quot;, channel: &quot;beta&quot; },
    ],
  },
}
</code></pre>
</div></div>
</div>

### Métadonnées de build

`buildMetadata` désigne une commande dont la sortie standard est ajoutée à la version après un `+`. À utiliser quand une partie de la version vient du code et non des commits : une plage de protocoles supportés, une révision amont vendorisée.

La commande est exécutée une fois par release, depuis la racine du dépôt, avant l'écriture du moindre fichier de version. FerrFlow élague la sortie et exige des caractères alphanumériques et des tirets séparés par des points, seul jeu autorisé par semver après le `+`. La release est interrompue si la commande échoue, n'affiche rien, ou affiche autre chose que ce jeu.

Seuls les fichiers de version portent le suffixe. Le tag, le changelog et le bump suivant conservent la version nue, car semver exclut les métadonnées de build de l'identité d'une version et les ignore dans les comparaisons de précédence. `1.4.0+26.2-26.45` est taggé `v1.4.0`, et le mineur qui suit est calculé à partir de `1.4.0`.

Avec `--dry-run`, la commande est affichée et non exécutée.

Dans un monorepo, la commande s'applique à chaque package publié. Un package peut la remplacer par la sienne, ou poser `buildMetadata = false` pour conserver une version nue pendant que le reste du workspace est estampillé. Chaque commande distincte n'est exécutée qu'une fois, quel que soit le nombre de packages qui la partagent.

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;buildMetadata&quot;: &quot;sh scripts/protocol-versions.sh&quot;
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
buildMetadata = &quot;sh scripts/protocol-versions.sh&quot;
</code></pre>

</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  workspace: {
    buildMetadata: &quot;sh scripts/protocol-versions.sh&quot;,
  },
}
</code></pre>
</div></div>
</div>

## `package`

Définit un package à versionner. Vous pouvez en avoir un ou plusieurs.

| Champ           | Requis | Défaut                | Description                                                 |
| --------------- | ------ | --------------------- | ----------------------------------------------------------- |
| `name`          | oui    | —                     | Identifiant du package, utilisé dans le préfixe du tag git  |
| `path`          | oui    | —                     | Chemin relatif vers le répertoire du package                |
| `changelog`     | non    | `{path}/CHANGELOG.md` | Chemin vers le fichier changelog                            |
| `sharedPaths`   | non    | `[]`                  | Chemins qui déclenchent ce package lorsqu'ils sont modifiés |
| `versioning`    | non    | hérité du workspace   | Surcharger la stratégie de versionnage pour ce package      |
| `tagTemplate`   | non    | hérité du workspace   | Surcharger le modèle de tag pour ce package                 |
| `latestTag`     | non    | hérité du workspace   | Surcharger le tag alias flottant pour ce package            |
| `buildMetadata` | non    | hérité du workspace   | Surcharger la commande de métadonnées, ou `false` pour une version nue |
| `versionSource` | non    | hérité du workspace   | Surcharger la résolution tag / fichier pour ce package      |

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

### Packages versionnés par tag uniquement

`versionedFiles` est optionnel. Omettez-le (ou mettez-le à `[]`) pour les packages dont la version est communiquée entièrement via les tags git et les GitHub Releases — modules Go, images Docker, GitHub Actions, dépôts d'infrastructure.

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;package&quot;: [
    {
      &quot;name&quot;: &quot;my-action&quot;,
      &quot;path&quot;: &quot;.&quot;,
      &quot;versionedFiles&quot;: []
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[[package]]
name = &quot;my-action&quot;
path = &quot;.&quot;
versioned_files = []
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  package: [
    {
      name: &quot;my-action&quot;,
      path: &quot;.&quot;,
      versionedFiles: [],
    },
  ],
}
</code></pre>
</div></div>
</div>

FerrFlow lit la version courante depuis le dernier tag git correspondant, calcule le prochain bump à partir des conventional commits, puis crée le tag, la GitHub Release, le changelog et les floating tags éventuels — sans toucher au moindre fichier source. Les hooks s'exécutent normalement, vous pouvez donc lancer `docker build`, `docker push` ou `gh release upload` depuis `postPublish` en utilisant `FERRFLOW_NEW_VERSION`.

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p>Avant la v5.1, les packages sans <code>versionedFiles</code> étaient silencieusement ignorés. Si vous vous appuyiez sur ce comportement pour exclure un package d&#39;une release, retirez-le plutôt de la config.</p>
</div></aside>

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
génération du changelog
  ↓
post_bump       ← modifier d'autres fichiers, ou réécrire le changelog qui vient d'être généré
  ↓
pre_commit      ← vérifier les changements stagés, lancer les linters
  ↓
git commit
  ↓
post_commit     ← réagir au commit de release
  ↓
pre_tag         ← smoke-test de l'arbre bumpé avant la pose du tag
  ↓
git tag
  ↓
post_tag        ← cargo publish avant le push (récupérable en cas d'échec)
  ↓
pre_publish     ← lancer les tests sur le commit taggé, builder les artefacts
  ↓
git push + création de la release
  ↓
post_publish    ← pousser les images Docker, notifier Slack, publier les packages

pre_release     ← (mode PR) après l'ouverture de la PR de release, avant le merge
on_success      ← une fois, après une release entièrement réussie
on_error        ← une fois, quand la release échoue ($FERRFLOW_ERROR_CODE)
```

### Réécrire le changelog depuis un hook

`post_bump` s'exécute après la génération et l'écriture de la section de changelog, et la reçoit dans `FERRFLOW_CHANGELOG`. Un hook peut réécrire `CHANGELOG.md` et FerrFlow reprend la modification : le fichier réécrit est commité, et c'est aussi lui qui alimente le tag git, le corps de la release sur la forge et le commit de release.

Cela suffit à transformer des sujets de commit en prose sans que FerrFlow ait à savoir comment vous vous y prenez :

```bash
#!/bin/sh
# Lit la section générée depuis $FERRFLOW_CHANGELOG, réécrit la prose dans
# CHANGELOG.md. N'importe quel outil convient, y compris aucun.
votre-reecrivain --input "$FERRFLOW_CHANGELOG" --write CHANGELOG.md
```

```json
{ "workspace": { "hooks": { "postBump": "sh ./scripts/prose.sh" } } }
```

Deux points à connaître. Si la réécriture perd le titre `## [version]`, FerrFlow retombe sur le texte généré plutôt que de publier des notes de release vides. Et rien de tout cela ne s'exécute sous `--dry-run`, où aucun changelog n'est écrit : prévisualisez donc le résultat par une vraie release sur une branche plutôt que d'attendre que `--dry-run` vous le montre.

La reproductibilité est à votre charge. Le changelog est commité et tagué, donc ce que produit le hook est définitif. Un réécrivain qui donne une réponse différente à chaque exécution rend les releases non reproductibles.

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
</div>

| Champ         | Type   | Défaut    | Description                                                                                           |
| ------------- | ------ | --------- | ----------------------------------------------------------------------------------------------------- |
| `preBump`     | string | —         | Exécuté après le calcul du bump, avant l'écriture des fichiers de version                             |
| `postBump`    | string | —         | Exécuté après l'écriture des fichiers de version                                                      |
| `preCommit`   | string | —         | Exécuté après le changelog, avant le commit git                                                       |
| `postCommit`  | string | —         | Exécuté après le commit de release, avant le tag                                                      |
| `preTag`      | string | —         | Exécuté après le commit, juste avant `git tag`                                                        |
| `postTag`     | string | —         | Exécuté après la création des tags, avant le push                                                     |
| `prePublish`  | string | —         | Exécuté après le commit+tag, avant le push                                                            |
| `postPublish` | string | —         | Exécuté après le push et la création de la release                                                    |
| `preRelease`  | string | —         | Mode PR uniquement : après l'ouverture de la PR de release, avant le merge (une fois)                 |
| `onSuccess`   | string | —         | Exécuté une fois après une release entièrement réussie                                                |
| `onError`     | string | —         | Exécuté une fois quand la release échoue ; définit `FERRFLOW_ERROR_CODE` (une fois)                   |
| `onFailure`   | string | `"abort"` | Stratégie — `"abort"` annule la release en cas d'échec de hook, `"continue"` affiche un avertissement |

### Variables d'environnement

Chaque hook reçoit ces variables :

| Variable                     | Description                                                     | Exemple                                                                |
| ---------------------------- | --------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `FERRFLOW_PACKAGE`           | Nom du package                                                  | `api`                                                                  |
| `FERRFLOW_OLD_VERSION`       | Version avant le bump (vide pour la première release)           | `1.2.3`                                                                |
| `FERRFLOW_NEW_VERSION`       | Version après le bump                                           | `1.3.0`                                                                |
| `FERRFLOW_BUMP_TYPE`         | `major`, `minor`, `patch` ou `none`                             | `minor`                                                                |
| `FERRFLOW_TAG`               | Nom complet du tag git                                          | `api@v1.3.0`                                                           |
| `FERRFLOW_DRY_RUN`           | `true` si `--dry-run` est activé                                | `false`                                                                |
| `FERRFLOW_PACKAGE_PATH`      | Chemin absolu vers la racine du package                         | `/home/user/repo/packages/api`                                         |
| `FERRFLOW_IS_PRERELEASE`     | `true` sur un canal de pré-release                              | `false`                                                                |
| `FERRFLOW_MONOREPO`          | `true` sur une release monorepo                                 | `false`                                                                |
| `FERRFLOW_CHANGELOG`         | Section de changelog rendue pour ce bump                        | `### Features\n- ...`                                                  |
| `FERRFLOW_COMMITS_JSON`      | Tableau JSON de `{ hash, message, type?, scope?, breaking }`    | `[{"hash":"a1b2","message":"feat: x","type":"feat","breaking":false}]` |
| `FERRFLOW_BUMPED_FILES_JSON` | Tableau JSON de `{ path, format }` modifiés par la release      | `[{"path":"package.json","format":"json"}]`                            |
| `FERRFLOW_ALL_PACKAGES_JSON` | Tableau JSON de `{ name, version, bump }` publiés dans ce batch | `[{"name":"api","version":"1.3.0","bump":"minor"}]`                    |
| `FERRFLOW_RELEASE_URL`       | URL de la release forge créée (`postPublish` uniquement)        | `https://github.com/acme/api/releases/tag/v1.3.0`                      |
| `FERRFLOW_ERROR_CODE`        | Code d'erreur, défini uniquement pour `onError`                 | `E2005`                                                                |

`FERRFLOW_COMMITS_JSON`, `FERRFLOW_BUMPED_FILES_JSON` et `FERRFLOW_ALL_PACKAGES_JSON` sont des chaînes JSON — passez-les dans `jq` depuis vos hooks shell.

Pour les hooks exécutés une seule fois par run (`preRelease`, `onSuccess`, `onError`), les variables par package sont vides et `FERRFLOW_TAG` contient tous les tags publiés séparés par des virgules.

`onFailure` est la **stratégie** d'échec (`abort` / `continue`), pas une commande. La commande exécutée _quand_ une release échoue est `onError`, qui reçoit le `FERRFLOW_ERROR_CODE` fautif.

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
</div>

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Exécutez <code>ferrflow init</code> pour générer automatiquement un fichier de configuration basé sur ce que FerrFlow détecte dans votre repo.</p>
</div></aside>
