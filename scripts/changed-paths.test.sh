#!/usr/bin/env bash
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
failures=0

expect() {
  local expected=$1
  shift
  local actual
  actual=$(printf '%s\n' "$@" | "$here/changed-paths.sh" | paste -sd' ')
  if [ "$actual" != "$expected" ]; then
    echo "FAIL: [$*] expected '$expected' got '$actual'" >&2
    failures=$((failures + 1))
  fi
}

expect "cli=false api=false site=true" site/src/app/app.ts
expect "cli=false api=false site=true" docs/site/docs-en/index.md site/pnpm-lock.yaml
expect "cli=false api=false site=true" site/package.json site/pnpm-lock.yaml
expect "cli=true api=false site=false" pnpm-lock.yaml
expect "cli=false api=true site=false" api/src/lib.rs api/Dockerfile
expect "cli=true api=true site=false" src/main.rs
expect "cli=true api=true site=false" Cargo.lock
expect "cli=true api=true site=false" schema/ferrflow.json
expect "cli=true api=false site=false" tests/fixtures/run-tests.sh
expect "cli=true api=false site=false" README.md
expect "cli=true api=false site=false" npm/package.json
expect "cli=true api=true site=true" .github/workflows/ci.yml
expect "cli=true api=true site=true" .dockerignore
expect "cli=true api=false site=true" site/src/main.ts README.md
expect "cli=false api=false site=false"

[ "$failures" -eq 0 ]
