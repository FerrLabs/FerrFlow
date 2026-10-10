---
title: 'FerrFlow: Auto-detect self-hosted forges'
summary: FerrFlow now recognises self-hosted GitLab, GitHub Enterprise, and Gitea/Forgejo instances automatically. No need to set `forge` by hand for a git host on your own domain.
date: 2026-07-18T14:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/730
docsLink: https://ferrflow.com/docs/configuration/config-file
---

Until now, forge detection only knew the SaaS hostnames: a self-hosted GitLab or Gitea on `git.company.com` came back as "no forge", so you had to set `forge = "gitlab"` yourself to get automatic release creation.

Now, when FerrFlow doesn't recognise the remote's hostname, it probes the host's API over HTTPS to figure out what it is:

- a GitLab instance (self-hosted GitLab CE/EE),
- a GitHub Enterprise instance,
- or a Gitea / Forgejo instance.

The probe is unauthenticated, short (a ~2s timeout), best-effort, and cached: so a run checks a given host at most once, and a hostname FerrFlow already knows never touches the network. You only need to set `forge` explicitly now if your host isn't reachable over HTTPS or you'd rather skip the probe.
