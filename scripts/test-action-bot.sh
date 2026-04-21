#!/usr/bin/env bash
# Best-effort lint for action.yml.
# Verifies the YAML parses and runs actionlint if installed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ACTION_FILE="$ROOT/action.yml"

if [ ! -f "$ACTION_FILE" ]; then
  echo "action.yml not found at $ACTION_FILE" >&2
  exit 1
fi

if command -v python3 >/dev/null 2>&1; then
  python3 -c "import sys, yaml; yaml.safe_load(open('$ACTION_FILE'))" \
    && echo "action.yml parses as valid YAML"
elif command -v python >/dev/null 2>&1; then
  python -c "import sys, yaml; yaml.safe_load(open('$ACTION_FILE'))" \
    && echo "action.yml parses as valid YAML"
else
  echo "python not found; skipping YAML parse check"
fi

if command -v actionlint >/dev/null 2>&1; then
  actionlint "$ACTION_FILE"
  echo "actionlint passed"
else
  echo "actionlint not installed; skipping"
fi
