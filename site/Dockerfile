# syntax=docker/dockerfile:1
# Build stage
FROM docker.io/library/node:24-alpine AS builder

WORKDIR /app

ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0
RUN corepack enable

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml .npmrc ./
COPY site/package.json ./site/

# NODE_AUTH_TOKEN authenticates pnpm against GitHub Packages for any private
# @ferrlabs/* packages. Mounted as a BuildKit secret (id=packages-read), never
# a build-arg, so it never lands in an image layer or the build log. pnpm won't
# expand it from the committed .npmrc, so it's written to a trusted config for
# the install and removed in the same layer.
RUN --mount=type=secret,id=packages-read \
    pnpm config set "//npm.pkg.github.com/:_authToken" "$(cat /run/secrets/packages-read)" \
    && pnpm install --frozen-lockfile --filter @ferrflow/site... \
    && pnpm config delete "//npm.pkg.github.com/:_authToken"

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
