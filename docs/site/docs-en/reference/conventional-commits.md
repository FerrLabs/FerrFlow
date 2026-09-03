---
title: Conventional Commits
description: How FerrFlow interprets commit messages to determine version bumps.
---

FerrFlow follows the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification to determine how much to bump the version.

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

## Permissive defaults

The table above lists the canonical forms, but FerrFlow does not require them. The defaults also accept capitalised and slash-separated variants, so a repository that never enforced the spec still gets sensible bumps from its existing history:

| Bump  | Also accepted by default                                                                   |
| ----- | ------------------------------------------------------------------------------------------ |
| minor | `Feat:`, `feature:`, and the slash forms `feat/`, `Feat/`, `feature/`, `Feature/`          |
| patch | `Fix:`, `Perf:`, `Refactor:`, and the slash forms `fix/`, `Fix/`, `refactor/`, `Refactor/` |

The colon forms take an optional scope, so `Feat(api):` and `Refactor(db):` are covered too. The slash forms do not: `Feat/add-login` matches, `Feat(api)/add-login` does not. There is no `perf/` slash form, and no bare `Feature:`.

Nothing maps to **major** by default: a major bump comes only from a breaking marker, described below.

If your history uses conventions of its own, remap them with [`commitFormats`](/docs/configuration/config-file/). Breaking markers are detected before any configured pattern is consulted, so a custom pattern can never strip a commit of its breaking status.

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

### Accepted footer variants

FerrFlow recognises the common real-world spellings of the footer, not just the strict spec form:

| Footer                                      | Detected               |
| ------------------------------------------- | ---------------------- |
| `BREAKING CHANGE: …`                        | yes (spec)             |
| `BREAKING-CHANGE: …`                        | yes (spec synonym)     |
| `breaking-change: …` / `breaking change: …` | yes (case-insensitive) |
| `Breaking Change: …`                        | yes (case-insensitive) |

It also treats a `!` placed **inside** the scope (`feat(api!):`, a common typo for `feat(api)!:`) as a breaking marker. The footer may sit after any number of body paragraphs.

### What is _not_ a breaking change

Detection stays strict, so a stray mention never triggers an accidental major bump:

- The footer must start a line, use a single space or hyphen (`BREAKING CHANGE` / `BREAKING-CHANGE`), and be followed by a colon **and a space**. `BREAKING CHANGE:no-space` and the plural `BREAKING CHANGES:` are ignored.
- A prose mention mid-line ("this fixes a breaking change in the parser") is not a footer.
- A `!` that is not immediately before the closing paren (`feat(a!b):`) is not a marker.

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
