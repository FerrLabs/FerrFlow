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

FerrFlow peut poster un commentaire sur chaque merge request montrant quelles versions seront bump\u00e9es au merge. Le commentaire est mis \u00e0 jour automatiquement \u00e0 chaque push.

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

Si aucun changement publiable n'est d\u00e9tect\u00e9, le commentaire l'indique.

Si vous stockez ce token dans une variable **protégée**, GitLab ne l'expose qu'aux pipelines des branches et tags protégés, et le pipeline d'une merge request venant d'une branche ordinaire tourne sans lui. FerrFlow affiche alors `Warning: preview comment not posted: no Gitlab token found in FERRFLOW_TOKEN or GITLAB_TOKEN` et le job réussit quand même. Retirez la protection de la variable, ou utilisez `CI_JOB_TOKEN`, que chaque job reçoit.

## GitLab Releases

Lorsque `GITLAB_TOKEN` est d\u00e9fini, FerrFlow cr\u00e9e une GitLab Release avec le changelog g\u00e9n\u00e9r\u00e9 comme notes de release, de la m\u00eame mani\u00e8re que l'int\u00e9gration GitHub.
