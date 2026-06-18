# Build stage
FROM docker.io/library/node:24-alpine AS builder

WORKDIR /app

# NODE_AUTH_TOKEN authenticates pnpm against GitHub Packages for any private
# @ferrlabs/* packages. Passed as a build-arg; the workflow's ephemeral token.
ARG NODE_AUTH_TOKEN

ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0
RUN corepack enable

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml .npmrc ./
COPY packages/site/package.json ./packages/site/

# pnpm ignores the ${NODE_AUTH_TOKEN} in the committed .npmrc, so the token
# must live in a trusted config for supply-chain verification to authenticate.
RUN pnpm config set "//npm.pkg.github.com/:_authToken" "${NODE_AUTH_TOKEN}"

RUN pnpm install --frozen-lockfile --filter @ferrflow/site...

COPY packages/site ./packages/site

ENV NODE_OPTIONS=--max-old-space-size=4096
RUN pnpm --filter @ferrflow/site build

# Runtime stage — Analog static output (prerendered HTML + client bundles).
FROM docker.io/nginxinc/nginx-unprivileged:alpine

USER root
RUN apk upgrade --no-cache
USER nginx

COPY --from=builder /app/packages/site/dist/analog/public /usr/share/nginx/html
COPY packages/site/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]
