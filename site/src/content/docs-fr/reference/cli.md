---
title: Commandes CLI
description: Référence complète de toutes les commandes et options du CLI FerrFlow.
---

## `ferrflow release`

Lance le pipeline complet de release : bump des versions, mise à jour des changelogs, commit, tag, push et création de la release.

```bash
ferrflow release [OPTIONS]
```

| Option                      | Description                                                                                                                                                                                 |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--force`                   | Autoriser les floating tags à reculer vers une version inférieure                                                                                                                           |
| `--force-version <VERSION>` | Forcer une version spécifique, sans analyser les commits. Format : `VERSION` (repo simple) ou `NAME@VERSION` (monorepo)                                                                     |
| `--channel <NAME>`          | Canal de pré-release à utiliser (ex. `beta`, `rc`, `dev`)                                                                                                                                   |
| `--draft`                   | Créer les releases en brouillon (GitHub uniquement). Un `ferrflow release` ultérieur sans `--draft` détecte et publie automatiquement les brouillons existants                              |
| `--force-unlock`            | Forcer la levée d'un verrou `.git/ferrflow.lock` existant. À n'utiliser que si aucun autre `ferrflow release` n'est en cours — par exemple après un crash ayant laissé le fichier de verrou |

**Ce que ça fait :**

1. Scanne les commits depuis le dernier tag pour chaque package
2. Détermine l'incrément de version à partir des Conventional Commits
3. Met à jour tous les `versionedFiles` avec la nouvelle version
4. Ajoute la nouvelle section au `CHANGELOG.md`
5. Crée un commit git, ouvre une PR, ou passe (selon `releaseCommitMode`)
6. Crée et pousse le tag git
7. Crée une release GitHub/GitLab avec le changelog comme notes

---

## `ferrflow check`

Prévisualiser ce que `ferrflow release` ferait sans effectuer de changements.

```bash
ferrflow check [OPTIONS]
```

| Option             | Description                                                     |
| ------------------ | --------------------------------------------------------------- |
| `--json`           | Sortie au format JSON                                           |
| `--channel <NAME>` | Canal de pré-release à utiliser (ex. `beta`, `rc`, `dev`)       |
| `--comment`        | Poster un commentaire de prévisualisation sur la PR/MR courante |

---

## `ferrflow publish`

Exécuter les [publishers](/fr/docs/configuration/config-file/#publishers) configurés pour la version actuellement publiée de chaque package — sans bumper, committer ni tagger. `ferrflow release` exécute déjà vos publishers à la fin d'une release ; `ferrflow publish` sert lorsque vous préférez les exécuter dans un **job CI séparé** disposant de la toolchain de build et de l'authentification registre dont les publishers ont besoin (docker buildx, helm, un `dist/` compilé, …) — ce que votre job de release n'a pas forcément.

```bash
ferrflow publish [PACKAGES...]
```

| Argument / option | Description                                                                                                                                                                                                   |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `[PACKAGES...]`   | Publier ces packages par leur nom (séparés par des espaces). Omettre pour auto-détecter depuis le tag déclencheur (`GITHUB_REF` / `CI_COMMIT_TAG`), avec repli sur chaque package qui déclare des publishers. |
| `--all`, `-a`     | Publier tous les packages, en ignorant tout scope de tag déclencheur.                                                                                                                                         |

Il lit la version actuelle de chaque package depuis ses `versionedFiles` (ou le dernier tag correspondant pour les packages tag-only), donc à exécuter **après** que `ferrflow release` a coupé la version. Les publishers sont idempotents : tout ce qui est déjà sur le registre est ignoré, donc une ré-exécution est sûre. Utilisez l'option globale `--dry-run` pour prévisualiser sans publier.

**Résolution du scope.** Sans argument, si le run a été déclenché par un tag de package (ex. `api@v2.2.1`), seul ce package est publié — un seul workflow déclenché par tag publie ainsi chaque package sur son propre tag, sans câblage par package. Sans tag correspondant (par exemple la ref de branche du job de release), tous les packages sont publiés, comme avant. Passez des noms de packages pour cibler un sous-ensemble, ou `--all` pour forcer tous les packages même sous un tag.

L'Action GitHub l'expose via `mode: publish` — elle installe le binaire et exécute `ferrflow publish` pour vous, en se scopant automatiquement au tag déclencheur (ou passez l'input `package` pour forcer). Un job déclenché par le tag n'a plus qu'à mettre en place la toolchain dont ses publishers ont besoin :

```yaml title=".github/workflows/publish.yml"
on:
  push:
    # `v*` pour les repos mono-package ; `*@v*` pour les tags par-package en monorepo
    tags: ['v*', '*@v*']
