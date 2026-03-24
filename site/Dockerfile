# Build stage
FROM node:22-alpine AS builder

WORKDIR /app

RUN npm install -g pnpm@9

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY packages/shared/package.json ./packages/shared/
COPY packages/site/package.json ./packages/site/
COPY packages/app/package.json ./packages/app/
COPY packages/mcp/package.json ./packages/mcp/

RUN pnpm install --frozen-lockfile

COPY packages/shared ./packages/shared
COPY packages/site ./packages/site

RUN pnpm --filter @ferrflow/site build

# Runtime stage
FROM nginx:alpine

COPY --from=builder /app/packages/site/dist /usr/share/nginx/html

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
