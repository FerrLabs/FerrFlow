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

Les commits et tags de release utilisent l'identité git du dépôt. Si le job n'en définit aucune, FerrFlow commite en tant que `github-actions[bot]`, le compte auquel un push via `GITHUB_TOKEN` est de toute façon attribué. Hors de GitHub Actions, une release sans `user.name` ni `user.email` s'arrête avant de modifier quoi que ce soit.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Vous préférez ne pas gérer de token ? Mettez <code>bot: true</code> pour attribuer les releases à <code>ferrflow[bot]</code> sans aucun secret : voir le guide <a href="/fr/docs/ci/hosted-bot">Bot hébergé</a>.</p>
</div></aside>

## Signer le tag de release

GitHub signe les commits qu'il crée via son API : c'est pourquoi un commit de release fait avec `bot: true` apparaît vérifié. Il ne signe jamais un objet tag, et une GitHub App ne possède aucune clé, donc le tag poussé par FerrFlow reste non signé tant que vous ne donnez pas de clé de signature à l'action.

```yaml
- uses: FerrLabs/FerrFlow@v7
  with:
    bot: true
    tag_signing_key: ${{ secrets.TAG_SIGNING_KEY }}
    tag_signing_name: ${{ vars.TAG_SIGNING_NAME }}
    tag_signing_email: ${{ vars.TAG_SIGNING_EMAIL }}
```

La clé est une clé privée OpenSSH non chiffrée, et le tagger avec lequel elle signe doit être un compte auquel GitHub peut rattacher la signature :

```bash
ssh-keygen -t ed25519 -C "releases" -N "" -f ferrflow-tag-signing
```

Sous PowerShell, écrivez la passphrase vide `-N '""'`, ou omettez `-N` et appuyez deux fois sur Entrée : PowerShell supprime un `""` nu, et la clé doit être sans passphrase pour que le runner puisse l'utiliser.

Ajoutez `ferrflow-tag-signing.pub` à ce compte via **Settings > SSH and GPG keys > New SSH key** en choisissant le type **Signing Key**, stockez la moitié privée dans le secret `TAG_SIGNING_KEY`, et mettez dans `tag_signing_email` une des adresses vérifiées du compte. GitHub vérifie un tag contre les clés du compte dont le tagger porte l'adresse vérifiée : en cas de décalage, le tag s'affiche comme non vérifié, la release n'échoue pas.

L'action écrit la clé sous `RUNNER_TEMP`, le temps du job, et elle n'atteint ni le dépôt ni le tag lui-même. Seul le tag est signé : le commit de release porte déjà la signature de GitHub en mode bot, et les archives de release sont signées à part avec cosign.

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
