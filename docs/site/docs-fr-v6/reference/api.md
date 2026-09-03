---
title: API FerrFlow
description: Endpoints HTTP hébergés pour FerrFlow — valider une config, prévisualiser les montées de version, résoudre la dernière release et récupérer le schéma de config.
---

L'API FerrFlow expose un petit ensemble d'endpoints HTTP hébergés sous `https://api.ferrflow.com/v1/ferrflow/*`. Ils s'appuient sur le même cœur FerrFlow que le CLI : `validate` et `preview` renvoient donc des résultats identiques à `ferrflow validate` et `ferrflow check` — aucune seconde implémentation qui pourrait diverger.

Chaque endpoint est public (sans authentification) et sûr à appeler depuis la CI, un éditeur ou un navigateur. Le contrat lisible par machine est servi sur [`/v1/ferrflow/openapi.json`](https://api.ferrflow.com/v1/ferrflow/openapi.json) (OpenAPI 3.1).

`https://api.ferrlabs.com/v1/ferrflow/*` atteint les mêmes endpoints et continuera de fonctionner indéfiniment — c'est là que l'API a été publiée en premier, et les versions du CLI déjà diffusées l'appellent toujours. Préférez `api.ferrflow.com` pour tout nouvel usage.

## `GET /v1/ferrflow/health`

Sonde de disponibilité et de version. Alimente les tableaux de bord d'état.

```json
{ "status": "ok", "service": "ferrflow-api", "version": "10.17.0", "time": "2026-07-21T15:00:00Z" }
```

## `GET /v1/ferrflow/schema`

Renvoie le schéma JSON de la config (`Content-Type: application/schema+json`), servi depuis le schéma embarqué dans la release FerrFlow — les octets exacts que le CLI utilise pour valider. Envoie un `ETag` fort et un `Cache-Control`, alors pointez le `$schema` de votre éditeur ici :

```json
{ "$schema": "https://api.ferrflow.com/v1/ferrflow/schema" }
```

`GET /v1/ferrflow/schema/v{major}` renvoie le schéma figé à un majeur du CLI (par ex. `/schema/v5`). Seul le majeur courant est servi aujourd'hui ; les majeurs plus anciens renvoient `404` jusqu'à l'arrivée des instantanés par majeur.

## `GET /v1/ferrflow/latest`

Résout la dernière release FerrFlow depuis GitHub, mise en cache côté serveur. Passez `platform` pour obtenir un seul artefact :

```bash
curl "https://api.ferrflow.com/v1/ferrflow/latest?platform=linux-x64"
```

```json
{
  "version": "5.48.0",
  "tag": "v5.48.0",
  "platform": "linux-x64",
  "download_url": "https://github.com/FerrLabs/FerrFlow/releases/download/v5.48.0/ferrflow-linux-x64.tar.gz",
  "bundle_url": "https://github.com/FerrLabs/FerrFlow/releases/download/v5.48.0/ferrflow-linux-x64.tar.gz.bundle",
  "published_at": "2026-07-27T19:20:00Z"
}
```

Les releases sont signées avec [Sigstore](/fr/verifying-releases/) — vérifiez le `.bundle` plutôt qu'une somme de contrôle (les releases jusqu'à v5.47.4 embarquent une paire `.sig` + `.crt`). Sans `platform`, la réponse liste les `assets` de chaque plateforme. Plateformes valides : `linux-x64`, `linux-arm64`, `linux-arm`, `darwin-x64`, `darwin-arm64`, `win32-x64`, `win32-arm64`.

## `POST /v1/ferrflow/validate`

Valide une config sans dépôt — vous envoyez le texte de la config et, en option, le contenu des fichiers versionnés qu'elle référence pour que les vérifications d'existence et de cohérence des versions s'exécutent. Le résultat est identique à `ferrflow validate --json`.

```bash
curl -X POST https://api.ferrflow.com/v1/ferrflow/validate \
  -H 'content-type: application/json' \
  -d '{
    "config": "{\"package\":[{\"name\":\"app\",\"path\":\".\",\"versionedFiles\":[{\"path\":\"package.json\",\"format\":\"json\"}]}]}",
    "files": { "package.json": "{\"version\":\"1.0.0\"}" }
  }'
```

```json
{
  "valid": true,
  "config_file": null,
  "package_count": 1,
  "errors": [],
  "warnings": [],
  "suggestions": []
}
```

Une config invalide reste une validation réussie : la réponse est `200` avec `"valid": false` et les entrées fautives. Seul un corps de requête malformé renvoie `400`. Le champ optionnel `format` (`json` | `json5` | `toml`) court-circuite la détection de format.

## `POST /v1/ferrflow/preview`

Calcule les montées de version et le changelog pour une liste explicite de commits — la même logique que `ferrflow check`, sous forme de service. Aucun accès au dépôt ; vous passez les commits.

```bash
curl -X POST https://api.ferrflow.com/v1/ferrflow/preview \
  -H 'content-type: application/json' \
  -d '{
    "config": "{\"package\":[{\"name\":\"api\",\"path\":\".\"}]}",
    "commits": [{ "message": "feat(api): add endpoint", "hash": "a1b2" }],
    "current_versions": { "api": "1.2.3" }
  }'
```

```json
{
  "packages": [
    {
      "name": "api",
      "current": "1.2.3",
      "next": "1.3.0",
      "bump": "minor",
      "commits": [{ "hash": "a1b2", "type": "feat", "scope": "api", "breaking": false }],
      "changelog": "### Features\n- ..."
    }
  ]
}
```

Dans une config monorepo, chaque commit est affecté à un package quand ses `files` se trouvent sous le `path` de ce package. Les packages sans commit publiable sont omis.
