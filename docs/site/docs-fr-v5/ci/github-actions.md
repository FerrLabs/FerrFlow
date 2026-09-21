---
title: GitHub Actions
description: Lancer les releases FerrFlow automatiquement avec GitHub Actions.
---

## Utiliser l'action officielle

La manière la plus simple d'utiliser FerrFlow dans GitHub Actions est l'action `FerrLabs/ferrflow@v5`. Elle installe le binaire et exécute `ferrflow release` automatiquement.

```yaml
name: Release

on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write # requis pour pousser les tags et créer les releases
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0 # historique complet nécessaire pour le scan des commits
          token: ${{ secrets.GITHUB_TOKEN }}

      - uses: FerrLabs/ferrflow@v5
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

<aside class="ferr-aside ferr-aside--warning"><div class="ferr-aside__body"><p><code>fetch-depth: 0</code> est requis. Sans cela, FerrFlow ne peut pas trouver les tags précédents et traitera chaque commit comme nouveau.</p>
</div></aside>

## Permissions

FerrFlow a besoin de `contents: write` pour :

- Pousser les commits de bump de version
- Créer et pousser les tags git
- Créer les GitHub Releases

Si votre repository a des règles de protection de branche, créez un token dédié avec les permissions nécessaires et passez-le via `FERRFLOW_TOKEN` ou configurez l'input `token` de l'action.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Vous préférez ne pas gérer de token ? Mettez <code>bot: true</code> pour attribuer les releases à <code>ferrflow[bot]</code> sans aucun secret — voir le guide <a href="/fr/docs/ci/hosted-bot">Bot hébergé</a>.</p>
</div></aside>

## Accéder à la sortie de la release

L'action expose la nouvelle version en output que vous pouvez utiliser dans les étapes suivantes :

```yaml
- uses: FerrLabs/ferrflow@v5
  id: ferrflow
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

- name: Build Docker image
  if: steps.ferrflow.outputs.version != ''
  run: |
    docker build -t myimage:${{ steps.ferrflow.outputs.version }} .
    docker push myimage:${{ steps.ferrflow.outputs.version }}
```

## Skip CI sur les commits de release

FerrFlow ajoute `[skip ci]` dans le message des commits de version par défaut pour éviter les boucles infinies. Aucune configuration supplémentaire nécessaire.

## Commentaires de preview sur les PR

FerrFlow peut poster un commentaire sur chaque pull request montrant quelles versions seront bumpées au merge. Le commentaire est mis à jour automatiquement à chaque push.

```yaml title=".github/workflows/preview.yml"
name: FerrFlow Preview

on:
  pull_request:

permissions:
  contents: read
  pull-requests: write

jobs:
  preview:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
      - uses: FerrLabs/ferrflow@v5
        with:
          mode: preview
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

Si aucun changement publiable n'est détecté, le commentaire l'indique.

## Exemple monorepo

Dans un monorepo, FerrFlow publie chaque package modifié en une seule exécution :

```yaml
- uses: FerrLabs/ferrflow@v5
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
# Crée api@v1.3.0 et site@v0.5.1 en une seule étape si les deux ont changé
```
