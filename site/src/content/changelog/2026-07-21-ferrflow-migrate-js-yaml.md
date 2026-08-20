---
title: 'FerrFlow · Migrate reads JS and YAML configs'
summary: '`ferrflow migrate` now imports JavaScript and YAML release configs directly — `.releaserc.js`, `release.config.js`, `.releaserc.yaml`, `.versionrc.js`, and friends — not just JSON, so you no longer have to hand-convert your config before migrating.'
date: 2026-07-21T14:00:00Z
product: ferrflow
type: new
prLink: https://github.com/FerrLabs/FerrFlow/pull/750
docsLink: https://ferrflow.com/docs/reference/cli
---

Plenty of semantic-release and standard-version setups keep their config in JavaScript (`release.config.js`) or YAML (`.releaserc.yaml`) rather than JSON. `ferrflow migrate` used to read JSON only and told you to convert the rest by hand first.

Now it reads them directly. A JavaScript config is evaluated with `node` — it imports the config, resolves it if it's a function, and reads the result — the same way FerrFlow already runs your `ferrflow.js` config, run locally against your own repo. A YAML config is parsed straight through. Both then feed the existing converters, so the mapping and the mapped/ignored/review report are identical to the JSON path.

```bash
ferrflow migrate            # auto-detects .releaserc.js, .releaserc.yaml, release.config.js, …
```

JavaScript configs need Node.js on your PATH; if it's missing, the error says so.
