---
title: Conventional Commits
description: Comment FerrFlow interprète les messages de commit pour déterminer les incréments de version.
---

FerrFlow suit la spécification [Conventional Commits](https://www.conventionalcommits.org/) pour déterminer de combien incrémenter la version.

## Règles d'incrément

| Type de commit                | Incrément de version | Exemple                             |
| ----------------------------- | -------------------- | ----------------------------------- |
| `feat:`                       | **minor**            | `feat: add wallet subscriptions`    |
| `fix:`                        | patch                | `fix: correct pagination offset`    |
| `perf:`                       | patch                | `perf: cache user queries`          |
| `refactor:`                   | patch                | `refactor: extract auth middleware` |
| `feat!:` ou `BREAKING CHANGE` | **major**            | `feat!: remove deprecated endpoint` |
| `chore:`                      | aucun                | `chore: update dependencies`        |
| `docs:`                       | aucun                | `docs: update README`               |
| `ci:`                         | aucun                | `ci: add linting step`              |
| `style:`                      | aucun                | `style: format code`                |
| `test:`                       | aucun                | `test: add unit tests`              |

## Breaking changes

Un breaking change peut être indiqué de deux manières :

**Suffixe point d'exclamation :**

```
feat!: remove the /v1/users endpoint
fix!: change authentication header format
```

**Footer `BREAKING CHANGE` :**

```
feat: redesign the API

BREAKING CHANGE: The /v1/users endpoint has been removed. Use /v2/users instead.
```

Les deux produisent un incrément **major**.

### Variantes de footer acceptées

FerrFlow reconnaît les orthographes courantes du footer, pas seulement la forme stricte de la spec :

| Footer                                      | Détecté                     |
| ------------------------------------------- | --------------------------- |
| `BREAKING CHANGE: …`                        | oui (spec)                  |
| `BREAKING-CHANGE: …`                        | oui (synonyme de la spec)   |
| `breaking-change: …` / `breaking change: …` | oui (insensible à la casse) |
| `Breaking Change: …`                        | oui (insensible à la casse) |

Il traite aussi un `!` placé **à l'intérieur** du scope — `feat(api!):`, une faute de frappe courante pour `feat(api)!:` — comme un marqueur breaking. Le footer peut se trouver après n'importe quel nombre de paragraphes de corps.

### Ce qui n'est _pas_ un breaking change

La détection reste stricte, pour qu'une mention de passage ne déclenche jamais un incrément major accidentel :

- Le footer doit débuter une ligne, utiliser une seule espace ou un tiret (`BREAKING CHANGE` / `BREAKING-CHANGE`), et être suivi d'un deux-points **et d'une espace**. `BREAKING CHANGE:sans-espace` et le pluriel `BREAKING CHANGES:` sont ignorés.
- Une mention en prose au milieu d'une ligne — « ceci corrige un breaking change dans le parseur » — n'est pas un footer.
- Un `!` qui n'est pas immédiatement avant la parenthèse fermante — `feat(a!b):` — n'est pas un marqueur.

## Scope

Les scopes sont optionnels et ignorés pour le calcul de l'incrément. Ils sont utiles pour la lisibilité :

```
feat(auth): add OAuth2 support       → incrément minor
fix(db): correct index on user table  → incrément patch
```

## Pas de release

Les commits de type `chore`, `docs`, `ci`, `style` ou `test` ne déclenchent pas de release. Si tous les commits depuis le dernier tag sont de ces types, FerrFlow se termine sans créer de nouvelle version.

## Commits multiples

Lorsque plusieurs commits sont présents depuis le dernier tag, FerrFlow prend l'incrément le **plus élevé** parmi tous :

```
fix: correct typo       → patch
feat: add export button → minor   ← gagne
chore: lint             → aucun
```

Résultat : incrément **minor**.
