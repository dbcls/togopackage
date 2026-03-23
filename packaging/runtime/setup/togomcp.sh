#!/usr/bin/env bash
set -euo pipefail

uid="$(id -u)"
UV_CACHE_DIR="${UV_CACHE_DIR:-/tmp/uv-cache-${uid}}"
mkdir -p "${UV_CACHE_DIR}"

# Optionally sync user-provided MIE YAML files into TOGOMCP_DIR before startup.
TOGOMCP_DIR="${TOGOMCP_DIR:-/togomcp}"
TOGOMCP_MIE_SYNC_SOURCE_DIR="${TOGOMCP_MIE_SYNC_SOURCE_DIR:-/data/togomcp/mie}"
TOGOMCP_MIE_SYNC_DEST_DIR="${TOGOMCP_DIR}/mie"
TOGOMCP_ENDPOINTS_DEST_FILE="${TOGOMCP_DIR}/resources/endpoints.csv"
TOGOMCP_SPARQL_ENDPOINT="${TOGOMCP_SPARQL_ENDPOINT:-http://localhost:7002/sparql}"
TOGOMCP_ENDPOINT_NAME="${TOGOMCP_ENDPOINT_NAME:-local}"
TOGOMCP_KEYWORD_SEARCH_API="${TOGOMCP_KEYWORD_SEARCH_API:-sparql}"

generate_endpoints_csv() {
  local mie_dir="$1"
  local dest_file="$2"
  local endpoint_url="$3"
  local endpoint_name="$4"
  local keyword_search_api="$5"

  printf '%s\n' 'database,endpoint_url,endpoint_name,keyword_search_api' > "${dest_file}"

  find "${mie_dir}" -maxdepth 1 -type f -name '*.yaml' -printf '%f\n' \
    | sed 's/\.yaml$//' \
    | sort -u \
    | awk -v endpoint_url="${endpoint_url}" -v endpoint_name="${endpoint_name}" -v keyword_search_api="${keyword_search_api}" '
        BEGIN { OFS = "," }
        NF == 0 { next }
        { print $1, endpoint_url, endpoint_name, keyword_search_api }
      ' >> "${dest_file}"
}

mkdir -p "${TOGOMCP_MIE_SYNC_DEST_DIR}"
find "${TOGOMCP_MIE_SYNC_DEST_DIR}" -maxdepth 1 -type f -name '*.yaml' -delete
if [ -d "${TOGOMCP_MIE_SYNC_SOURCE_DIR}" ] && find "${TOGOMCP_MIE_SYNC_SOURCE_DIR}" -maxdepth 1 -type f -name '*.yaml' | grep -q .; then
  find "${TOGOMCP_MIE_SYNC_SOURCE_DIR}" -maxdepth 1 -type f -name '*.yaml' -exec cp -f {} "${TOGOMCP_MIE_SYNC_DEST_DIR}/" \;
fi

tmp_file="$(mktemp)"
generate_endpoints_csv \
  "${TOGOMCP_MIE_SYNC_DEST_DIR}" \
  "${tmp_file}" \
  "${TOGOMCP_SPARQL_ENDPOINT}" \
  "${TOGOMCP_ENDPOINT_NAME}" \
  "${TOGOMCP_KEYWORD_SEARCH_API}"
cat "${tmp_file}" > "${TOGOMCP_ENDPOINTS_DEST_FILE}"
rm -f "${tmp_file}"
