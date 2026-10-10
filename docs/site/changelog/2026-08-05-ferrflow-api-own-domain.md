---
title: 'FerrFlow: the hosted API moves to api.ferrflow.com'
summary: 'The FerrFlow API and the hosted bot token exchange now answer on api.ferrflow.com. The old api.ferrlabs.com host keeps working indefinitely.'
date: 2026-08-05T10:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow-Cloud/pull/772
docsLink: https://ferrflow.com/docs/reference/api
---

The hosted FerrFlow API: `validate`, `preview`, `latest`, the config schema, and the bot token exchange: was published under the holding company's hostname, `api.ferrlabs.com`. Pointing a `$schema` at a *ferrlabs* URL to validate a *ferrflow* config never read right, and every other product already serves its API from its own domain. It now answers on `api.ferrflow.com`, which is the host the documentation uses from here on.

Paths are unchanged, so moving over is a hostname swap and nothing else:

```bash
curl "https://api.ferrflow.com/v1/ferrflow/latest?platform=linux-x64"
```

```json
{ "$schema": "https://api.ferrflow.com/v1/ferrflow/schema" }
```

**Nothing breaks.** `api.ferrlabs.com/v1/ferrflow/*` reaches exactly the same endpoints and will keep doing so indefinitely. It is the same service behind a second hostname, not a redirect and not a deprecation. FerrFlow versions already released have the old endpoint compiled in and continue to work untouched; new versions of the CLI and the GitHub Action exchange their bot token on the new host. If you override the endpoint yourself, `bot_endpoint:` on the Action and `FERRFLOW_BOT_ENDPOINT` in the CLI still take precedence.

One thing that deliberately did **not** move: the OIDC audience stays `ferrflow.ferrlabs.com`. It is the identifier your runner mints the token for, not a URL, and the service validates against that exact string.
