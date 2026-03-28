# Build stage
FROM docker.io/library/node:22-alpine AS builder

WORKDIR /app

RUN npm install -g pnpm@9

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY packages/site/package.json ./packages/site/

RUN pnpm install --frozen-lockfile

COPY packages/site ./packages/site

RUN pnpm --filter @ferrflow/site build

# Runtime stage
FROM docker.io/library/nginx:alpine

COPY --from=builder /app/packages/site/dist /usr/share/nginx/html

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
