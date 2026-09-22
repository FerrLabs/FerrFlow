---
title: GitLab CI
description: Run FerrFlow releases automatically in GitLab CI.
---

## Using the Docker image

The official FerrFlow Docker image ships the binary and can be used directly as a GitLab CI job image.

```yaml title=".gitlab-ci.yml"
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

The image's entrypoint is `ferrflow`, so `docker run ghcr.io/ferrlabs/ferrflow:latest check` works as is. GitLab runs a job's `script` through a shell instead, which is why every example here resets it with `entrypoint: [""]`. Without that line the job fails with `unrecognized subcommand 'sh'`.

The image ships `git`, trusts the checkout whatever user cloned it, and signs release commits as `FerrFlow <bot@ferrflow.com>`. To sign them as the person who triggered the pipeline instead, set `GIT_AUTHOR_NAME: $GITLAB_USER_NAME` and `GIT_AUTHOR_EMAIL: $GITLAB_USER_EMAIL` in the job variables.

<aside class="ferr-aside ferr-aside--warning"><div class="ferr-aside__body"><p>Make sure your CI runner clones with full history. Add <code>GIT_DEPTH: 0</code> to the job variables to disable shallow cloning.</p>
</div></aside>

## Full history

```yaml title=".gitlab-ci.yml"
release:
  image:
    name: ghcr.io/ferrlabs/ferrflow:latest
    entrypoint: [""]
  variables:
    GIT_DEPTH: 0 # full history: required for tag scanning
    GITLAB_TOKEN: $CI_JOB_TOKEN
  script:
    - ferrflow release
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

## Using `CI_JOB_TOKEN`

The examples above pass the job token as `GITLAB_TOKEN: $CI_JOB_TOKEN`. FerrFlow recognises it because its value equals `CI_JOB_TOKEN`, and authenticates the way GitLab expects for a job token: API calls carry a `JOB-TOKEN` header and git pushes as `gitlab-ci-token`. The same applies when the job token is passed as `FERRFLOW_TOKEN`. Any other token (project, group or personal access token) is sent as `PRIVATE-TOKEN` and pushes as `oauth2`.

What a job token may do is set per project under **Settings > CI/CD > Job token permissions**. Pushing the release commit and tags needs **Allow Git push requests to the repository**. When GitLab refuses one of the API calls FerrFlow makes with a job token, use a project access token with the `api` scope instead.

## Using a deploy token

If `CI_JOB_TOKEN` doesn't have permission to push tags, create a project deploy token with `write_repository` access and store it as a CI variable:

```yaml title=".gitlab-ci.yml"
release:
  image:
    name: ghcr.io/ferrlabs/ferrflow:latest
    entrypoint: [""]
  variables:
    GIT_DEPTH: 0
    GITLAB_TOKEN: $FERRFLOW_DEPLOY_TOKEN # CI variable with write_repository access
  script:
    - ferrflow release
  rules:
    - if: $CI_COMMIT_BRANCH == "main"
```

## MR preview comments

FerrFlow can post a comment on every merge request showing what versions will be bumped when the MR is merged. The comment is automatically updated on each push.

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

The comment looks like:

> **FerrFlow Release Preview**
>
> | Package | Current | Next    | Bump  |
> | ------- | ------- | ------- | ----- |
> | api     | `1.5.0` | `1.6.0` | minor |
>
> Based on 2 commit(s).

If no releasable changes are detected, the comment says so.

<aside class="ferr-aside ferr-aside--note"><div class="ferr-aside__body"><p><code>CI_JOB_TOKEN</code> has permission to post MR notes by default. If your project restricts this, use a project access token with <code>api</code> scope stored as a CI variable.</p>
</div></aside>

If you store that token as a **protected** variable, GitLab only exposes it to pipelines on protected branches and tags, and a merge request pipeline from an ordinary branch runs without it. FerrFlow then prints `Warning: preview comment not posted: no Gitlab token found in FERRFLOW_TOKEN or GITLAB_TOKEN` and the job still succeeds. Either unprotect the variable or use `CI_JOB_TOKEN`, which every job receives.

## GitLab Releases

When `GITLAB_TOKEN` is set, FerrFlow creates a GitLab Release with the generated changelog as release notes, matching the behaviour of the GitHub integration.
