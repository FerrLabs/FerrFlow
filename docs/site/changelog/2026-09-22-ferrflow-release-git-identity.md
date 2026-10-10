---
title: 'FerrFlow: releases on GitHub Actions no longer need a git identity'
summary: 'A release using GITHUB_TOKEN without bot mode wrote the version files and changelog, then failed on the release commit with "Author identity unknown". On Actions it now commits as github-actions[bot]; elsewhere it stops before touching anything.'
date: 2026-09-22T20:00:00Z
product: ferrflow
type: fix
draft: true
prLink: https://github.com/FerrLabs/FerrFlow/pull/1128
docsLink: https://ferrflow.com/docs/ci/github-actions/
---

Until now, FerrFlow only set a git author in bot mode. A workflow calling the action with `GITHUB_TOKEN` and no `bot: true` got as far as writing the version files and the CHANGELOG, and then `git commit` failed with `Author identity unknown`. The job left a dirty tree behind and no release.

A release now checks for an identity before it bumps anything, and it checks the same way `git commit` does. Anything git itself would accept still counts: a configured `user.name` and `user.email`, the `GIT_AUTHOR_*` variables, or an identity git works out on its own.

If there is none and the job runs on GitHub Actions, FerrFlow commits and tags as `github-actions[bot]`, the account a `GITHUB_TOKEN` push is credited to anyway. Your workflow needs no changes, and a `git config` step you already have keeps working. Anywhere else, the release stops at once with the two `git config` commands to run, and nothing is written.

`ferrflow doctor` also has a new `git identity` check, so you can catch this before a first release.