jobs:
  publish:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v6
      - uses: docker/setup-buildx-action@v4
      - uses: docker/login-action@v4
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: FerrLabs/FerrFlow@v5
        with:
          mode: publish
```

---

## `ferrflow changelog`

Générer ou mettre à jour `CHANGELOG.md` uniquement, sans bumper les versions ni créer de tags.

```bash
ferrflow changelog
```

Ne prend aucune option spécifique. Utilisez l'option globale `--dry-run` pour afficher l'entrée sans l'écrire.

---

## `ferrflow init`

Générer un fichier de configuration pour le repository courant. Détecte les fichiers de version existants (`Cargo.toml`, `package.json`, etc.) et génère la configuration appropriée.

```bash
ferrflow init [OPTIONS]
```

| Option              | Description                                                    |
| ------------------- | -------------------------------------------------------------- |
| `--format <FORMAT>` | Format du fichier de configuration : `json`, `json5` ou `toml` |

---

## `ferrflow migrate`

Générer une configuration FerrFlow à partir de celle d'un autre outil de release. Lancez cette commande dans votre repo et elle écrit le `ferrflow.json` équivalent.

```bash
ferrflow migrate [OPTIONS]
```

| Option           | Description                                                                                            |
| ---------------- | ------------------------------------------------------------------------------------------------------ |
| `--from <OUTIL>` | Source : `semantic-release`, `changesets`, `release-please`, `standard-version`. Auto-détecté si omis. |

### Sources

| Outil              | Lit                             | Ce qui est converti (extraits)                                                                                                                                                           |
| ------------------ | ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `semantic-release` | `.releaserc`, `.releaserc.json` | `tagFormat` → `tagTemplate` ; `branches` → canaux ; `@semantic-release/exec` → `hooks` ; plugins `changelog` / `github` / `gitlab` (voir la table ci-dessous)                            |
| `release-please`   | `release-please-config.json`    | la map `packages` → packages FerrFlow (le `release-type` de chaque package → le bon fichier de version) ; `include-component-in-tag` → `tagTemplate` ; flux PR → `releaseCommitMode: pr` |
| `standard-version` | `.versionrc`, `.versionrc.json` | `tagPrefix` → `tagTemplate` ; `bumpFiles` / `packageFiles` → `versionedFiles`                                                                                                            |
| `changesets`       | `.changeset/config.json`        | `baseBranch` → `branch` ; `linked` / `fixed` → groupes de versions (voir la note)                                                                                                        |

Mapping des plugins semantic-release :

| semantic-release                      | FerrFlow                                                                                                                                                  |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tagFormat: "v${version}"`            | `tagTemplate: "v{{version}}"`                                                                                                                             |
| `branches`                            | `branches` — `main`/`master` deviennent la ligne stable, une branche `prerelease: true` (ou nommée) devient un canal                                      |
| `@semantic-release/changelog`         | le chemin `changelog` du package                                                                                                                          |
| `@semantic-release/exec`              | `hooks` (`prepareCmd` → `preBump`, `publishCmd` → `postPublish`, `successCmd` → `onSuccess`, `failCmd` → `onError`, `verifyConditionsCmd` → `preRelease`) |
| `@semantic-release/github` / `gitlab` | `forge`                                                                                                                                                   |

