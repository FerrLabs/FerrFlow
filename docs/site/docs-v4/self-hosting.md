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

| Variable                      | Required               | Purpose                                                      |
| ----------------------------- | ---------------------- | ------------------------------------------------------------ |
| `DATABASE_URL`                | yes                    | Postgres connection string                                   |
| `JWT_SECRET`                  | yes                    | Signing key for session JWTs                                 |
| `ENCRYPTION_KEY`              | yes                    | 32-byte base64 key for the secrets at-rest encryption        |
| `FERRFLOW_HMAC_SECRET`        | yes                    | HMAC key for CLI telemetry ingestion                         |
| `SERVER_HOST`                 | no (default `0.0.0.0`) | Bind address                                                 |
| `SERVER_PORT`                 | no (default `3000`)    | Bind port                                                    |
| `DATABASE_MAX_CONNECTIONS`    | no (default `10`)      | Postgres pool size                                           |
| `RATE_LIMIT_REQUESTS`         | no (default `60`)      | Requests per minute before rate-limiting                     |
| `RUST_LOG`                    | no (default `info`)    | Log level                                                    |
| `FERRFLOW_DISABLE_AUTO_ADMIN` | no (default unset)     | Set to `false` to opt into env-driven admin bootstrap        |
| `FERRFLOW_ADMIN_EMAIL`        | conditional            | Email for the bootstrap staff user (required when opting in) |
| `FERRFLOW_ADMIN_PASSWORD`     | conditional            | Password for the bootstrap staff user (8–128 chars)          |

## Bootstrap admin

On first boot, self-hosters usually need a staff user and an organization to
log in to. Instead of running SQL by hand you can provision one from the
environment.

Opt in explicitly by setting:

```bash
FERRFLOW_DISABLE_AUTO_ADMIN=false
FERRFLOW_ADMIN_EMAIL=you@example.com
FERRFLOW_ADMIN_PASSWORD=a-long-random-password
```

On startup (after migrations, before the HTTP server binds) the API will:

1. Create a user with `is_staff=true` and `email_verified=true`.
2. Create a personal organization owned by that user.
3. Log a single info line containing the email (never the password).

Rules:

- Both `FERRFLOW_ADMIN_EMAIL` and `FERRFLOW_ADMIN_PASSWORD` must be set, or
  neither. Setting only one aborts boot with a clear error.
- If a staff user already exists, bootstrap is a no-op.
- Any value other than the exact string `false` on
  `FERRFLOW_DISABLE_AUTO_ADMIN` (including unset) is treated as disabled.
  That keeps hosted deployments safe from accidental enablement.

Once bootstrap has run, unset the three variables — the API doesn't need
them again, and leaving the password in the environment is needless
exposure.

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
