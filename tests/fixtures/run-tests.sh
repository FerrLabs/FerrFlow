#!/usr/bin/env bash
set -euo pipefail

FERRFLOW="${FERRFLOW_BIN:-ferrflow}"
GEN_DIR="${1:-${GEN_DIR:-fixtures/generated}}"
DIFF_DIR="${DIFF_DIR:-}"
MAX_PARALLEL="${MAX_PARALLEL:-$(nproc 2>/dev/null || echo 4)}"
PASSED=0
FAILED=0
SKIPPED=0
ERRORS=()
TOTAL=0

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

# Parse a TOML integer value
parse_toml_int() {
    local file="$1" key="$2"
    python3 -c "
import sys, re
content = open('$file').read()
m = re.search(r'^${key}\s*=\s*(\d+)', content, re.MULTILINE)
if m:
    print(m.group(1))
" 2>/dev/null || true
}

# Parse a TOML boolean value
parse_toml_bool() {
    local file="$1" key="$2"
    python3 -c "
import sys, re
content = open('$file').read()
m = re.search(r'^${key}\s*=\s*(true|false)', content, re.MULTILINE)
if m:
    print(m.group(1))
" 2>/dev/null || true
}

# Run a single fixture test and write result to a temp file
run_fixture() {
    local fixture_dir="$1"
    local result_file="$2"
    local name diff_file output json_output failed

    name="$(basename "$fixture_dir")"
    expect_file="$fixture_dir/.expect.toml"

    if [ ! -f "$expect_file" ]; then
        echo "SKIP" > "$result_file"
        return
    fi

    # Run ferrflow check from the fixture directory
    output=$(cd "$fixture_dir" && "$FERRFLOW" check 2>&1 || true)
    output=$(echo "$output" | strip_ansi)

    # Run ferrflow check --json if json expectations exist
    json_output=""
    if grep -q 'json_contains\|json_not_contains\|packages_released' "$expect_file" 2>/dev/null; then
        json_output=$(cd "$fixture_dir" && "$FERRFLOW" check --json 2>&1 || true)
        json_output=$(echo "$json_output" | strip_ansi)
    fi

    failed=false
    local failure_details=""

    # Check check_contains
    while IFS= read -r expected; do
        [ -z "$expected" ] && continue
        if ! echo "$output" | grep -qF "$expected"; then
            failure_details+="  FAIL $name: expected output to contain '$expected'\n"
            failed=true
        fi
    done < <(parse_toml_array "$expect_file" "check_contains")

    # Check check_not_contains
    while IFS= read -r unexpected; do
        [ -z "$unexpected" ] && continue
        if echo "$output" | grep -qF "$unexpected"; then
            failure_details+="  FAIL $name: expected output NOT to contain '$unexpected'\n"
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
                failure_details+="  FAIL $name: '$item' not found in output for order check\n"
                failed=true
                order_ok=false
                break
            fi
            if [ "$pos" -le "$last_pos" ]; then
                failure_details+="  FAIL $name: '$item' appears before expected position\n"
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
                    failure_details+="  FAIL $name: no blank line between '$current' and '$next'\n"
                    failed=true
                fi
            done
        fi
    fi

    # Check packages_released count from JSON output
    local expected_count
    expected_count=$(parse_toml_int "$expect_file" "packages_released")
    if [ -n "$expected_count" ] && [ -n "$json_output" ]; then
        local actual_count
        actual_count=$(echo "$json_output" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    pkgs = data if isinstance(data, list) else data.get('packages', data.get('releases', []))
    print(len([p for p in pkgs if p.get('bump', p.get('bump_type', 'none')) != 'none']))
except:
    print(-1)
" 2>/dev/null || echo "-1")
        if [ "$actual_count" != "$expected_count" ]; then
            failure_details+="  FAIL $name: expected $expected_count packages released, got $actual_count\n"
            failed=true
        fi
    fi

    # Check json_contains
    while IFS= read -r expected; do
        [ -z "$expected" ] && continue
        if [ -n "$json_output" ] && ! echo "$json_output" | grep -qF "$expected"; then
            failure_details+="  FAIL $name: expected JSON output to contain '$expected'\n"
            failed=true
        fi
    done < <(parse_toml_array "$expect_file" "json_contains")

    # Check json_not_contains
    while IFS= read -r unexpected; do
        [ -z "$unexpected" ] && continue
        if [ -n "$json_output" ] && echo "$json_output" | grep -qF "$unexpected"; then
            failure_details+="  FAIL $name: expected JSON output NOT to contain '$unexpected'\n"
            failed=true
        fi
    done < <(parse_toml_array "$expect_file" "json_not_contains")

    # Check exit_code expectation
    local expected_exit
    expected_exit=$(parse_toml_int "$expect_file" "exit_code")
    if [ -n "$expected_exit" ]; then
        local actual_exit
        actual_exit=$(cd "$fixture_dir" && "$FERRFLOW" check 2>/dev/null; echo $?)
        if [ "$actual_exit" != "$expected_exit" ]; then
            failure_details+="  FAIL $name: expected exit code $expected_exit, got $actual_exit\n"
            failed=true
        fi
    fi

    if [ "$failed" = true ]; then
        # Write diff to file if DIFF_DIR is set
        if [ -n "$DIFF_DIR" ]; then
            diff_file="$DIFF_DIR/$name.diff"
            {
                echo "=== Fixture: $name ==="
                echo "--- Expected ---"
                echo -e "$failure_details"
                echo "--- Actual output ---"
                echo "$output"
                if [ -n "$json_output" ]; then
                    echo "--- JSON output ---"
                    echo "$json_output"
                fi
            } > "$diff_file"
        fi
        {
            echo "FAIL"
            echo -e "$failure_details"
            echo "        output was:"
            echo "$output" | sed 's/^/        | /'
        } > "$result_file"
    else
        echo "PASS" > "$result_file"
    fi
}

# Collect fixture directories
fixture_dirs=()
for fixture_dir in "$GEN_DIR"/*/; do
    [ -d "$fixture_dir" ] && fixture_dirs+=("$fixture_dir")
done

TOTAL=${#fixture_dirs[@]}

if [ "$TOTAL" -eq 0 ]; then
    echo "No fixtures found in $GEN_DIR"
    exit 1
fi

echo "Running $TOTAL fixture tests (parallelism: $MAX_PARALLEL)..."
echo ""

# Create temp directory for results
RESULTS_DIR=$(mktemp -d)
trap 'rm -rf "$RESULTS_DIR"' EXIT

# Create diff directory if requested
if [ -n "$DIFF_DIR" ]; then
    mkdir -p "$DIFF_DIR"
fi

# Run fixtures in parallel
running=0
idx=0
for fixture_dir in "${fixture_dirs[@]}"; do
    result_file="$RESULTS_DIR/$(basename "$fixture_dir")"
    run_fixture "$fixture_dir" "$result_file" &
    running=$((running + 1))
    idx=$((idx + 1))

    if [ "$running" -ge "$MAX_PARALLEL" ]; then
        wait -n 2>/dev/null || wait
        running=$((running - 1))
    fi
done

# Wait for remaining jobs
wait

# Collect results
for fixture_dir in "${fixture_dirs[@]}"; do
    name="$(basename "$fixture_dir")"
    result_file="$RESULTS_DIR/$name"

    if [ ! -f "$result_file" ]; then
        echo "  SKIP $name (no result)"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    status=$(head -1 "$result_file")
    case "$status" in
        PASS)
            echo "  ok   $name"
            PASSED=$((PASSED + 1))
            ;;
        SKIP)
            echo "  SKIP $name (no .expect.toml)"
            SKIPPED=$((SKIPPED + 1))
            ;;
        FAIL)
            tail -n +2 "$result_file"
            FAILED=$((FAILED + 1))
            ERRORS+=("$name")
            ;;
        *)
            echo "  ERROR $name (unexpected result)"
            FAILED=$((FAILED + 1))
            ERRORS+=("$name")
            ;;
    esac
done

echo ""
echo "Results: $PASSED passed, $FAILED failed, $SKIPPED skipped (total: $TOTAL)"

# Write GitHub Actions summary if available
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    {
        echo "## Fixture Test Results"
        echo ""
        echo "| Metric | Count |"
        echo "|--------|-------|"
        echo "| Passed | $PASSED |"
        echo "| Failed | $FAILED |"
        echo "| Skipped | $SKIPPED |"
        echo "| Total | $TOTAL |"
        echo ""
        if [ $FAILED -gt 0 ]; then
            echo "### Failed fixtures"
            echo ""
            for err in "${ERRORS[@]}"; do
                echo "- \`$err\`"
            done
            echo ""
            echo "Check the uploaded **fixture-diffs** artifact for details."
        fi
    } >> "$GITHUB_STEP_SUMMARY"
fi

if [ $FAILED -gt 0 ]; then
    echo "Failed: ${ERRORS[*]}"
    exit 1
fi
