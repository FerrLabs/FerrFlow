#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: publish-wasm.sh <version>}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"
WASM_DIR="${REPO_DIR}/ferrflow-wasm"

echo "Building @ferrflow/wasm@${VERSION}..."

cd "$WASM_DIR"
wasm-pack build --target bundler --scope ferrflow

cd pkg

node -e "
  const fs = require('fs');
  const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
  pkg.name = '@ferrflow/wasm';
  pkg.version = '${VERSION}';
  pkg.repository = {
    type: 'git',
    url: 'git+https://github.com/FerrLabs/FerrFlow.git',
    directory: 'ferrflow-wasm'
  };
  pkg.homepage = 'https://ferrflow.com';
  fs.writeFileSync('package.json', JSON.stringify(pkg, null, 2) + '\n');
"

if npm view "@ferrflow/wasm@${VERSION}" version >/dev/null 2>&1; then
  echo "@ferrflow/wasm@${VERSION} already on registry — skipping"
else
  echo "Publishing @ferrflow/wasm@${VERSION}..."
  npm publish --access public
fi

echo "Published @ferrflow/wasm@${VERSION}"
