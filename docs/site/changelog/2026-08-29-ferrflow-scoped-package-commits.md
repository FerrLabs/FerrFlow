---
title: 'FerrFlow: a package release no longer carries other packages'' commits'
summary: 'In a monorepo, each package collected every commit since its own last tag, whatever those commits touched. That padded the changelog with other packages'' work and, less visibly, let their commits decide the version bump.'
date: 2026-08-29T12:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/974
docsLink: https://ferrflow.com/docs/configuration/monorepo/
---

Path prefixes decide which package a commit belongs to. That is what the documentation says, and it is how FerrFlow decided whether a package had anything to release. It was not how FerrFlow decided what that release contained.

Each package took every commit since its own last tag. A repository with a site and a server would publish the site with the server's refactors listed underneath it, because those commits sat in the window even though nothing in them touched the site.

`ferrflow diff` was fixed to scope its range by path back in August, on the understanding that a release already worked that way. It did not, so from that point until now the preview and the release disagreed with each other, and the preview was the one telling the truth. They agree again.

The visible half is a changelog claiming work that shipped elsewhere. The half worth knowing about is the version. The bump is the strongest change in that list, so a `feat` anywhere in the repository turned every package released alongside it into a minor. A one-line fix to a site could go out as `1.1.0` because a server gained an endpoint the same week.

Releases now carry only the commits that touch the package, matching `path` and any `sharedPaths` it declares. Nothing to configure, and `sharedPaths` behaves as before: a commit under a shared directory belongs to every package that declares it, and to no other.

Expect quieter changelogs and, in mixed repositories, smaller version bumps than you have been getting. The bumps you were getting were wrong, so this is worth a look if you have release automation keyed to major or minor.

The effect is largest with `recoverMissedReleases` enabled, since recovery reaches back to the last tag rather than the last release, and everything the other packages did in between came along.
