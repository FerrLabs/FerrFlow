#!/usr/bin/env bash
set -euo pipefail
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/pre-push 2>/dev/null || true
echo "Git hooks installed (core.hooksPath=.githooks)."
echo "pre-commit: cargo fmt --check + cargo clippy -D warnings"
echo "pre-push:   cargo test --workspace --all-features"
