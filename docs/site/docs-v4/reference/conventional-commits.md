---
title: Conventional Commits
description: How FerrFlow interprets commit messages to determine version bumps.
---

FerrFlow follows the [Conventional Commits](https://www.conventionalcommits.org/) specification to determine how much to bump the version.

## Bump rules

| Commit type                   | Version bump | Example                             |
| ----------------------------- | ------------ | ----------------------------------- |
| `feat:`                       | **minor**    | `feat: add wallet subscriptions`    |
| `fix:`                        | patch        | `fix: correct pagination offset`    |
| `perf:`                       | patch        | `perf: cache user queries`          |
| `refactor:`                   | patch        | `refactor: extract auth middleware` |
| `feat!:` or `BREAKING CHANGE` | **major**    | `feat!: remove deprecated endpoint` |
| `chore:`                      | none         | `chore: update dependencies`        |
| `docs:`                       | none         | `docs: update README`               |
| `ci:`                         | none         | `ci: add linting step`              |
| `style:`                      | none         | `style: format code`                |
| `test:`                       | none         | `test: add unit tests`              |

## Breaking changes

A breaking change can be indicated in two ways:

**Exclamation mark suffix:**

```
feat!: remove the /v1/users endpoint
fix!: change authentication header format
```

**`BREAKING CHANGE` footer:**

```
feat: redesign the API

BREAKING CHANGE: The /v1/users endpoint has been removed. Use /v2/users instead.
```

Both produce a **major** version bump.

## Scope

Scopes are optional and ignored for bump calculation. They're useful for readability:

```
feat(auth): add OAuth2 support       → minor bump
fix(db): correct index on user table  → patch bump
```

## No release

Commits with types `chore`, `docs`, `ci`, `style`, or `test` do not trigger a release. If all commits since the last tag are of these types, FerrFlow exits without creating a new version.

## Multiple commits

When multiple commits are present since the last tag, FerrFlow takes the **highest** bump across all of them:

```
fix: correct typo       → patch
feat: add export button → minor   ← wins
chore: lint             → none
```

Result: **minor** bump.
