---
title: 'FerrFlow: release commits from the bot now show as verified'
summary: 'Commits authored by ferrflow[bot] showed as unverified, and no signing key could fix it because a GitHub App cannot register one. With bot: true on GitHub, the release commit is now authored through GitHub itself, which signs it.'
date: 2026-08-25T18:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/941
docsLink: https://ferrflow.com/docs/configuration/github-action/
---

Release commits made under `bot: true` carried the `ferrflow[bot]` identity but showed as unverified in the GitHub UI, sitting next to everyone else's verified work. There was no configuration that fixed it: GitHub Apps have no setting for a GPG or SSH key, so there was no key for FerrFlow to sign with. The alternative, shipping a signing key into every user's CI, is exactly what you do not want a release tool to ask of you.

GitHub offers the way out. A commit created through its `createCommitOnBranch` API is signed by GitHub and marked verified, and an App can use it to author commits in its own name. No private key exists anywhere in the chain. With `bot: true` on a GitHub repository, FerrFlow now creates the release commit that way instead of through local git, in both `commit` and `pr` modes.

One condition comes with it, and it shapes the result. GitHub signs a bot's commit only when the request carries no custom author, committer or signature information. So the commit is attributed to `ferrflow[bot]` and to nothing else. There is no version of this where FerrFlow sets a custom author and still gets the verified mark.

Nothing changes anywhere else. Without `bot: true`, or on GitLab, Gitea, Forgejo and Bitbucket, the commit is built by local git exactly as before. If you want verified commits under your own identity rather than the bot's, set `commit.gpgsign` and FerrFlow honours it on every forge and on both commit paths.

Worth knowing: annotated tags are still created and pushed by git, so they are not covered by this. Only the commit is signed.
