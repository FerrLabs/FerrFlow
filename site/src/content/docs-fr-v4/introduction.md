---
title: Introduction
description: Ce qu'est FerrFlow et pourquoi il existe.
---

FerrFlow est un binaire unique qui automatise le versionnage sémantique pour n'importe quel repository — monorepo ou classique, quel que soit le langage.

Il analyse votre historique de commits, détermine le bon incrément de version, met à jour vos fichiers de version, rédige un changelog, crée un tag git et publie une release. Zéro dépendance runtime.

<div class="ferr-card-group" data-cols="2">
  <div class="ferr-card"><p class="ferr-card__title">CLI d'abord</p><div class="ferr-card__body"><p>Tout se passe depuis votre terminal ou votre CI. Aucune UI à cliquer, aucun serveur de configuration à surveiller.</p>
</div></div>
  <div class="ferr-card"><p class="ferr-card__title">Multi-forge</p><div class="ferr-card__body"><p>GitHub, GitLab, auto-hébergé — FerrFlow s&#39;adapte à votre forge. Un seul outil, toutes les plateformes.</p>
</div></div>
  <div class="ferr-card"><p class="ferr-card__title">Commits conventionnels</p><div class="ferr-card__body"><p>Analyse l&#39;historique des commits pour déterminer les incréments de version automatiquement. Aucune maintenance manuelle du changelog.</p>
</div></div>
  <div class="ferr-card"><p class="ferr-card__title">Zéro infra</p><div class="ferr-card__body"><p>Un seul binaire, sans daemon, sans serveur, sans base de données. Tourne là où votre CI tourne.</p>
</div></div>
</div>

## Pourquoi pas semantic-release ou changesets ?

La plupart des outils de versionnage sont liés à un écosystème spécifique ou nécessitent Node.js dans votre CI.

| Outil            | Monorepo    | Multi-langage      | Runtime   |
| ---------------- | ----------- | ------------------ | --------- |
| semantic-release | via plugins | JS/Node uniquement | Node.js   |
| changesets       | bump manuel | JS uniquement      | Node.js   |
| release-please   | limité      | partiel            | Node.js   |
| cargo-release    | non         | Rust uniquement    | Rust      |
| **FerrFlow**     | **natif**   | **tous**           | **aucun** |

FerrFlow est distribué sous forme de binaire compilé. Déposez-le dans n'importe quel environnement CI sans installer de runtime. Un build WASM (`@ferrflow/wasm`) est également disponible pour une utilisation côté navigateur. Pour la comparaison côte-à-côte (latence, RSS, taille) avec les outils de release JS, voir [Performance](/fr/performance) — chiffres rafraîchis à chaque release.

<aside class="ferr-aside ferr-aside--note"><p class="ferr-aside__title">À noter</p><div class="ferr-aside__body"><p>FerrFlow ne fait que du versionnage. Le suivi des issues, les secrets et les agents IA vivent dans d&#39;autres produits FerrLabs.</p>
</div></aside>

## Comment ça marche

1. **Lit les commits** depuis le dernier tag git pour chaque package
2. **Détermine l'incrément** à partir des [Conventional Commits](/fr/docs/reference/conventional-commits) (`feat` → minor, `fix` → patch, breaking → major)
3. **Met à jour les fichiers de version** — `Cargo.toml`, `package.json`, `pom.xml`, etc.
4. **Rédige le changelog** au format Keep a Changelog
5. **Crée un tag git** (`api@v1.2.0`) et pousse
6. **Publie une release GitHub/GitLab** avec le changelog comme notes de version

Dans un monorepo, FerrFlow ne publie que les packages modifiés et comprend les chemins de dépendances partagées.

## Fonctionnalités clés

- **Hooks pre/post-release** — exécutez des scripts à chaque étape du cycle de vie (bump, commit, publish, failure)
- **Commandes de requête** — `ferrflow version`, `ferrflow tag` et `ferrflow status` pour le scripting CI
- **Tout fichier de version** — Cargo.toml, package.json, pom.xml, build.gradle, Chart.yaml, texte brut, et plus
- **Support navigateur** — `@ferrflow/wasm` apporte le parsing de commits, le calcul de bump et la génération de changelog dans le navigateur
