---
title: 'FerrFlow: Post-publish hooks get the release URL'
summary: '`postPublish` hooks now receive `releaseUrl`: the URL of the GitHub/GitLab release FerrFlow just created: so you can link to it from a Slack notification, a dashboard, or a downstream job.'
date: 2026-07-21T08:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/743
docsLink: https://ferrflow.com/docs/configuration/config-file
---

A `postPublish` hook knew the version and tag, but not where the release actually landed. To announce a release with a link, you had to reconstruct the URL by hand.

FerrFlow now hands it to you. Once the forge release is created, `postPublish` hooks receive `releaseUrl`: the canonical URL of the GitHub or GitLab release for that package's tag. Shell hooks read `FERRFLOW_RELEASE_URL`; JS/TS function hooks get `ctx.releaseUrl`.

```js
export default {
  workspace: {
    hooks: {
      postPublish(ctx) {
        if (ctx.releaseUrl) notify(`${ctx.package}@${ctx.newVersion} → ${ctx.releaseUrl}`);
      },
    },
  },
};
```

It's `null` wherever the URL isn't known yet: at `prePublish` (the release isn't created until after), on `--dry-run`, and when no forge is configured.
