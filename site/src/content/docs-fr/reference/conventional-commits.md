---
title: Conventional Commits
description: Comment FerrFlow interprète les messages de commit pour déterminer les incréments de version.
---

FerrFlow suit la spécification [Conventional Commits](https://www.conventionalcommits.org/fr/v1.0.0/) pour déterminer de combien incrémenter la version.

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

## Défauts permissifs

Le tableau ci-dessus liste les formes canoniques, mais FerrFlow ne les impose pas. Les défauts acceptent aussi les variantes capitalisées et séparées par une barre oblique, pour qu'un dépôt qui n'a jamais appliqué la spec obtienne quand même des incréments corrects depuis son historique existant :

| Incrément | Également acceptés par défaut                                                                        |
| --------- | ---------------------------------------------------------------------------------------------------- |
| mineur    | `Feat:`, `feature:`, et les formes à barre oblique `feat/`, `Feat/`, `feature/`, `Feature/`          |
| patch     | `Fix:`, `Perf:`, `Refactor:`, et les formes à barre oblique `fix/`, `Fix/`, `refactor/`, `Refactor/` |

Les formes à deux-points acceptent un scope optionnel, donc `Feat(api):` et `Refactor(db):` sont couverts aussi. Les formes à barre oblique non : `Feat/add-login` correspond, `Feat(api)/add-login` non. Il n'existe pas de forme `perf/`, ni de `Feature:` sans barre oblique.

Rien ne produit un incrément **majeur** par défaut : un majeur vient uniquement d'un marqueur de breaking change, décrit ci-dessous.

Si votre historique suit ses propres conventions, remappez-les avec [`commitFormats`](/fr/docs/configuration/config-file/). Les marqueurs de breaking change sont détectés avant toute consultation des motifs configurés, donc un motif personnalisé ne peut jamais retirer à un commit son statut de breaking change.

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

Il traite aussi un `!` placé **à l'intérieur** du scope (`feat(api!):`, une faute de frappe courante pour `feat(api)!:`) comme un marqueur breaking. Le footer peut se trouver après n'importe quel nombre de paragraphes de corps.

### Ce qui n'est _pas_ un breaking change

La détection reste stricte, pour qu'une mention de passage ne déclenche jamais un incrément major accidentel :

- Le footer doit débuter une ligne, utiliser une seule espace ou un tiret (`BREAKING CHANGE` / `BREAKING-CHANGE`), et être suivi d'un deux-points **et d'une espace**. `BREAKING CHANGE:sans-espace` et le pluriel `BREAKING CHANGES:` sont ignorés.
- Une mention en prose au milieu d'une ligne (« ceci corrige un breaking change dans le parseur ») n'est pas un footer.
- Un `!` qui n'est pas immédiatement avant la parenthèse fermante (`feat(a!b):`) n'est pas un marqueur.

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
