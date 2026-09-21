---
title: GitLab CI
description: Lancer les releases FerrFlow automatiquement avec GitLab CI.
---

## Utiliser l'image Docker

L'image Docker officielle FerrFlow embarque le binaire et peut être utilisée directement comme image de job GitLab CI.

```yaml
release:
  image: ghcr.io/ferrlabs/ferrflow:latest
  stage: release
  script:
    - ferrflow release
  variables:
    GITLAB_TOKEN: $CI_JOB_TOKEN
  rules:
    - if: $CI_COMMIT_BRANCH == "main"
      when: on_success
```

<aside class="ferr-aside ferr-aside--warning"><div class="ferr-aside__body"><p>Assurez-vous que votre runner CI clone avec l&#39;historique complet. Ajoutez <code>GIT_DEPTH: 0</code> aux variables du job pour désactiver le clonage superficiel.</p>
</div></aside>

## Historique complet

```yaml
release:
  image: ghcr.io/ferrlabs/ferrflow:latest
  variables:
    GIT_DEPTH: 0 # historique complet — requis pour le scan des tags
    GITLAB_TOKEN: $CI_JOB_TOKEN
  script:
    - ferrflow release
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

## Utiliser un deploy token

Si `CI_JOB_TOKEN` n'a pas les permissions pour pousser des tags, créez un deploy token de projet avec l'accès `write_repository` et stockez-le comme variable CI :

```yaml
release:
  image: ghcr.io/ferrlabs/ferrflow:latest
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
  image: ghcr.io/ferrlabs/ferrflow:latest
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

## GitLab Releases

Lorsque `GITLAB_TOKEN` est défini, FerrFlow crée une GitLab Release avec le changelog généré comme notes de release, de la même manière que l'intégration GitHub.
