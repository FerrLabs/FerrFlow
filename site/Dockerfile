# Build stage
FROM docker.io/library/node:22-alpine AS builder

WORKDIR /app

RUN npm install -g pnpm@9

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY packages/shared/package.json ./packages/shared/
COPY packages/site/package.json ./packages/site/
COPY packages/docs/package.json ./packages/docs/

RUN pnpm install --frozen-lockfile

COPY packages/shared ./packages/shared
COPY packages/site ./packages/site
COPY packages/docs ./packages/docs

RUN pnpm --filter @ferrflow/site build
RUN pnpm --filter @ferrflow/docs build

# Runtime stage
FROM docker.io/library/nginx:alpine

COPY --from=builder /app/packages/site/dist /usr/share/nginx/html
COPY --from=builder /app/packages/docs/dist /usr/share/nginx/html/docs
COPY packages/site/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
