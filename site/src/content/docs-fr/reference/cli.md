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

## `ferrflow completions`

Générer un script de complétion shell et l'afficher sur la sortie standard.

```bash
ferrflow completions <SHELL>
```

`<SHELL>` est l'un de `bash`, `elvish`, `fish`, `powershell` ou `zsh`.

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
