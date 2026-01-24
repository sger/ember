#!/usr/bin/env bash
set -euo pipefail

EMB=target/debug/ember

cargo build

failures=0

echo "Running SUCCESS examples"
echo "------------------------"

# Run all .em files except known-failing ones
for file in examples/*.em; do
  case "$file" in
    examples/test_undef.em|examples/test_type.em|examples/test_idx.em|examples/test_div.em)
      continue
      ;;
  esac

  echo "▶ $file"
  if ! "$EMB" "$file"; then
    echo "✗ FAILED (expected success): $file"
    failures=$((failures + 1))
  else
    echo "✓ OK"
  fi
  echo
done

if [ "$failures" -ne 0 ]; then
  echo "$failures success-example(s) failed"
  exit 1
fi

echo "All success examples passed 🎉"
