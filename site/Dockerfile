# syntax=docker/dockerfile:1
# Build stage
FROM docker.io/library/node:26-alpine AS builder

WORKDIR /app

ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0
RUN npm install -g corepack@0.36.0 && corepack enable

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml .npmrc ./
COPY site/package.json ./site/

RUN pnpm install --frozen-lockfile --filter @ferrflow/site...

COPY docs/site ./docs/site
COPY site ./site

ENV NODE_OPTIONS=--max-old-space-size=4096
RUN pnpm --filter @ferrflow/site build

# Runtime stage — Analog static output (prerendered HTML + client bundles).
FROM docker.io/nginxinc/nginx-unprivileged:alpine

USER root
RUN apk upgrade --no-cache
USER nginx

COPY --from=builder /app/site/dist/analog/public /usr/share/nginx/html
COPY site/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]
