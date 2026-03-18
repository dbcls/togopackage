#!/usr/bin/env bash
set -euo pipefail

uid="$(id -u)"
UV_CACHE_DIR="${UV_CACHE_DIR:-/tmp/uv-cache-${uid}}"
mkdir -p "${UV_CACHE_DIR}"

# Optionally sync user-provided MIE YAML files into TOGOMCP_DIR before startup.
TOGOMCP_DIR="${TOGOMCP_DIR:-/togomcp}"
TOGOMCP_DATA_DIR="${TOGOMCP_DATA_DIR:-/data/togomcp}"
TOGOMCP_DEFAULTS_DIR="${TOGOMCP_DEFAULTS_DIR:-/togo/defaults/togomcp}"
TOGOMCP_DEFAULT_MIE_DIR="${TOGOMCP_DEFAULT_MIE_DIR:-${TOGOMCP_DEFAULTS_DIR}/mie}"
TOGOMCP_DEFAULT_ENDPOINTS_FILE="${TOGOMCP_DEFAULT_ENDPOINTS_FILE:-${TOGOMCP_DEFAULTS_DIR}/endpoints.csv}"
TOGOMCP_MIE_SYNC_SOURCE_DIR="${TOGOMCP_MIE_SYNC_SOURCE_DIR:-/data/togomcp/mie}"
TOGOMCP_MIE_SYNC_DEST_DIR="${TOGOMCP_DIR}/mie"
TOGOMCP_ENDPOINTS_DEST_FILE="${TOGOMCP_DIR}/resources/endpoints.csv"
TOGOMCP_ENDPOINTS_SOURCE_FILE="${TOGOMCP_ENDPOINTS_SOURCE_FILE:-${TOGOMCP_DATA_DIR}/endpoints.csv}"

append_csv_rows() {
  local source_file="$1"
  local dest_file="$2"

  [ -f "${source_file}" ] || return 0

  awk '
    BEGIN { OFS="," }
    { sub(/\r$/, "", $0) }
    /^[[:space:]]*$/ { next }
    /^[[:space:]]*#/ { next }
    NR == 1 && $0 == "database,endpoint_url,endpoint_name,keyword_search_api" { next }
    { print $0 }
  ' "${source_file}" >> "${dest_file}"
}

mkdir -p "${TOGOMCP_MIE_SYNC_DEST_DIR}"
find "${TOGOMCP_MIE_SYNC_DEST_DIR}" -maxdepth 1 -type f -name '*.yaml' -delete
if [ -d "${TOGOMCP_DEFAULT_MIE_DIR}" ]; then
  find "${TOGOMCP_DEFAULT_MIE_DIR}" -maxdepth 1 -type f -name '*.yaml' -exec cp -f {} "${TOGOMCP_MIE_SYNC_DEST_DIR}/" \;
fi
if [ -d "${TOGOMCP_MIE_SYNC_SOURCE_DIR}" ]; then
  find "${TOGOMCP_MIE_SYNC_SOURCE_DIR}" -maxdepth 1 -type f -name '*.yaml' -exec cp -f {} "${TOGOMCP_MIE_SYNC_DEST_DIR}/" \;
fi

tmp_file="$(mktemp)"
deduped_file="$(mktemp)"
printf '%s\n' 'database,endpoint_url,endpoint_name,keyword_search_api' > "${tmp_file}"
append_csv_rows "${TOGOMCP_DEFAULT_ENDPOINTS_FILE}" "${tmp_file}"
append_csv_rows "${TOGOMCP_ENDPOINTS_SOURCE_FILE}" "${tmp_file}"
awk 'NR == 1 || !seen[$0]++' "${tmp_file}" > "${deduped_file}"
cat "${deduped_file}" > "${TOGOMCP_ENDPOINTS_DEST_FILE}"
rm -f "${tmp_file}" "${deduped_file}"
