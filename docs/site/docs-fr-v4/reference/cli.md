---
title: Commandes CLI
description: Référence complète de toutes les commandes et options du CLI FerrFlow.
---

## `ferrflow release`

Lance le pipeline complet de release : bump des versions, mise à jour des changelogs, commit, tag, push et création de la release.

```bash
ferrflow release [OPTIONS]
```

| Option                      | Description                                                                                                             |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `--dry-run`                 | Prévisualiser tous les changements sans écrire, committer ou pousser                                                    |
| `--force`                   | Autoriser les floating tags à reculer vers une version inférieure                                                       |
| `--force-version <VERSION>` | Forcer une version spécifique, sans analyser les commits. Format : `VERSION` (repo simple) ou `NAME@VERSION` (monorepo) |
| `--verbose`, `-v`           | Afficher la sortie détaillée incluant les hashes de commits et les diffs de fichiers                                    |

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

Prévisualiser ce que `ferrflow release` ferait sans effectuer de changements. Équivalent à `ferrflow release --dry-run`.

```bash
ferrflow check
```

---

## `ferrflow changelog`

Générer ou mettre à jour `CHANGELOG.md` uniquement, sans bumper les versions ni créer de tags.

```bash
ferrflow changelog [OPTIONS]
```

| Option      | Description                                              |
| ----------- | -------------------------------------------------------- |
| `--dry-run` | Afficher l'entrée du changelog sans écrire sur le disque |

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

## Options globales

Ces options fonctionnent avec toutes les commandes :

| Option            | Description                                                                                                                                |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `--config <PATH>` | Chemin vers un fichier de configuration personnalisé (défaut : auto-détecté). Accepte aussi la variable d'environnement `FERRFLOW_CONFIG`. |
| `--version`       | Afficher la version de FerrFlow et quitter                                                                                                 |
| `--help`, `-h`    | Afficher l'aide                                                                                                                            |
