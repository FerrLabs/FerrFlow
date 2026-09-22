---
title: GitLab CI
description: Lancer les releases FerrFlow automatiquement avec GitLab CI.
---

## Utiliser l'image Docker

L'image Docker officielle FerrFlow embarque le binaire et peut être utilisée directement comme image de job GitLab CI.

```yaml
release:
  image:
    name: ghcr.io/ferrlabs/ferrflow:latest
    entrypoint: [""]
  stage: release
  script:
    - ferrflow release
  variables:
    GITLAB_TOKEN: $CI_JOB_TOKEN
  rules:
    - if: $CI_COMMIT_BRANCH == "main"
      when: on_success
```

Le point d'entrée de l'image est `ferrflow`, donc `docker run ghcr.io/ferrlabs/ferrflow:latest check` fonctionne tel quel. GitLab, lui, exécute le `script` d'un job dans un shell : c'est pourquoi chaque exemple ici le réinitialise avec `entrypoint: [""]`. Sans cette ligne, le job échoue avec `unrecognized subcommand 'sh'`.

L'image embarque `git`, fait confiance au dépôt quel que soit l'utilisateur qui l'a cloné, et signe les commits de release `FerrFlow <bot@ferrflow.com>`. Pour les signer au nom de la personne qui a déclenché le pipeline, définissez `GIT_AUTHOR_NAME: $GITLAB_USER_NAME` et `GIT_AUTHOR_EMAIL: $GITLAB_USER_EMAIL` dans les variables du job.

<aside class="ferr-aside ferr-aside--warning"><div class="ferr-aside__body"><p>Assurez-vous que votre runner CI clone avec l&#39;historique complet. Ajoutez <code>GIT_DEPTH: 0</code> aux variables du job pour désactiver le clonage superficiel.</p>
</div></aside>

## Historique complet

```yaml
release:
  image:
    name: ghcr.io/ferrlabs/ferrflow:latest
    entrypoint: [""]
  variables:
    GIT_DEPTH: 0 # historique complet : requis pour le scan des tags
    GITLAB_TOKEN: $CI_JOB_TOKEN
  script:
    - ferrflow release
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

## Utiliser `CI_JOB_TOKEN`

Les exemples ci-dessus passent le token du job via `GITLAB_TOKEN: $CI_JOB_TOKEN`. FerrFlow le reconnaît parce que sa valeur est égale à `CI_JOB_TOKEN`, et s'authentifie comme GitLab l'attend pour un token de job : les appels d'API portent un en-tête `JOB-TOKEN` et git pousse en tant que `gitlab-ci-token`. C'est aussi le cas quand le token du job est passé dans `FERRFLOW_TOKEN`. Tout autre token (token d'accès de projet, de groupe ou personnel) est envoyé en `PRIVATE-TOKEN` et pousse en tant que `oauth2`.

Ce qu'un token de job a le droit de faire se règle par projet dans **Settings > CI/CD > Job token permissions**. Pousser le commit et les tags de release nécessite **Allow Git push requests to the repository**. Quand GitLab refuse un des appels d'API que FerrFlow fait avec un token de job, utilisez plutôt un token d'accès de projet avec le scope `api`.

## Utiliser un deploy token

Si `CI_JOB_TOKEN` n'a pas les permissions pour pousser des tags, créez un deploy token de projet avec l'accès `write_repository` et stockez-le comme variable CI :

```yaml
release:
  image:
    name: ghcr.io/ferrlabs/ferrflow:latest
    entrypoint: [""]
  variables:
    GIT_DEPTH: 0
    GITLAB_TOKEN: $FERRFLOW_DEPLOY_TOKEN # variable CI avec accès write_repository
  script:
    - ferrflow release
  rules:
    - if: $CI_COMMIT_BRANCH == "main"
```

## Commentaires de preview sur les MR

FerrFlow peut poster un commentaire sur chaque merge request montrant quelles versions seront bumpées au merge. Le commentaire est mis à jour automatiquement à chaque push.

```yaml title=".gitlab-ci.yml"
ferrflow-preview:
  image:
    name: ghcr.io/ferrlabs/ferrflow:latest
    entrypoint: [""]
  stage: test
  variables:
    GIT_DEPTH: 0
    GITLAB_TOKEN: $CI_JOB_TOKEN
  script:
    - ferrflow check --comment
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

Si aucun changement publiable n'est détecté, le commentaire l'indique.

Si vous stockez ce token dans une variable **protégée**, GitLab ne l'expose qu'aux pipelines des branches et tags protégés, et le pipeline d'une merge request venant d'une branche ordinaire tourne sans lui. FerrFlow affiche alors `Warning: preview comment not posted: no Gitlab token found in FERRFLOW_TOKEN or GITLAB_TOKEN` et le job réussit quand même. Retirez la protection de la variable, ou utilisez `CI_JOB_TOKEN`, que chaque job reçoit.

## GitLab Releases

Lorsque `GITLAB_TOKEN` est défini, FerrFlow crée une GitLab Release avec le changelog généré comme notes de release, de la même manière que l'intégration GitHub.
