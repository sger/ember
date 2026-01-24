#!/usr/bin/env bash
set -euo pipefail

EMB=target/debug/ember

cargo build

echo "Running FAILURE examples (expect non-zero exit)"
echo "----------------------------------------------"

failures=0

FAIL_FILES=(
  examples/test_undef.em
  examples/test_type.em
  examples/test_idx.em
  examples/test_div.em
)

for file in "${FAIL_FILES[@]}"; do
  echo "▶ $file"
  if "$EMB" "$file"; then
    echo "✗ UNEXPECTED PASS (expected failure): $file"
    failures=$((failures + 1))
  else
    echo "✓ Expected failure"
  fi
  echo
done

if [ "$failures" -ne 0 ]; then
  echo "$failures failure-example(s) unexpectedly passed"
  exit 1
fi

echo "All failure examples failed as expected 🎉"
