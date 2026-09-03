---
title: Bot hébergé (ferrflow[bot])
description: Publier des releases sous l'identité ferrflow[bot] sans aucun secret, grâce à la GitHub App hébergée de FerrFlow et à l'échange de token OIDC.
---

Par défaut, les releases et les tags que FerrFlow pousse sont attribués au propriétaire du token présent dans votre workflow — généralement un personal access token, si bien que les releases apparaissent sous _votre_ compte. Le bot hébergé permet à la place d'attribuer les releases à **`ferrflow[bot]`**, avec une identité propre et cohérente sur tous vos repos.

Le principe est celui de Renovate ou Dependabot :

- **Zéro secret** dans votre workflow — aucun PAT à créer, stocker ou faire tourner.
- Releases attribuées à **`ferrflow[bot]`**.
- **Tokens courts et scopés** — chaque run reçoit un token neuf qui expire au bout d'une heure et est limité à un seul repository.

## 1. Installer l'app

Rendez-vous sur **[github.com/apps/ferrflow](https://github.com/apps/ferrflow)**, cliquez sur **Install**, puis choisissez l'organisation et les repositories que FerrFlow doit pouvoir publier. C'est tout — aucun secret à créer.

L'app ne demande que **Contents** (lecture et écriture, pour pousser les tags et créer les releases) et **Metadata** (lecture). Vous pouvez la revoir ou la désinstaller à tout moment depuis les réglages de votre organisation.

## 2. L'activer dans votre workflow

Ajoutez `bot: true` à l'action et accordez au workflow la permission d'émettre un token OIDC :

```yaml title=".github/workflows/release.yml"
name: Release

on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      id-token: write # permet au runner de prouver l'identité du repo à FerrFlow
      contents: read # pour le checkout ; le push de la release utilise le token du bot
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0 # l'historique complet est nécessaire à l'analyse des commits

      - uses: FerrLabs/FerrFlow@v5
        with:
          bot: true
```

`permissions.id-token: write` est obligatoire — c'est ce qui permet au runner de demander le token OIDC prouvant quel repository appelle. Sans cette permission, FerrFlow s'arrête avec une erreur claire plutôt que de se rabattre silencieusement sur autre chose.

## Comment ça marche

Aucun secret ne quitte jamais votre repository. Chaque run échange une preuve d'identité contre un token :

1. Le runner GitHub Actions émet un **token OIDC** court décrivant votre repository (audience `ferrflow.ferrlabs.com`).
2. FerrFlow envoie ce token à **`api.ferrflow.com`**, qui le vérifie contre les clés publiques de GitHub.
3. Le service signe un **token d'installation scopé** pour l'app FerrFlow sur votre repo (avec une clé privée qui ne quitte jamais le KMS de FerrLabs) et le renvoie. Le token vit une heure.
4. FerrFlow utilise ce token pour pousser tags, commits et releases — attribués à `ferrflow[bot]`.

## Modèle de sécurité

- **La clé privée de l'app ne quitte jamais les serveurs de FerrLabs** — elle réside dans un KMS et ne sert qu'à signer des tokens d'installation côté serveur.
- **Votre identité est prouvée par OIDC, pas par un secret partagé** — rien de sensible n'est stocké dans votre repo ni transmis depuis celui-ci.
- **Les tokens sont minimaux et éphémères** — limités à un seul repository, expirant au bout d'une heure.
- **Vous gardez le contrôle** — désinstallez l'app à tout moment pour révoquer immédiatement tout accès.

## Dépannage

FerrFlow ne se rabat jamais en silence : si le mode bot ne peut pas obtenir de token, il échoue avec un message nommant la cause exacte.

| Message                                                           | Cause et correctif                                                                                                   |
| ----------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `bot mode requires permissions: id-token: write in your workflow` | Le job n'a pas `permissions.id-token: write`. Ajoutez-le (voir ci-dessus).                                           |
| `FerrFlow App not installed on this repository's owner`           | Installez l'app sur [github.com/apps/ferrflow](https://github.com/apps/ferrflow) pour cette org / ce repo.           |
| `FerrFlow hosted bot rate limit hit (429)`                        | Les demandes de token sont limitées par repository. Réessayez sous peu, ou utilisez un PAT via `token:` pour ce run. |
| `FerrFlow hosted bot service unavailable`                         | Incident temporaire du service. Consultez [status.ferrlabs.com](https://status.ferrlabs.com) ou réessayez.           |

## Alternatives

Le bot hébergé est la voie recommandée, mais ce n'est pas la seule :

- **`token:` avec un PAT** — fournissez votre propre [personal access token](/fr/docs/ci/github-actions). Les releases sont attribuées au propriétaire de ce token. Fonctionne partout, y compris hors GitHub Actions.
- **`token:` avec votre propre GitHub App** — si vous préférez faire tourner votre propre identité de bot, passez un token émis par votre app.
- **`GITHUB_TOKEN` par défaut** — l'option la plus simple, mais notez que les push effectués avec `GITHUB_TOKEN` **ne déclenchent pas les workflows en aval** (un push de tag ne lancera donc pas un job de publish séparé). Le bot hébergé et les PAT n'ont pas cette limite.
