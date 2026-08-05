---
title: Self-hosting
description: Run FerrFlow on your own infrastructure with a single Docker image.
---

The self-host bundle is a single Docker image that contains the API and the
dashboard. The API serves the dashboard assets on the same origin, so there is
only one container to run and no CORS configuration to manage.

Image: `ghcr.io/ferrlabs/ferrflow-selfhost:latest`

## Prerequisites

- Docker 24+ and Docker Compose
- A PostgreSQL 16 database with the TimescaleDB extension (the reference compose
  file below provisions one for you)

## Quickstart with Docker Compose

Download the reference compose file and environment template from the
[Application repository](https://github.com/FerrLabs/Application):

```bash
curl -O https://raw.githubusercontent.com/FerrLabs/Application/main/docker-compose.selfhost.yml
curl -o .env https://raw.githubusercontent.com/FerrLabs/Application/main/.env.example.selfhost
```

Generate secrets and edit `.env`:

```bash
# Database password (any long random string)
echo "DB_PASSWORD=$(openssl rand -base64 32)" >> .env.secrets
# JWT signing secret
echo "JWT_SECRET=$(openssl rand -base64 64)" >> .env.secrets
# AES-256-GCM key for encrypting secrets at rest (must be base64-encoded 32 bytes)
echo "ENCRYPTION_KEY=$(openssl rand -base64 32)" >> .env.secrets
# HMAC secret used by the FerrFlow CLI telemetry endpoint
echo "FERRFLOW_HMAC_SECRET=$(openssl rand -hex 32)" >> .env.secrets
```

Merge `.env.secrets` into `.env` manually, then start the stack:

```bash
docker compose -f docker-compose.selfhost.yml up -d
```

The dashboard is now available on `http://localhost:3000`.

## Environment variables

| Variable                   | Required               | Purpose                                               |
| -------------------------- | ---------------------- | ----------------------------------------------------- |
| `DATABASE_URL`             | yes                    | Postgres connection string                            |
| `JWT_SECRET`               | yes                    | Signing key for session JWTs                          |
| `ENCRYPTION_KEY`           | yes                    | 32-byte base64 key for the secrets at-rest encryption |
| `FERRFLOW_HMAC_SECRET`     | yes                    | HMAC key for CLI telemetry ingestion                  |
| `SERVER_HOST`              | no (default `0.0.0.0`) | Bind address                                          |
| `SERVER_PORT`              | no (default `3000`)    | Bind port                                             |
| `DATABASE_MAX_CONNECTIONS` | no (default `10`)      | Postgres pool size                                    |
| `RATE_LIMIT_REQUESTS`      | no (default `60`)      | Requests per minute before rate-limiting              |
| `RUST_LOG`                 | no (default `info`)    | Log level                                             |

## Using an external Postgres

Remove the `db` service and the `depends_on` block from the compose file, then
point `DATABASE_URL` at your existing instance. The TimescaleDB extension is
required — create it once with:

```sql
CREATE EXTENSION IF NOT EXISTS timescaledb;
```

Migrations run automatically on startup.

## Reverse proxy and TLS

Terminate TLS at a proxy in front of the container. Example Caddyfile:

```caddy
ferrflow.example.com {
  reverse_proxy localhost:3000
}
```

No CORS origins need to be whitelisted because the browser only talks to the
single proxy origin.

## Updating

```bash
docker compose -f docker-compose.selfhost.yml pull
docker compose -f docker-compose.selfhost.yml up -d
```

Database migrations are applied on startup. Take a database backup before
upgrading across a major version.

## Health check

`GET /health` returns `200` once the API has connected to Postgres and applied
migrations.
