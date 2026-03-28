#!/usr/bin/env bash
set -euo pipefail

# Generate synthetic git repos for benchmarking.
# Usage: ./generate.sh <output_dir>
#
# Creates four fixtures:
#   single/       — single-package repo, ~100 commits
#   mono-small/   — 10 packages, ~100 commits
#   mono-medium/  — 50 packages, ~500 commits
#   mono-large/   — 200 packages, ~10000 commits

OUTPUT_DIR="${1:-.}"
COMMIT_TYPES=("feat" "fix" "refactor" "perf" "chore" "docs" "ci" "test")
SCOPES=("core" "api" "cli" "config" "parser" "auth" "db" "cache" "logging" "events")

rand_element() {
  local arr=("$@")
  echo "${arr[$((RANDOM % ${#arr[@]}))]}"
}

rand_word() {
  local words=("update" "add" "remove" "refactor" "improve" "fix" "handle" "support" "implement" "optimize")
  echo "$(rand_element "${words[@]}") $(rand_element "feature" "endpoint" "handler" "logic" "validation" "error" "check" "flow" "config" "output")"
}

# Generate a random date within the past year
rand_date() {
  local days_ago=$((RANDOM % 365))
  local hours=$((RANDOM % 24))
  local mins=$((RANDOM % 60))
  date -u -d "$days_ago days ago $hours hours $mins minutes" +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null \
    || date -u -v-${days_ago}d -v${hours}H -v${mins}M +"%Y-%m-%dT%H:%M:%SZ"
}

make_commit() {
  local type scope msg breaking ts
  type=$(rand_element "${COMMIT_TYPES[@]}")
  scope=$(rand_element "${SCOPES[@]}")
  msg=$(rand_word)
  ts=$(rand_date)

  # ~5% chance of breaking change
  if (( RANDOM % 20 == 0 )); then
    breaking="!"
  else
    breaking=""
  fi

  echo "change" >> dummy.txt
  git add -A
  GIT_AUTHOR_DATE="$ts" GIT_COMMITTER_DATE="$ts" \
    git commit -q -m "${type}(${scope})${breaking}: ${msg}" --allow-empty
}

create_single() {
  local dir="$OUTPUT_DIR/single"
  rm -rf "$dir"
  mkdir -p "$dir"
  cd "$dir"
  git init -q
  git config user.email "bench@ferrflow.dev"
  git config user.name "FerrFlow Bench"
  git checkout -q -b main

  # Create .ferrflow config
  cat > .ferrflow <<'JSON'
{
  "package": [
    {
      "name": "myapp",
      "path": ".",
      "changelog": "CHANGELOG.md",
      "versioned_files": [
        { "path": "package.json", "format": "json" }
      ]
    }
  ]
}
JSON

  # Create package.json
  cat > package.json <<'JSON'
{
  "name": "myapp",
  "version": "0.1.0"
}
JSON

  touch dummy.txt
  git add -A
  git commit -q -m "chore: initial commit"
  git tag "v0.1.0"

  for _ in $(seq 1 100); do
    make_commit
  done

  echo "Created single fixture: 100 commits"
  cd - > /dev/null
}

create_mono() {
  local name="$1" pkg_count="$2" commit_count="$3"
  local dir="$OUTPUT_DIR/$name"
  rm -rf "$dir"
  mkdir -p "$dir"
  cd "$dir"
  git init -q
  git config user.email "bench@ferrflow.dev"
  git config user.name "FerrFlow Bench"
  git checkout -q -b main

  # Generate packages
  local packages=()
  local config_packages=""
  for i in $(seq 1 "$pkg_count"); do
    local pkg_name="pkg-$(printf '%03d' "$i")"
    packages+=("$pkg_name")
    mkdir -p "packages/$pkg_name"
    cat > "packages/$pkg_name/package.json" <<JSON
{
  "name": "$pkg_name",
  "version": "0.1.0"
}
JSON
    if [ -n "$config_packages" ]; then
      config_packages="$config_packages,"
    fi
    config_packages="$config_packages
    {
      \"name\": \"$pkg_name\",
      \"path\": \"packages/$pkg_name\",
      \"changelog\": \"packages/$pkg_name/CHANGELOG.md\",
      \"versioned_files\": [
        { \"path\": \"packages/$pkg_name/package.json\", \"format\": \"json\" }
      ]
    }"
  done

  cat > .ferrflow <<JSON
{
  "package": [$config_packages
  ]
}
JSON

  touch dummy.txt
  git add -A
  git commit -q -m "chore: initial commit"

  # Tag all packages at v0.1.0
  for pkg in "${packages[@]}"; do
    git tag "${pkg}@v0.1.0"
  done

  # Generate commits touching random packages
  for _ in $(seq 1 "$commit_count"); do
    local pkg=$(rand_element "${packages[@]}")
    echo "change" >> "packages/$pkg/dummy.txt"
    local type=$(rand_element "${COMMIT_TYPES[@]}")
    local msg=$(rand_word)
    local ts=$(rand_date)
    git add -A
    GIT_AUTHOR_DATE="$ts" GIT_COMMITTER_DATE="$ts" \
      git commit -q -m "${type}(${pkg}): ${msg}" --allow-empty
  done

  echo "Created $name fixture: $pkg_count packages, $commit_count commits"
  cd - > /dev/null
}

echo "Generating benchmark fixtures..."
create_single
create_mono "mono-small" 10 100
create_mono "mono-medium" 50 500
create_mono "mono-large" 200 10000
echo "Done."
