#!/usr/bin/env bash
set -euo pipefail

cli=false
api=false
site=false

while IFS= read -r file; do
  [ -n "$file" ] || continue
  case "$file" in
    .github/workflows/ci.yml | .github/workflows/changes.yml | .github/workflows/sonarqube.yml | .github/workflows/fuzz.yml | .github/workflows/docker.yml | scripts/changed-paths.sh)
      cli=true
      api=true
      site=true
      ;;
    .dockerignore)
      cli=true
      api=true
      site=true
      ;;
    site/* | docs/site/* | package.json | pnpm-lock.yaml | pnpm-workspace.yaml | .npmrc | .prettierrc.json | .prettierignore)
      site=true
      ;;
    api/*)
      api=true
      ;;
    src/* | schema/* | ferrflow-wasm/* | Cargo.toml | Cargo.lock | rust-toolchain.toml | .cargo/*)
      cli=true
      api=true
      ;;
    *)
      cli=true
      ;;
  esac
done

printf 'cli=%s\napi=%s\nsite=%s\n' "$cli" "$api" "$site"