Tout ce qui n'a pas d'équivalent FerrFlow est **signalé, jamais deviné**. Chaque exécution affiche ce qui a été converti, ignoré, et ce qui demande une revue manuelle — par exemple `@semantic-release/npm` (configurez `publishers` à la main), des règles `commit-analyzer` personnalisées (les règles de bump de FerrFlow sont fixes), et `repositoryUrl` (FerrFlow déduit le remote depuis git). Elle n'écrase pas une configuration FerrFlow existante.

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p><strong>changesets.</strong> changesets versionne à partir de fichiers <code>.changeset/*.md</code> écrits à la main, alors que FerrFlow versionne depuis les commits conventionnels — après migration, adoptez les Conventional Commits, vos fichiers changeset existants ne sont pas lus. FerrFlow lit votre déclaration de workspace (<code>workspaces</code> dans <code>package.json</code>, ou <code>pnpm-workspace.yaml</code>) et génère une entrée <code>package</code> par package découvert : vos groupes <code>linked</code>/<code>fixed</code> référencent donc déjà de vrais packages et la config migrée valide telle quelle. Un dépôt sans déclaration de workspace obtient un seul package racine.</p>
</div></aside>

```bash
ferrflow migrate                        # auto-détection
ferrflow migrate --from release-please
```

Les configurations source JSON, YAML et JavaScript fonctionnent toutes — une configuration JavaScript (`.releaserc.js`, `release.config.js`, `.versionrc.js`) est évaluée avec `node` (Node.js doit donc être dans le PATH), et une configuration YAML (`.releaserc.yaml`, `.versionrc.yaml`) est parsée directement. Après migration, relisez la configuration générée, puis lancez `ferrflow validate` et `ferrflow check`.

---

## `ferrflow status`

Afficher la version actuelle de chaque package et si une release serait déclenchée.

```bash
ferrflow status [OPTIONS]
```

| Option              | Description                                  |
| ------------------- | -------------------------------------------- |
| `--output <FORMAT>` | Format de sortie : `text` (défaut) ou `json` |

Exemple de sortie :

```
api    1.2.3   minor bump pending (1 feat commit)
site   0.4.1   no release (only chore commits)
```

---

## `ferrflow diff`

Comparer deux versions d'un package : les commits qui y sont entrés, l'incrément de chaque commit, les fichiers modifiés, et le changelog que FerrFlow générerait pour l'intervalle. Pratique pour auditer une release, comprendre pourquoi une version a bumpé ainsi, ou rédiger des notes de release a posteriori pour un intervalle.

```bash
ferrflow diff [PACKAGE] <FROM>..<TO> [--json]
```

| Argument / option | Description                                                                                                  |
| ----------------- | ------------------------------------------------------------------------------------------------------------ |
| `<FROM>..<TO>`    | L'intervalle de versions. Chaque borne est un tag ou une version — `v1.4.0`, ou un tag complet `api@v1.6.0`. |
| `[PACKAGE]`       | Nom du package — requis en monorepo, optionnel (et déduit) dans un repo mono-package.                        |
| `--json`          | Émettre la comparaison en objet JSON structuré au lieu de la vue humaine.                                    |

Chaque borne est résolue en essayant d'abord la chaîne comme tag (un vrai tag, ou `v1.4.0` en mono-package), puis comme le tag du package pour cette version (`api@v1.4.0`).

```bash
ferrflow diff v1.4.0..v1.6.0            # repo mono-package
ferrflow diff api v1.4.0..v1.6.0        # monorepo — nommez le package
```

La sortie liste chaque commit de l'intervalle avec son incrément (`major` / `minor` / `patch` / `none`), met en évidence les breaking changes, résume les fichiers modifiés, et rend la section de changelog pour l'intervalle. En monorepo, l'intervalle couvre pour l'instant tous les commits entre les deux tags (pas encore restreint aux chemins du package nommé).

---

## `ferrflow version`

Afficher la version actuelle d'un ou de tous les packages. Utile dans les scripts CI.

```bash
ferrflow version [PACKAGE] [OPTIONS]
```

| Option   | Description           |
| -------- | --------------------- |
| `--json` | Sortie au format JSON |

Retourne la version depuis le dernier tag git correspondant au modèle de tag du package.

---

## `ferrflow tag`

Afficher le dernier tag pour un ou tous les packages.

```bash
ferrflow tag [PACKAGE] [OPTIONS]
```

| Option   | Description           |
| -------- | --------------------- |
| `--json` | Sortie au format JSON |

---

## `ferrflow validate`

Valider la configuration et les fichiers versionnés qu'elle référence, sans rien bumper. Utilisez `--repo` pour valider un dépôt distant plutôt que l'arbre de travail.

```bash
ferrflow validate [OPTIONS]
```

| Option          | Description                                                                       |
| --------------- | --------------------------------------------------------------------------------- |
| `--json`        | Sortie au format JSON                                                             |
| `--repo <REPO>` | Dépôt distant à valider (ex. `owner/repo` pour GitHub, ou `gitlab:group/project`) |
| `--ref <REF>`   | Ref git pour la validation distante (branche, tag ou commit)                      |

---

## `ferrflow doctor`

Lancer un diagnostic en lecture seule sur le dépôt, la configuration et la forge, puis afficher un rapport par catégories — la commande « est-ce que ma config est saine ? ». Utilisez-la sur un checkout tout neuf pour voir ce qui manque avant la première release, ou quand une exécution se comporte mal et que vous devriez sinon scruter les logs `--verbose`.

```bash
ferrflow doctor [OPTIONS]
```

| Option           | Description                                                                    |
| ---------------- | ------------------------------------------------------------------------------ |
| `--format <FMT>` | `human` (défaut) ou `json`                                                     |
| `--online`       | Sonder aussi l'API de la forge (rate limit / auth GitHub) ; nécessite un token |

Le rapport groupe les vérifications en cinq sections — **Repo** (dépôt git, historique de commits, arbre de travail propre, remote, tags), **Config** (quel fichier de config l'emporte, s'il parse, plus toute la suite de vérifications de `ferrflow validate`), **Versioning** (stratégie et version sur disque de chaque package), **Forge** (forge détectée et présence d'un token d'auth dans l'environnement) et **CI** (fichiers de workflow, et si un workflow épingle l'action `FerrLabs/FerrFlow`). Chaque vérification est verte, un avertissement, ou une erreur.

Le code de sortie est scriptable : `0` quand tout est vert, `1` s'il n'y a que des avertissements, `2` si une vérification est en erreur. La sortie `--format json` a une forme stable — `{ status, exit_code, sections: [{ title, checks: [{ name, status, detail }] }] }` — pour que la CI puisse s'appuyer dessus.

```bash
ferrflow doctor                 # rapport lisible
ferrflow doctor --format json   # lisible par machine, stable pour la CI
ferrflow doctor --online        # vérifie aussi le rate limit de l'API GitHub
```

---

## `ferrflow completions`

Générer un script de complétion shell et l'afficher sur la sortie standard.

```bash
ferrflow completions <SHELL>
```

`<SHELL>` est l'un de `bash`, `elvish`, `fish`, `powershell` ou `zsh`.

---

## `ferrflow schema`

Afficher le schéma JSON du fichier de configuration ferrflow. Le schéma est embarqué dans le binaire : la commande fonctionne donc hors ligne, sans appel réseau à `ferrflow.com/schema/ferrflow.json`.

```bash
ferrflow schema [OPTIONS]
```

| Option            | Description                                                      |
| ----------------- | ---------------------------------------------------------------- |
| `--pretty`        | Formater la sortie au lieu d'un JSON compact sur une seule ligne |
| `--output <FILE>` | Écrire dans un fichier plutôt que sur la sortie standard         |

Utilisez-la pour pointer un éditeur vers une copie locale, ou pour valider `.ferrflow.json` dans un hook pre-commit sans accès internet :

```bash
ferrflow schema --pretty --output ferrflow.schema.json
```

Puis renseignez `"$schema": "./ferrflow.schema.json"` dans votre configuration. La commande parse le schéma embarqué avant de l'afficher : elle sort donc avec un code non nul si l'artefact de build est corrompu.

---

## Options globales

Ces options fonctionnent avec toutes les commandes :

| Option                  | Description                                                                                                                                                                                                               |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--dry-run`             | Montrer ce qui se passerait sans effectuer de changements                                                                                                                                                                 |
| `--verbose`, `-v`       | Sortie détaillée, incluant les hashes de commits et les diffs de fichiers                                                                                                                                                 |
| `--log-format <FORMAT>` | Format de la sortie de diagnostic sur stderr : `human` (défaut, coloré) ou `json` (un événement structuré par ligne). Les **données** des commandes (`--json`, valeurs de `version` / `tag`) restent toujours sur stdout. |
| `--config <PATH>`       | Chemin vers un fichier de configuration personnalisé (défaut : auto-détecté). Accepte aussi la variable d'environnement `FERRFLOW_CONFIG`.                                                                                |
| `--jobs <N>`            | Nombre max de threads pour le travail CPU-parallèle (planification par paquet). Défaut : tous les cœurs logiques ; `1` force le mono-thread. Accepte aussi la variable d'environnement `FERRFLOW_JOBS`.                   |
| `--version`             | Afficher la version de FerrFlow et quitter                                                                                                                                                                                |
| `--help`, `-h`          | Afficher l'aide                                                                                                                                                                                                           |

## Logging & sortie

FerrFlow sépare les **données** des **logs** sur les deux flux de sortie :

- **stdout** porte les données — la sortie `--json` de `check` / `release` / `status` / `validate`, et la valeur affichée par `version` et `tag`. Capturez-la dans vos scripts : `V=$(ferrflow version)`.
- **stderr** porte le rapport humain et chaque événement de diagnostic.

Vous pouvez ainsi capturer le résultat machine et le journal d'exécution indépendamment :

```bash
ferrflow check --json > result.json 2> run.log
```

`--log-format json` rend chaque diagnostic sous forme d'un événement JSON structuré par ligne sur stderr, prêt pour Datadog / Loki / CloudWatch :

```json
{
  "timestamp": "2026-01-01T00:00:00Z",
  "level": "INFO",
  "fields": { "message": "✓ Updated CHANGELOG.md" },
  "target": "ferrflow::changelog"
}
```

`--verbose` (ou un filtre `RUST_LOG` comme `RUST_LOG=ferrflow::git=trace`) contrôle les niveaux affichés.
