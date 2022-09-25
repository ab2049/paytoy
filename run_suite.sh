#!/bin/bash
# Run a suite of test cases
# Usage: run_suite.sh suite_name

SUITE="$1"

if [[ ! -d "test_suites/$SUITE/input" ]]; then
    echo "Suite test_suites/$SUITE not found"
    exit 1
fi

for TEST in "test_suites/$SUITE"/input/*; do
    ./run_t.sh "$TEST" || exit 1
    echo "$SUITE: Test case $TEST passed"
done
