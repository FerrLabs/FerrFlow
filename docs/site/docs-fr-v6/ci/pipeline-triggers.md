---
title: Triggers de pipeline
description: Choisir la bonne strategie de declenchement CI pour vos releases FerrFlow.
---

FerrFlow fonctionne avec n'importe quelle strategie de declenchement CI. Cette page couvre les patterns les plus courants, quand les utiliser, et comment ils interagissent avec `releaseCommitMode`.

## Push sur main

La configuration la plus simple : executer `ferrflow release` a chaque push sur la branche par defaut. FerrFlow determine si une release est necessaire en se basant sur les commits depuis le dernier tag.

```yaml title=".github/workflows/release.yml"
name: Release

on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    if: "!startsWith(github.event.head_commit.message, 'chore(release):')"
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
          token: ${{ secrets.FERRFLOW_TOKEN }}

      - uses: FerrLabs/ferrflow@v5
        env:
          GITHUB_TOKEN: ${{ secrets.FERRFLOW_TOKEN }}
```

**Quand l'utiliser :** La plupart des projets. Simple, previsible, entierement automatise.

**Compromis :** Chaque push sur main declenche un workflow, meme si aucune release n'est necessaire. FerrFlow se termine rapidement quand il n'y a pas de commits a releaser.

**Compatible avec :** `releaseCommitMode: commit` (par defaut) ou `none`.

## Declenchement par tag

Executez votre pipeline de build/deploiement quand FerrFlow cree un nouveau tag. Cela separe l'etape de release (tagging) des etapes en aval (build, publication, deploiement).

```yaml title=".github/workflows/build.yml"
name: Build & Publish

on:
  push:
    tags:
      - 'v*' # single-repo: v1.2.0
      - '*@v*' # monorepo: api@v1.2.0, site@v0.5.1

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Extraire la version du tag
        id: version
        run: |
          TAG="${GITHUB_REF_NAME}"
          VERSION="${TAG##*v}"
          echo "tag=$TAG" >> "$GITHUB_OUTPUT"
          echo "version=$VERSION" >> "$GITHUB_OUTPUT"

      - name: Build
        run: echo "Building version ${{ steps.version.outputs.version }}"
```

**Quand l'utiliser :** Quand vous voulez decoupler le versioning du build/deploiement. Courant pour les builds Docker, la publication npm ou les releases binaires.

**Compromis :** Necessite deux workflows : un pour la release (push-to-main) et un pour le build en aval (tag-triggered). Ajoute quelques secondes de latence.

### Monorepo : builds par package

Dans un monorepo, utilisez les patterns de tags pour ne builder que le package concerne :

```yaml title=".github/workflows/build.yml"
name: Build Package

on:
  push:
    tags:
      - 'api@v*'
      - 'site@v*'

jobs:
  build-api:
    if: startsWith(github.ref_name, 'api@v')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - run: echo "Building API ${{ github.ref_name }}"

  build-site:
    if: startsWith(github.ref_name, 'site@v')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - run: echo "Building site ${{ github.ref_name }}"
```

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p>Si FerrFlow release plusieurs packages en une seule execution (ex: <code>api@v1.3.0</code> et <code>site@v0.5.1</code>), chaque tag declenche son propre workflow. Les builds s&#39;executent en parallele automatiquement.</p>
</div></aside>

## Declenchement par release

Executez un pipeline quand une GitHub Release est publiee. Fonctionne bien avec le flag `--draft` de FerrFlow : FerrFlow cree une release brouillon, vous la reviewez, puis la publication declenche le build.

```yaml title=".github/workflows/deploy.yml"
name: Deploy

on:
  release:
    types: [published]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          ref: ${{ github.event.release.tag_name }}

      - name: Deploy
        run: echo "Deploying ${{ github.event.release.tag_name }}"
```

**Quand l'utiliser :** Quand vous voulez une etape de revue manuelle avant le deploiement. Creez des releases brouillon avec `ferrflow release --draft`, reviewez le changelog, puis publiez.

**Compromis :** Ajoute une etape manuelle. La release brouillon doit etre publiee avant que le deploy ne se lance.

## Manuel (workflow_dispatch)

Declenchez une release a la demande avec un flag dry-run optionnel.

```yaml title=".github/workflows/release.yml"
name: Release

on:
  workflow_dispatch:
    inputs:
      dry_run:
        description: 'Dry run (pas de tags, pas de commits, pas de releases)'
        type: boolean
        default: false

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
          token: ${{ secrets.FERRFLOW_TOKEN }}

      - uses: FerrLabs/ferrflow@v5
        with:
          args: ${{ inputs.dry_run == true && '--dry-run' || '' }}
        env:
          GITHUB_TOKEN: ${{ secrets.FERRFLOW_TOKEN }}
```

