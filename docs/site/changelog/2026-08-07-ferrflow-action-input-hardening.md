---
title: 'FerrFlow: the Action no longer interpolates its inputs into shell'
summary: 'force_version, package and version reached the Action’s run scripts as raw shell source. They now arrive as environment variables and are validated before use.'
date: 2026-08-07T09:00:00Z
product: ferrflow
type: security
prLink: https://github.com/FerrLabs/FerrFlow/pull/810
---

The FerrFlow GitHub Action built three of its shell steps with `${{ }}` interpolation, which substitutes a value into the script *before* the shell parses it. Any workflow that wired `force_version`, `package` or `version` from data it did not control: an issue title, a `workflow_dispatch` input, a matrix value derived from a pull request: could therefore have had that data executed as shell, in a job that by design holds a repository write token.

All three inputs now reach the step as environment variables, are quoted at every use, and are checked before they are passed on: `version` must be `latest` or a semver triple, `force_version` must be `VERSION` or `NAME@VERSION`, and each entry in `package` must look like a package name. Anything else fails the step with a clear message instead of running.

Arguments are assembled into an array and passed as `ferrflow "${args[@]}"`, so a value containing spaces or shell metacharacters is one inert argument rather than parsed source.

**What you need to do:** nothing beyond moving to a current version of the Action. No input changed name, shape or meaning. If you were passing a value that does not match the formats above: a version string with unusual characters, for instance. The step will now tell you rather than silently doing something else.

Also fixed in the same change: `mode: publish` did not forward the `bot` inputs, so `bot: true` combined with `mode: publish` silently ran without the hosted bot token.
