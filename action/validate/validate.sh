#!/usr/bin/env bash
set -uo pipefail

# Build init args
init_args=()
if [[ -n "${INPUT_SPEC}" ]]; then
  init_args+=(--spec "${INPUT_SPEC}")
fi

# Run init (non-fatal — validate will also scaffold if needed)
oav init "${init_args[@]}" 2>&1 || true

# Build validate args
args=(--color never)

if [[ -n "${INPUT_SPEC}" ]]; then
  args+=(--spec "${INPUT_SPEC}")
fi
if [[ -n "${INPUT_MODE}" ]]; then
  args+=(--mode "${INPUT_MODE}")
fi
if [[ -n "${INPUT_SERVER_GENERATORS}" ]]; then
  args+=(--server-generators "${INPUT_SERVER_GENERATORS}")
fi
if [[ -n "${INPUT_CLIENT_GENERATORS}" ]]; then
  args+=(--client-generators "${INPUT_CLIENT_GENERATORS}")
fi
if [[ "${INPUT_SKIP_LINT}" == "true" ]]; then
  args+=(--skip-lint)
fi
if [[ "${INPUT_SKIP_GENERATE}" == "true" ]]; then
  args+=(--skip-generate)
fi
if [[ "${INPUT_SKIP_COMPILE}" == "true" ]]; then
  args+=(--skip-compile)
fi
if [[ -n "${INPUT_LINTER}" ]]; then
  args+=(--linter "${INPUT_LINTER}")
fi
if [[ -n "${INPUT_RULESET}" ]]; then
  args+=(--ruleset "${INPUT_RULESET}")
fi
if [[ -n "${INPUT_DOCKER_TIMEOUT}" ]]; then
  args+=(--docker-timeout "${INPUT_DOCKER_TIMEOUT}")
fi
if [[ -n "${INPUT_SEARCH_DEPTH}" ]]; then
  args+=(--search-depth "${INPUT_SEARCH_DEPTH}")
fi

echo "Running: oav validate ${args[*]}"

set +e
oav validate "${args[@]}" 2>&1
exit_code=$?
set -e

# Map exit code to result
case $exit_code in
  0) result="pass" ;;
  1) result="fail" ;;
  *) result="error" ;;
esac

# Parse status.tsv for counts
total=0
passed=0
failed=0
if [[ -f .oav/status.tsv ]]; then
  while IFS=$'\t' read -r _stage _scope _target status _log; do
    total=$((total + 1))
    case "$status" in
      ok)   passed=$((passed + 1)) ;;
      fail) failed=$((failed + 1)) ;;
    esac
  done < .oav/status.tsv
fi

{
  echo "result=${result}"
  echo "exit_code=${exit_code}"
  echo "total=${total}"
  echo "passed=${passed}"
  echo "failed=${failed}"
} >> "$GITHUB_OUTPUT"
