#!/usr/bin/env bash
set -euo pipefail

# Benchmark regression detector
#
# Usage: ./compare.sh <baseline.json> <latest.json>
#
# Exits 0 if no regressions, 1 if any threshold exceeded.
# Requires: jq

BASELINE="${1:-}"
LATEST="${2:-}"

if [[ -z "$BASELINE" || -z "$LATEST" ]]; then
  echo "Usage: $0 <baseline.json> <latest.json>" >&2
  exit 1
fi

if [[ ! -f "$BASELINE" ]]; then
  echo "No baseline found at $BASELINE -- skipping regression check (first run?)" >&2
  exit 0
fi

if [[ ! -f "$LATEST" ]]; then
  echo "No results found at $LATEST" >&2
  exit 1
fi

require_cmd() {
  if ! command -v "$1" &>/dev/null; then
    echo "Required command not found: $1" >&2
    exit 1
  fi
}

require_cmd jq

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

RELATIVE_THRESHOLD=0.25  # 25%
BINARY_SIZE_THRESHOLD=0.20  # 20%

# Absolute thresholds (ms)
declare -A ABS_THRESHOLDS=(
  ["ferrflow|mono-large|check"]=2000
  ["ferrflow|mono-large|version"]=500
  ["ferrflow|mono-large|tag"]=500
)

# ---------------------------------------------------------------------------
# Check
# ---------------------------------------------------------------------------

FAILED=false

echo "Benchmark regression check"
echo "=========================="
echo ""

# Relative checks (ferrflow benchmarks only)
for key in $(jq -r '.benchmarks | keys[]' "$LATEST" | grep '^ferrflow|'); do
  new_median=$(jq -r ".benchmarks[\"$key\"].median_ms" "$LATEST")
  old_median=$(jq -r ".benchmarks[\"$key\"].median_ms // empty" "$BASELINE" 2>/dev/null || echo "")

  if [[ -z "$old_median" || "$old_median" == "null" ]]; then
    echo "  NEW  $key: ${new_median}ms (no baseline)"
    continue
  fi

  pct=$(awk "BEGIN {printf \"%.1f\", (($new_median - $old_median) / $old_median) * 100}")
  sign=""
  if (( $(awk "BEGIN {print ($new_median > $old_median) ? 1 : 0}") )); then
    sign="+"
  fi

  if (( $(awk "BEGIN {print (($new_median - $old_median) / $old_median > $RELATIVE_THRESHOLD) ? 1 : 0}") )); then
    echo "  FAIL $key: ${old_median}ms -> ${new_median}ms (${sign}${pct}%) -- exceeds ${RELATIVE_THRESHOLD}x threshold"
    FAILED=true
  else
    echo "  OK   $key: ${old_median}ms -> ${new_median}ms (${sign}${pct}%)"
  fi
done

# Absolute checks
echo ""
for key in "${!ABS_THRESHOLDS[@]}"; do
  threshold="${ABS_THRESHOLDS[$key]}"
  new_median=$(jq -r ".benchmarks[\"$key\"].median_ms // empty" "$LATEST" 2>/dev/null || echo "")

  if [[ -z "$new_median" || "$new_median" == "null" ]]; then
    continue
  fi

  if (( $(awk "BEGIN {print ($new_median > $threshold) ? 1 : 0}") )); then
    echo "  FAIL $key: ${new_median}ms exceeds absolute limit ${threshold}ms"
    FAILED=true
  else
    echo "  OK   $key: ${new_median}ms (limit: ${threshold}ms)"
  fi
done

# Binary size check
echo ""
new_size=$(jq -r '.ferrflow_binary_size_mb' "$LATEST")
old_size=$(jq -r '.ferrflow_binary_size_mb // empty' "$BASELINE" 2>/dev/null || echo "")

if [[ -n "$old_size" && "$old_size" != "null" && "$old_size" != "N/A" && "$new_size" != "N/A" ]]; then
  pct=$(awk "BEGIN {printf \"%.1f\", (($new_size - $old_size) / $old_size) * 100}")
  if (( $(awk "BEGIN {print (($new_size - $old_size) / $old_size > $BINARY_SIZE_THRESHOLD) ? 1 : 0}") )); then
    echo "  FAIL binary size: ${old_size}MB -> ${new_size}MB (+${pct}%) -- exceeds 20% threshold"
    FAILED=true
  else
    echo "  OK   binary size: ${old_size}MB -> ${new_size}MB (${pct}%)"
  fi
fi

echo ""
if $FAILED; then
  echo "REGRESSION DETECTED -- see failures above"
  exit 1
else
  echo "All benchmarks within thresholds"
  exit 0
fi
