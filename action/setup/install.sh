#!/usr/bin/env bash
set -euo pipefail

version="${INPUT_VERSION}"
target="${INPUT_TARGET}"
token="${INPUT_TOKEN}"
install_dir="${RUNNER_TOOL_CACHE:-$HOME/.oav/bin}/oav/${version}"

mkdir -p "$install_dir"

if [[ "${CACHE_HIT}" != "true" ]]; then
  base_url="https://github.com/entur/openapi-validator-cli/releases/download/v${version}"
  asset="oav-${version}-${target}.tar.gz"
  sha_asset="${asset}.sha256"

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  echo "Downloading oav v${version} for ${target}..."
  curl -fsSL -H "Authorization: token ${token}" \
    "${base_url}/${asset}" -o "${tmpdir}/${asset}"
  curl -fsSL -H "Authorization: token ${token}" \
    "${base_url}/${sha_asset}" -o "${tmpdir}/${sha_asset}"

  echo "Verifying checksum..."
  (cd "$tmpdir" && sha256sum -c "$sha_asset")

  tar -xzf "${tmpdir}/${asset}" -C "$tmpdir"
  install -m 0755 "${tmpdir}/oav" "${install_dir}/oav"

  echo "Installed oav v${version} to ${install_dir}"
else
  echo "Using cached oav v${version}"
fi

echo "version=${version}" >> "$GITHUB_OUTPUT"
echo "${install_dir}" >> "$GITHUB_PATH"
