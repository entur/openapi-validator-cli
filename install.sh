#!/usr/bin/env bash
set -euo pipefail

# Optional environment overrides:
#   OAV_REPO=org/repo
#   OAV_VERSION=1.2.3
#   OAV_INSTALL_DIR=/custom/bin
#   OAV_GITHUB_TOKEN=token
#   OAV_GITHUB_HOST=github.company.com
#   OAV_GITHUB_API=https://github.company.com/api/v3

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required"
  exit 1
fi

repo="${OAV_REPO:-entur/openapi-validator-cli}"
host="${OAV_GITHUB_HOST:-github.com}"
api="${OAV_GITHUB_API:-https://api.github.com}"
if [[ "$host" != "github.com" && -z "${OAV_GITHUB_API:-}" ]]; then
  api="https://${host}/api/v3"
fi
if [[ "$repo" != */* ]]; then
  echo "OAV_REPO must be in org/repo format."
  exit 1
fi

token="${OAV_GITHUB_TOKEN:-${GITHUB_TOKEN:-}}"
auth_header=()
if [[ -n "$token" ]]; then
  auth_header=(-H "Authorization: token ${token}")
fi

if [[ -n "${OAV_VERSION:-}" ]]; then
  version="${OAV_VERSION}"
else
  json="$(curl -fsSL "${auth_header[@]}" "${api}/repos/${repo}/releases/latest")"
  tag="$(printf '%s' "$json" | awk -F\" '/"tag_name":/ {print $4; exit}')"
  if [[ -z "$tag" ]]; then
    echo "Unable to determine latest release tag."
    exit 1
  fi
  version="${tag#v}"
fi

os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Darwin) platform="apple-darwin" ;;
  Linux) platform="unknown-linux-gnu" ;;
  *) echo "Unsupported OS: $os" ; exit 1 ;;
esac

case "$arch" in
  x86_64) arch="x86_64" ;;
  arm64|aarch64) arch="aarch64" ;;
  *) echo "Unsupported architecture: $arch" ; exit 1 ;;
esac

target="${arch}-${platform}"
case "$target" in
  x86_64-apple-darwin|aarch64-apple-darwin|x86_64-unknown-linux-gnu) ;;
  *) echo "No prebuilt binary for ${target}" ; exit 1 ;;
esac

base_url="https://${host}/${repo}/releases/download/v${version}"
asset="oav-${version}-${target}.tar.gz"
sha_asset="${asset}.sha256"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

curl -fsSL "${auth_header[@]}" "${base_url}/${asset}" -o "${tmpdir}/${asset}"
curl -fsSL "${auth_header[@]}" "${base_url}/${sha_asset}" -o "${tmpdir}/${sha_asset}"

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$tmpdir" && sha256sum -c "$sha_asset")
elif command -v shasum >/dev/null 2>&1; then
  (cd "$tmpdir" && shasum -a 256 -c "$sha_asset")
else
  echo "No sha256 checker found (sha256sum or shasum)."
  exit 1
fi

tar -xzf "${tmpdir}/${asset}" -C "$tmpdir"

install_dir="${OAV_INSTALL_DIR:-}"
if [[ -z "$install_dir" ]]; then
  if [[ -w "/usr/local/bin" ]]; then
    install_dir="/usr/local/bin"
  else
    install_dir="${HOME}/.local/bin"
  fi
fi

mkdir -p "$install_dir"
install -m 0755 "${tmpdir}/oav" "${install_dir}/oav"

echo "Installed oav to ${install_dir}/oav"

if [[ ":$PATH:" != *":${install_dir}:"* ]]; then
  echo "Add ${install_dir} to your PATH."
fi
