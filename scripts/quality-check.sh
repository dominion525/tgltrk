#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

# --- Colors (disabled if not a terminal) ---
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    RED='' GREEN='' YELLOW='' BOLD='' RESET=''
fi

pass()  { printf "${GREEN}PASS${RESET}"; }
fail()  { printf "${RED}FAIL${RESET}"; }

# --- Prerequisite checks ---
has_cmd() { command -v "$1" &>/dev/null; }

if ! has_cmd cargo || ! has_cmd rustc; then
    echo "Error: cargo and rustc are required" >&2
    exit 1
fi

if ! has_cmd cargo-audit; then
    echo "Error: cargo-audit is required. Install with: cargo install cargo-audit" >&2
    exit 1
fi

TOTAL=5
STEP=0
ERRORS=0

header() {
    STEP=$((STEP + 1))
    printf "${BOLD}[%d/%d] %-16s${RESET}" "$STEP" "$TOTAL" "$1"
}

echo ""
echo "${BOLD}=== tgltrk quality check ===${RESET}"
echo ""

# --- 1. Test ---
header "Test"
test_output=$(cargo test 2>&1) || {
    printf " $(fail)\n"
    echo "$test_output" | tail -20
    echo ""
    echo "Tests failed. Aborting."
    exit 1
}
test_passed=$(echo "$test_output" | grep -oE '[0-9]+ passed' | tail -1 | grep -oE '[0-9]+')
printf " $(pass) (%s passed)\n" "${test_passed:-?}"

# --- 2. Fmt ---
header "Fmt"
fmt_output=$(cargo fmt --check 2>&1) && {
    printf " $(pass)\n"
} || {
    printf " $(fail)\n"
    echo "$fmt_output" | tail -20
    ERRORS=$((ERRORS + 1))
}

# --- 3. Clippy ---
header "Clippy"
clippy_output=$(cargo clippy --all-targets -- -D warnings 2>&1) || {
    printf " $(fail)\n"
    echo "$clippy_output" | tail -20
    echo ""
    echo "Clippy failed. Aborting."
    exit 1
}
printf " $(pass)\n"

# --- 4. Audit ---
header "Audit"
audit_output=$(cargo audit 2>&1) && {
    printf " $(pass)\n"
} || {
    printf " $(fail)\n"
    echo "$audit_output" | tail -20
    ERRORS=$((ERRORS + 1))
}

# --- 5. Doc ---
header "Doc"
doc_output=$(cargo doc 2>&1) && {
    printf " $(pass)\n"
} || {
    printf " $(fail)\n"
    echo "$doc_output" | tail -20
    ERRORS=$((ERRORS + 1))
}

echo ""
if [ "$ERRORS" -gt 0 ]; then
    echo "${RED}${BOLD}Quality check completed with $ERRORS issue(s).${RESET}"
    exit 1
else
    echo "${GREEN}${BOLD}Quality check passed.${RESET}"
    exit 0
fi
