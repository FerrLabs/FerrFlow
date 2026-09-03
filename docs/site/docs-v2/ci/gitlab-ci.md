---
title: GitLab CI
description: Run FerrFlow releases automatically in GitLab CI.
---

## Using the Docker image

The official FerrFlow Docker image ships the binary and can be used directly as a GitLab CI job image.

```yaml title=".gitlab-ci.yml"
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

<aside class="ferr-aside ferr-aside--warning"><div class="ferr-aside__body"><p>Make sure your CI runner clones with full history. Add <code>GIT_DEPTH: 0</code> to the job variables to disable shallow cloning.</p>
</div></aside>

## Full history

```yaml title=".gitlab-ci.yml"
release:
  image: ghcr.io/ferrlabs/ferrflow:latest
  variables:
    GIT_DEPTH: 0 # full history — required for tag scanning
    GITLAB_TOKEN: $CI_JOB_TOKEN
  script:
    - ferrflow release
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

## Using a deploy token

If `CI_JOB_TOKEN` doesn't have permission to push tags, create a project deploy token with `write_repository` access and store it as a CI variable:

```yaml title=".gitlab-ci.yml"
release:
  image: ghcr.io/ferrlabs/ferrflow:latest
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

## GitLab Releases

When `GITLAB_TOKEN` is set, FerrFlow creates a GitLab Release with the generated changelog as release notes, matching the behaviour of the GitHub integration.
