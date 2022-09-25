#!/bin/bash
# Run one test case
# Usage: run_t.sh <test case input>

TEST=$(basename "$1")
DIRNAME=$(dirname "$1")
DIRNAME=$(dirname "$DIRNAME")
SUITE=$(basename "$DIRNAME")

INPUT="test_suites/$SUITE/input/$TEST"
if [[ ! -f "$INPUT" ]]; then
    echo "Test case $TEST not found in test_suites/$SUITE/input"
    exit 1
fi

EXPECTED_OUTPUT="test_suites/$SUITE/output/$TEST"
EXPECTED_ERROR="test_suites/$SUITE/error/$TEST"

if [[ ! -r "$EXPECTED_OUTPUT" && ! -s "$EXPECTED_ERROR" ]]; then
    echo "Test case $TEST has neither expected output $EXPECTED_OUTPUT or expected error $EXPECTED_ERROR"
    exit 1
fi

if [[ ! -s "$EXPECTED_OUTPUT" ]]; then
   EXPECTED_OUTPUT=
fi

if [[ ! -s "$EXPECTED_ERROR" ]]; then
   EXPECTED_ERROR=
fi

MYTMPDIR=$(mktemp -d "${TMPDIR:-/tmp}/run_t.XXXXXXXXX") || exit 1
trap 'rm -rf -- "$MYTMPDIR"' EXIT

OUTPUT="$MYTMPDIR/$TEST".out
ERROR="$MYTMPDIR/$TEST".err

cargo run -q --release -- "$INPUT" > "$OUTPUT" 2> "$ERROR"
STATUS=$?

if [[ -s "$OUTPUT" && -z "$EXPECTED_OUTPUT" ]]; then
    echo "$TEST unexpectedly has output in $OUTPUT" 2>&1
    exit 1
fi

if [[ -s "$ERROR" && -z "$EXPECTED_ERROR" ]]; then
    echo "$TEST unexpectedly has errors in $ERROR" 2>&1
    exit 1
fi

if [[ $STATUS -ne 0 && -z "$EXPECTED_ERROR" ]]; then
    echo "$TEST unexpectedly failed with status $STATUS" 2>&1
    exit 1
fi

if [[ $STATUS -eq 0 && -n "$EXPECTED_ERROR" ]]; then
    echo "$TEST unexpectedly succeeded with $STATUS" 2>&1
    exit 1
fi

if [[ -n "$EXPECTED_OUTPUT" ]]; then
    diff -u "$EXPECTED_OUTPUT" "$OUTPUT" > "$MYTMPDIR/$TEST".diff
    DIFF_STATUS=$?
    if [[ $DIFF_STATUS -ne 0 ]]; then
        echo "$TEST: output differs from expected output" 2>&1
        cat "$MYTMPDIR/$TEST".diff 2>&1
        exit 1
    fi
fi

if [[ -n "$EXPECTED_ERROR" ]]; then
    diff -u "$EXPECTED_ERROR" "$ERROR" > "$MYTMPDIR/$TEST".diff
    DIFF_STATUS=$?
    if [[ $DIFF_STATUS -ne 0 ]]; then
        echo "$TEST: error differs from expected error" 2>&1
        cat "$MYTMPDIR/$TEST".diff 2>&1
        exit 1
    fi
fi
