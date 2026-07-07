#!/usr/bin/env bash
set -euo pipefail

SPARQL_PROXY_PUBLIC_PATH="${SPARQL_PROXY_PUBLIC_PATH:-/}"
stamp_dir="/tmp/togopackage-build-stamps"
stamp_file="${stamp_dir}/sparql-proxy-root-path"
default_root_path="/"
current_root_path="${default_root_path}"

mkdir -p "${stamp_dir}"
if [ -f "${stamp_file}" ]; then
  current_root_path="$(cat "${stamp_file}")"
fi

if [ "${current_root_path}" != "${SPARQL_PROXY_PUBLIC_PATH}" ]; then
  ROOT_PATH="${SPARQL_PROXY_PUBLIC_PATH}" npm run build
  printf '%s' "${SPARQL_PROXY_PUBLIC_PATH}" > "${stamp_file}"
fi