**Quand l'utiliser :** Equipes qui preferent des decisions de release explicites. Aussi utile comme workflow secondaire pour des releases ad-hoc.

## Base sur une PR

Utilisez `releaseCommitMode: pr` pour que FerrFlow ouvre une pull request avec le bump de version au lieu de committer directement.

```yaml title="ferrflow.json"
{ 'workspace': { 'releaseCommitMode': 'pr' } }
```

```yaml title=".github/workflows/release.yml"
name: Release

on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    if: "!startsWith(github.event.head_commit.message, 'chore(release):')"
    permissions:
      contents: write
      pull-requests: write
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
          token: ${{ secrets.FERRFLOW_TOKEN }}

      - uses: FerrLabs/ferrflow@v5
        env:
          GITHUB_TOKEN: ${{ secrets.FERRFLOW_TOKEN }}
```

**Quand l'utiliser :** Quand vous voulez reviewer les bumps de version, ou quand la protection de branche empeche les push directs sur main.

## Securite de concurrence

Depuis la v5.2, `ferrflow release` acquiert `.git/ferrflow.lock` de maniere atomique (`O_CREAT|O_EXCL`) au debut de chaque execution mutante. Une seconde invocation concurrente sur le meme depot echoue immediatement avec une erreur claire au lieu de courir contre les refs git — le scenario classique est une release declenchee manuellement qui demarre en meme temps qu'un `auto-release` planifie en cron, ce qui produit des jeux de tags poussés à moitié, des refus non fast-forward ou des draft releases dupliquees.

Rien à brancher — le verrou est automatique sur chaque invocation `release`. Les commandes en lecture seule (`check`, `status`, `version`, `tag`) ne le prennent pas.

Si une execution précédente a planté sans relacher le verrou, l'invocation suivante le reprend automatiquement apres 30 minutes (l'hote + le PID inscrits dans le lockfile permettent à FerrFlow de detecter les verrous orphelins). Pour le reprendre plus tot, supprimez `.git/ferrflow.lock` à la main.

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p>Le verrou est par-depot, scope a <code>.git/</code>. Il ne protege pas entre des clones separes du meme depot — si vous lancez des releases simultanees depuis deux runners differents contre deux checkouts du meme remote, le verrou ne voit pas l&#39;autre cote. Utilisez un seul runner de release, ou serialisez au niveau CI (<code>concurrency:</code> dans GitHub Actions, <code>interruptible: false</code> dans GitLab).</p>
</div></aside>

## Reprise apres crash

Une release enchaine plusieurs etapes a effet de bord : ecriture des fichiers de version → commit → tag → push du commit → push des tags → creation des GitHub Releases → execution des hooks `post_publish`. Si le process meurt entre l'une d'elles (coupure reseau, arret du runner, SIGKILL), le depot reste dans un etat de release partiel.

Depuis la v5.3, FerrFlow ecrit `.git/ferrflow.checkpoint.json` apres chaque phase reussie. La prochaine invocation `ferrflow release` sur le meme commit reprend a la phase enregistree et saute le travail deja fait — utile a l'echelle CI ou les pannes transitoires sont la norme. Le checkpoint est efface automatiquement quand la release se termine sans erreur.

Deux garde-fous :

- Verrouillage HEAD : le checkpoint enregistre le SHA du commit sur lequel la release tournait. Si vous avancez HEAD entre deux runs, FerrFlow refuse de reprendre et vous demande soit de revenir au commit enregistre, soit de supprimer le checkpoint manuellement. Ca evite de rejouer d'anciens tags sur un graphe de commits different.
- Idempotence : les operations git et forge sous-jacentes sont deja idempotentes (pousser un tag deja pousse est un no-op, creer une release qui existe deja la reutilise), donc un crash en milieu de phase recupere proprement au run suivant.

Pour repartir de zero apres un crash dont vous ne voulez pas reprendre l'etat, supprimez `.git/ferrflow.checkpoint.json` a la main.

## Resume

| Trigger           | Automatique | Gate de revue | Ideal pour                    |
| ----------------- | ----------- | ------------- | ----------------------------- |
| Push sur main     | Oui         | Non           | La plupart des projets        |
| Tag-triggered     | Oui         | Non           | Build/deploy decouples        |
| Release-triggered | Non         | Oui           | Brouillon, revue, publication |
| Manuel            | Non         | Oui           | Cadence de release controlee  |
| Base sur PR       | Partiel     | Oui           | Protection de branche / revue |
