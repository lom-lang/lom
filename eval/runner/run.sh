#!/usr/bin/env bash
# Lom Eval Runner (Bash)
#
# Usage:
#   ./run.sh --verify                        Verify reference solutions (smoke test eval set)
#   ./run.sh --candidates-dir eval/candidates  Evaluate LLM-generated candidates
#   ./run.sh --verify --verbose              Show per-task detail
#   ./run.sh --help
#
# Requirements:
#   - lom on PATH (run `cargo build` first)
#   - jq (https://stedolan.github.io/jq/) for JSON parsing
#   - bash 4+

set -euo pipefail

VERIFY=0
VERBOSE=0
CANDIDATES_DIR=""
LOM_BIN="${LOM_BIN:-lom}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EVAL_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TASKS_DIR="$EVAL_DIR/tasks"

print_help() {
    cat <<EOF
Lom Eval Runner (Bash)

Usage:
  ./run.sh --verify                          Verify reference solutions
  ./run.sh --candidates-dir <dir>            Evaluate LLM candidates in <dir>
  ./run.sh --verify --verbose                Show per-task detail
  ./run.sh --help                            This help

Requirements:
  - Build lom first:  cargo build
  - lom on PATH (or set LOM_BIN=/path/to/lom)
  - jq for JSON parsing
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --verify) VERIFY=1; shift ;;
        --verbose|-v) VERBOSE=1; shift ;;
        --candidates-dir) CANDIDATES_DIR="$2"; shift 2 ;;
        --lom-bin) LOM_BIN="$2"; shift 2 ;;
        --help|-h) print_help; exit 0 ;;
        *) echo "Unknown option: $1"; print_help; exit 1 ;;
    esac
done

if [[ $VERIFY -eq 0 && -z "$CANDIDATES_DIR" ]]; then
    print_help
    exit 0
fi

command -v jq >/dev/null 2>&1 || { echo "ERROR: jq is required. Install from https://stedolan.github.io/jq/"; exit 1; }
command -v "$LOM_BIN" >/dev/null 2>&1 || { echo "ERROR: lom binary not found. Build it first: cargo build"; exit 1; }

[[ -d "$TASKS_DIR" ]] || { echo "ERROR: tasks dir not found: $TASKS_DIR"; exit 1; }

TOTAL=0
PASSED=0
FAILED=0
declare -A CAT_TOTAL CAT_PASSED CAT_FAILED

MODE="verify"
[[ -n "$CANDIDATES_DIR" ]] && MODE="candidates"
echo "Lom Eval Runner — mode: $MODE"
echo ""

shopt -s nullglob
for file in "$TASKS_DIR"/*.json; do
    filename="$(basename "$file")"
    category="${filename#[0-9]*_}"
    category="${category%.json}"
    [[ -z "${CAT_TOTAL[$category]+x}" ]] && CAT_TOTAL[$category]=0 && CAT_PASSED[$category]=0 && CAT_FAILED[$category]=0

    task_count=$(jq 'length' "$file")
    for i in $(seq 0 $((task_count - 1))); do
        TOTAL=$((TOTAL + 1))
        CAT_TOTAL[$category]=$((CAT_TOTAL[$category] + 1))

        task_id=$(jq -r ".[$i].id" "$file")
        expected=$(jq -r ".[$i].expected" "$file")

        if [[ $VERIFY -eq 1 ]]; then
            src=$(jq -r ".[$i].solution" "$file")
        else
            candidate_path="$CANDIDATES_DIR/$task_id.lom"
            if [[ ! -f "$candidate_path" ]]; then
                FAILED=$((FAILED + 1))
                CAT_FAILED[$category]=$((CAT_FAILED[$category] + 1))
                [[ $VERBOSE -eq 1 ]] && echo "  [$task_id] MISSING candidate: $candidate_path"
                continue
            fi
            src=$(cat "$candidate_path")
        fi

        tmpfile="$(mktemp).lom"
        printf '%s' "$src" > "$tmpfile"
        actual=$("$LOM_BIN" "$tmpfile" 2>&1 || true)
        rm -f "$tmpfile"

        # Normalize line endings (jq already strips trailing \n; we re-add for comparison)
        actual_normalized=$(printf '%s\n' "$actual")
        expected_normalized=$(printf '%s\n' "$expected")

        if [[ "$actual_normalized" == "$expected_normalized" ]]; then
            PASSED=$((PASSED + 1))
            CAT_PASSED[$category]=$((CAT_PASSED[$category] + 1))
            [[ $VERBOSE -eq 1 ]] && echo "  [$task_id] PASS ($category)"
        else
            FAILED=$((FAILED + 1))
            CAT_FAILED[$category]=$((CAT_FAILED[$category] + 1))
            if [[ $VERBOSE -eq 1 ]]; then
                echo "  [$task_id] FAIL ($category)"
                echo "    expected: $(printf '%s' "$expected_normalized" | tr '\n' '|')"
                echo "    actual:   $(printf '%s' "$actual_normalized" | tr '\n' '|')"
            fi
        fi
    done
done

echo ""
echo "===== Summary ====="
echo "Total:  $TOTAL"
echo "Passed: $PASSED"
echo "Failed: $FAILED"
if [[ $TOTAL -gt 0 ]]; then
    rate=$(awk "BEGIN { printf \"%.1f\", $PASSED / $TOTAL * 100 }")
    echo "Rate:   ${rate}%"
fi
echo ""
echo "===== By category ====="
for cat in "${!CAT_TOTAL[@]}"; do
    t=${CAT_TOTAL[$cat]}
    p=${CAT_PASSED[$cat]}
    r=$(awk "BEGIN { printf \"%.1f\", $p / $t * 100 }")
    printf "  %-20s %d/%d (%s%%)\n" "$cat" "$p" "$t" "$r"
done | sort

[[ $FAILED -gt 0 ]] && exit 1 || exit 0
