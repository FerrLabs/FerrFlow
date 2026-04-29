# Build stage
FROM docker.io/library/node:24-alpine AS builder

WORKDIR /app

# NODE_AUTH_TOKEN authenticates pnpm against GitHub Packages so it can
# resolve private @ferrlabs/* packages (ui-astro, styles, …).
ARG NODE_AUTH_TOKEN

RUN npm install -g pnpm@10

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml .npmrc ./
COPY packages/site/package.json ./packages/site/

RUN NODE_AUTH_TOKEN="${NODE_AUTH_TOKEN}" pnpm install --frozen-lockfile --filter @ferrflow/site...

COPY packages/site ./packages/site

RUN pnpm --filter @ferrflow/site build

# Runtime stage
FROM docker.io/nginxinc/nginx-unprivileged:alpine

USER root
RUN apk upgrade --no-cache
USER nginx

COPY --from=builder /app/packages/site/dist /usr/share/nginx/html
COPY packages/site/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]
