#!/usr/bin/env bash
set -euo pipefail

FERRFLOW="${FERRFLOW_BIN:-ferrflow}"
GEN_DIR="${1:-${GEN_DIR:-fixtures/generated}}"
PASSED=0
FAILED=0
ERRORS=()

# Resolve to absolute path if relative
if [[ "$FERRFLOW" == ./* || "$FERRFLOW" == ../* ]]; then
    FERRFLOW="$(cd "$(dirname "$FERRFLOW")" && pwd)/$(basename "$FERRFLOW")"
fi

if ! command -v "$FERRFLOW" &>/dev/null && [ ! -x "$FERRFLOW" ]; then
    echo "Error: ferrflow not found. Set FERRFLOW_BIN or add ferrflow to PATH."
    exit 1
fi

if [ ! -d "$GEN_DIR" ]; then
    echo "Error: $GEN_DIR not found. Run the generator first."
    exit 1
fi

strip_ansi() {
    sed 's/\x1b\[[0-9;]*m//g'
}

# Parse a TOML string array value. Reads the file, extracts the array for the
# given key, and prints one element per line (without quotes).
parse_toml_array() {
    local file="$1" key="$2"
    python3 -c "
import sys, re
content = open('$file').read()
m = re.search(r'^${key}\s*=\s*\[(.*?)\]', content, re.MULTILINE | re.DOTALL)
if m:
    items = re.findall(r'\"([^\"]*)\"', m.group(1))
    for item in items:
        print(item)
" 2>/dev/null || true
}

for fixture_dir in "$GEN_DIR"/*/; do
    name="$(basename "$fixture_dir")"
    expect_file="$fixture_dir/.expect.toml"

    if [ ! -f "$expect_file" ]; then
        echo "  SKIP $name (no .expect.toml)"
        continue
    fi

    # Run ferrflow check from the fixture directory
    output=$(cd "$fixture_dir" && "$FERRFLOW" check 2>&1 || true)
    output=$(echo "$output" | strip_ansi)

    failed=false

    # Check check_contains
    while IFS= read -r expected; do
        [ -z "$expected" ] && continue
        if ! echo "$output" | grep -qF "$expected"; then
            echo "  FAIL $name: expected output to contain '$expected'"
            failed=true
        fi
    done < <(parse_toml_array "$expect_file" "check_contains")

    # Check check_not_contains
    while IFS= read -r unexpected; do
        [ -z "$unexpected" ] && continue
        if echo "$output" | grep -qF "$unexpected"; then
            echo "  FAIL $name: expected output NOT to contain '$unexpected'"
            failed=true
        fi
    done < <(parse_toml_array "$expect_file" "check_not_contains")

    # Check output_order
    mapfile -t order_items < <(parse_toml_array "$expect_file" "output_order")

    if [ ${#order_items[@]} -gt 1 ]; then
        last_pos=-1
        order_ok=true
        for item in "${order_items[@]}"; do
            pos=$(echo "$output" | grep -b -o "$item" | head -1 | cut -d: -f1 || echo "-1")
            if [ "$pos" = "-1" ]; then
                echo "  FAIL $name: '$item' not found in output for order check"
                failed=true
                order_ok=false
                break
            fi
            if [ "$pos" -le "$last_pos" ]; then
                echo "  FAIL $name: '$item' appears before expected position"
                failed=true
                order_ok=false
                break
            fi
            last_pos=$pos
        done

        # Check blank line separation between ordered items
        if [ "$order_ok" = true ]; then
            for i in $(seq 0 $((${#order_items[@]} - 2))); do
                current="${order_items[$i]}"
                next="${order_items[$((i + 1))]}"
                between=$(echo "$output" | sed -n "/$current/,/$next/p")
                if ! echo "$between" | grep -q '^$'; then
                    echo "  FAIL $name: no blank line between '$current' and '$next'"
                    failed=true
                fi
            done
        fi
    fi

    if [ "$failed" = true ]; then
        FAILED=$((FAILED + 1))
        ERRORS+=("$name")
        echo "        output was:"
        echo "$output" | sed 's/^/        | /'
    else
        echo "  ok   $name"
        PASSED=$((PASSED + 1))
    fi
done

echo ""
echo "Results: $PASSED passed, $FAILED failed"

if [ $FAILED -gt 0 ]; then
    echo "Failed: ${ERRORS[*]}"
    exit 1
fi
