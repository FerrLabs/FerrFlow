#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: publish.sh <version>}"

# Map: archive name -> platform dir
declare -A ARCHIVES=(
  ["ferrflow-linux-x64.tar.gz"]="linux-x64"
  ["ferrflow-linux-arm64.tar.gz"]="linux-arm64"
  ["ferrflow-darwin-x64.tar.gz"]="darwin-x64"
  ["ferrflow-darwin-arm64.tar.gz"]="darwin-arm64"
  ["ferrflow-windows-x64.zip"]="win32-x64"
)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
NPM_DIR="$(dirname "$SCRIPT_DIR")"
WORK_DIR="$(mktemp -d)"

echo "Downloading release binaries for v${VERSION}..."

for archive in "${!ARCHIVES[@]}"; do
  platform="${ARCHIVES[$archive]}"
  echo "  ${archive} -> @ferrflow/${platform}"

  gh release download "v${VERSION}" -p "$archive" -D "$WORK_DIR"

  # Prepare platform package
  pkg_dir="${WORK_DIR}/packages/${platform}"
  mkdir -p "${pkg_dir}/bin"

  # Extract binary
  if [[ "$archive" == *.zip ]]; then
    unzip -q "${WORK_DIR}/${archive}" -d "${pkg_dir}/bin/"
  else
    tar xzf "${WORK_DIR}/${archive}" -C "${pkg_dir}/bin/"
  fi

  # Copy package.json, README, and LICENSE
  cp "${NPM_DIR}/platforms/${platform}/package.json" "${pkg_dir}/package.json"
  cp "${NPM_DIR}/../README.md" "${pkg_dir}/README.md"
  cp "${NPM_DIR}/../LICENSE" "${pkg_dir}/LICENSE" 2>/dev/null || true
  cd "$pkg_dir"
  npm version "$VERSION" --no-git-tag-version --allow-same-version
  npm publish --access public
  cd - > /dev/null
done

# Publish main wrapper
echo "Publishing main package ferrflow@${VERSION}..."
cp "${NPM_DIR}/../README.md" "${NPM_DIR}/README.md"
cp "${NPM_DIR}/../LICENSE" "${NPM_DIR}/LICENSE" 2>/dev/null || true
cd "$NPM_DIR"

# Update version and optionalDependencies versions
npm version "$VERSION" --no-git-tag-version --allow-same-version

node -e "
  const pkg = JSON.parse(require('fs').readFileSync('package.json', 'utf8'));
  for (const dep of Object.keys(pkg.optionalDependencies || {})) {
    pkg.optionalDependencies[dep] = '${VERSION}';
  }
  require('fs').writeFileSync('package.json', JSON.stringify(pkg, null, 2) + '\n');
"

npm publish --access public

echo "Published ferrflow@${VERSION} with all platform packages"

# Cleanup
rm -rf "$WORK_DIR"
