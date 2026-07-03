#!/usr/bin/env bash
set -euo pipefail

# Profile-Guided Optimization build for the x86_64-unknown-linux-musl release
# binary (issue #619). Scoped to x64-linux only: a musl x86_64 binary runs
# natively on the x86 CI runner, so the instrument -> profile -> optimize loop
# is clean here. Every other release target keeps the LTO-only build.
#
# PGO is done with plain rustc flags (-Cprofile-generate / -Cprofile-use) rather
# than a helper crate, so the exact toolchain contract is visible. The optimized
# pass layers -Cprofile-use on top of the [profile.release] LTO from Cargo.toml.
#
# Usage: scripts/pgo-build.sh [target]
# Env:   FIXTURES_DIR  directory of generated fixture git repos (profiling load)

TARGET="${1:-x86_64-unknown-linux-musl}"
FIXTURES_DIR="${FIXTURES_DIR:-fixtures-generated}"
BIN_NAME="ferrflow"
PROF_DIR="$(pwd)/target/pgo-data"
BIN_PATH="$(pwd)/target/${TARGET}/release/${BIN_NAME}"

echo "==> PGO build for ${TARGET}"
rustup target add "$TARGET"
rustup component add llvm-tools-preview

LLVM_PROFDATA="$(find "$(rustc --print sysroot)" -name 'llvm-profdata*' -type f | head -1)"
if [ -z "${LLVM_PROFDATA}" ]; then
  echo "llvm-profdata not found in sysroot" >&2
  exit 1
fi

rm -rf "$PROF_DIR"
mkdir -p "$PROF_DIR"

echo "==> Instrumented build (-Cprofile-generate)"
RUSTFLAGS="-Cprofile-generate=${PROF_DIR}" \
  cargo build --release --target "$TARGET"

# Give the profiling run realistic commit/tag volume on the hot walking paths.
git fetch --unshallow 2>/dev/null || git fetch --depth=2000 2>/dev/null || true
git fetch --tags 2>/dev/null || true

echo "==> Collecting profiles"
# Breadth: each generated fixture exercises a distinct commit/version/changelog
# path. Some fixtures are intentionally malformed, so ignore exit codes.
if [ -d "$FIXTURES_DIR" ]; then
  while IFS= read -r gitdir; do
    repo="$(dirname "$gitdir")"
    ( cd "$repo" && "$BIN_PATH" check >/dev/null 2>&1 ) || true
    ( cd "$repo" && "$BIN_PATH" version >/dev/null 2>&1 ) || true
  done < <(find "$FIXTURES_DIR" -type d -name .git)
fi
# Volume: this repo's own deep history — the closest always-available stand-in
# for a large monorepo on the commit-walking / tag-index hot paths.
"$BIN_PATH" check >/dev/null 2>&1 || true
"$BIN_PATH" version >/dev/null 2>&1 || true
"$BIN_PATH" tag >/dev/null 2>&1 || true

shopt -s nullglob
profraws=("$PROF_DIR"/*.profraw)
if [ ${#profraws[@]} -eq 0 ]; then
  echo "no .profraw profiles were produced — instrumented run collected nothing" >&2
  exit 1
fi
echo "==> Merging ${#profraws[@]} profile(s)"
"$LLVM_PROFDATA" merge -o "$PROF_DIR/merged.profdata" "${profraws[@]}"

echo "==> Optimized build (-Cprofile-use + release LTO)"
RUSTFLAGS="-Cprofile-use=${PROF_DIR}/merged.profdata -Cllvm-args=-pgo-warn-missing-function" \
  cargo build --release --target "$TARGET"

echo "==> PGO build complete: target/${TARGET}/release/${BIN_NAME}"
