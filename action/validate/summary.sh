#!/usr/bin/env bash
set -euo pipefail

status_file=".oav/status.tsv"

if [[ ! -f "$status_file" ]]; then
  echo "No status.tsv found — skipping step summary."
  exit 0
fi

total=0
passed=0
failed=0

rows=""
while IFS=$'\t' read -r stage scope target status _; do
  total=$((total + 1))
  case "$status" in
    ok)   passed=$((passed + 1)); icon="+" ;;
    fail) failed=$((failed + 1)); icon="x" ;;
    *)    icon="-" ;;
  esac
  rows="${rows}| ${stage} | ${scope} | ${target} | ${icon} ${status} |"$'\n'
done < "$status_file"

if [[ $failed -gt 0 ]]; then
  header="OpenAPI Validation: ${passed}/${total} passed, ${failed} failed"
else
  header="OpenAPI Validation: ${total}/${total} passed"
fi

{
  echo "## ${header}"
  echo ""
  echo "| Stage | Scope | Target | Status |"
  echo "|-------|-------|--------|--------|"
  echo -n "${rows}"
} >> "$GITHUB_STEP_SUMMARY"
