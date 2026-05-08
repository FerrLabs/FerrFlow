#!/usr/bin/env bash
set -euo pipefail
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/pre-push 2>/dev/null || true
echo "Git hooks installed (core.hooksPath=.githooks)."
echo "pre-commit: cargo fmt --check + cargo clippy -D warnings (when .rs files staged)"
echo "pre-push:   cargo fmt --check + cargo clippy --all-targets -- -D warnings"
