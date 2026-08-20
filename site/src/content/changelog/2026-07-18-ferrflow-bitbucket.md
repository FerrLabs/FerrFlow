---
title: 'FerrFlow · Bitbucket support'
summary: FerrFlow now recognises Bitbucket Cloud repositories and handles their releases — no more scripting a hook to get automatic release tagging on Bitbucket.
date: 2026-07-18T13:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/729
docsLink: https://ferrflow.com/docs/configuration/config-file
---

Bitbucket users used to fall through FerrFlow's forge detection and had to script release creation by hand. Now `bitbucket.org` remotes are detected automatically (or set `forge = "bitbucket"` explicitly), authenticated with `BITBUCKET_TOKEN`.

Bitbucket has no "release" object the way GitHub and GitLab do, so on Bitbucket a release **is** the annotated tag FerrFlow already creates and pushes — FerrFlow looks it up via the Bitbucket Cloud API and surfaces its URL in the release output. The changelog lives in your `CHANGELOG.md` and the tag message, as usual.

```toml
[workspace]
forge = "bitbucket"   # or leave it "auto" — bitbucket.org is detected
```

Pull-request release mode and Bitbucket Server / Data Center aren't supported yet — this first pass covers Bitbucket Cloud release creation, mirroring how Gitea/Forgejo landed.
