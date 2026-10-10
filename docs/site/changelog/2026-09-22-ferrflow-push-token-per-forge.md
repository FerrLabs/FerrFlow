---
title: 'FerrFlow: GITHUB_TOKEN only reaches remotes that are GitHub'
summary: 'A push to a remote FerrFlow could not identify used to carry GITHUB_TOKEN as its password, so a self-hosted forge or an unknown host received a GitHub credential. Each forge now pushes with its own token, and an unrecognised host gets none.'
date: 2026-09-22T18:36:17Z
product: ferrflow
type: security
prLink: https://github.com/FerrLabs/FerrFlow/pull/1152
docsLink: https://ferrflow.com/docs/configuration/config-file/
---

When FerrFlow pushed the release commit and tags, it picked the credential by asking one question: is this remote GitLab? Everything else got `GITHUB_TOKEN`. That covers GitHub, but also a self-hosted Gitea or Forgejo, Bitbucket, and any host whose forge could not be identified. Every GitHub Actions job has `GITHUB_TOKEN` in its environment, so a remote pointing somewhere else received a GitHub credential as its push password.

Each forge now pushes with the token it actually uses: `GITHUB_TOKEN` for GitHub, `GITLAB_TOKEN` for GitLab, `GITEA_TOKEN` or `FORGEJO_TOKEN` for Gitea and Forgejo, `BITBUCKET_TOKEN` for Bitbucket. These are the same variables the API side has always read, so the two halves can no longer disagree. `FERRFLOW_TOKEN` still works everywhere: it is the explicit "use this for whatever this remote is".

A host FerrFlow cannot identify now gets no forge token at all. If `GITHUB_TOKEN` is set and withheld, the run says so once:

```
Warning: not sending GITHUB_TOKEN to git.corp.example, which is not recognised as GitHub. Set `forge` in the config if it is a GitHub Enterprise instance, or pass the token as FERRFLOW_TOKEN.
```

Two things to check if you self-host. A GitHub Enterprise instance that FerrFlow cannot reach to identify needs `"forge": "github"` under `workspace` in your config. Gitea and Forgejo Actions export their job token under the name `GITHUB_TOKEN`, so those jobs keep pushing for now, with a warning: pass it as `GITEA_TOKEN` instead, because that fallback goes away in the next major version, and creating the release already reads `GITEA_TOKEN` or `FORGEJO_TOKEN` rather than falling back.

Identifying a self-hosted host got stricter in the same pass. A GitHub Enterprise instance is recognised only by the `X-GitHub-Enterprise-Version` header its API sends, not by anything answering `/api/v3`, and the probe no longer follows redirects, so one host can no longer borrow another's answer to be handed a token. An instance in private mode, which answers 401 without a session, is now detected where it was missed before. The trade-off is that an instance which redirects its own API root is no longer detected either, and takes the same `forge` setting as above.
