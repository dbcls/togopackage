#!/usr/bin/env bash
set -euo pipefail

: "${VIRTUOSO_DATA_DIR:=/data/virtuoso}"
: "${QLEVER_DATA_DIR:=/data/sources}"
: "${SOURCE_MANIFEST_PATH:=${QLEVER_DATA_DIR}/source-manifest.json}"
: "${VIRTUOSO_INI_PATH:=${VIRTUOSO_DATA_DIR}/virtuoso.ini}"
: "${VIRTUOSO_LOAD_SQL_PATH:=${VIRTUOSO_DATA_DIR}/load.sql}"
: "${VIRTUOSO_HTTP_PORT:=8890}"
: "${VIRTUOSO_ISQL_PORT:=1111}"
: "${VIRTUOSO_DBA_PASSWORD:=dba}"

VIRTUOSO_DB_DIR="${VIRTUOSO_DATA_DIR}/db"
VIRTUOSO_LOAD_STAMP="${VIRTUOSO_DATA_DIR}/.loaded-input-hash"
export VIRTUOSO_DATA_DIR QLEVER_DATA_DIR SOURCE_MANIFEST_PATH
export VIRTUOSO_INI_PATH VIRTUOSO_LOAD_SQL_PATH VIRTUOSO_HTTP_PORT
export VIRTUOSO_ISQL_PORT VIRTUOSO_DBA_PASSWORD

mkdir -p "${VIRTUOSO_DB_DIR}"

log_virtuoso() {
  local message="$1"
  printf '%s\n' "${message}" >&2
}

write_input_stamp() {
  local stamp_path="$1"
  local input_hash="$2"
  printf '%s\n' "${input_hash}" >"${stamp_path}"
}

skip_input_state() {
  local component="$1"
  log_virtuoso "${component} is up to date for current input. Skipped rebuild."
}

wait_for_virtuoso_http() {
  local url="http://127.0.0.1:${VIRTUOSO_HTTP_PORT}/sparql"
  local attempt
  for attempt in $(seq 1 60); do
    if curl -fsS -o /dev/null "${url}"; then
      return 0
    fi
    if ! kill -0 "${virtuoso_pid}" 2>/dev/null; then
      wait "${virtuoso_pid}"
    fi
    sleep 1
  done
  log_virtuoso "Timed out waiting for Virtuoso HTTP endpoint."
  return 1
}

load_sources_if_needed() {
  local input_hash
  local load_output
  input_hash="$(/usr/local/bin/togopackage-ingest generate-virtuoso-load-sql)"
  if [ -f "${VIRTUOSO_LOAD_STAMP}" ] && [ "$(cat "${VIRTUOSO_LOAD_STAMP}")" = "${input_hash}" ]; then
    skip_input_state "Virtuoso"
    return 0
  fi

  log_virtuoso "Virtuoso data import started."
  if load_output="$(bash -lc "isql-vt \"127.0.0.1:${VIRTUOSO_ISQL_PORT}\" dba \"${VIRTUOSO_DBA_PASSWORD}\" VERBOSE=OFF PROMPT=OFF <\"${VIRTUOSO_LOAD_SQL_PATH}\"" 2>&1)"; then
    printf '%s\n' "${load_output}" >&2
    if printf '%s\n' "${load_output}" | grep -q '^\*\*\* Error'; then
      log_virtuoso "Virtuoso data import failed."
      return 1
    fi
    write_input_stamp "${VIRTUOSO_LOAD_STAMP}" "${input_hash}"
    log_virtuoso "Virtuoso data import completed successfully."
    return 0
  fi

  printf '%s\n' "${load_output}" >&2
  log_virtuoso "Virtuoso data import failed."
  return 1
}

forward_signal() {
  if [ -n "${virtuoso_pid:-}" ] && kill -0 "${virtuoso_pid}" 2>/dev/null; then
    kill "${virtuoso_pid}" 2>/dev/null || true
    wait "${virtuoso_pid}" 2>/dev/null || true
  fi
}

trap forward_signal EXIT INT TERM
log_virtuoso "Virtuoso service starting."
/usr/bin/virtuoso-t -f -c "${VIRTUOSO_INI_PATH}" +pwddba "${VIRTUOSO_DBA_PASSWORD}" +pwddav "${VIRTUOSO_DBA_PASSWORD}" &
virtuoso_pid=$!
wait_for_virtuoso_http
load_sources_if_needed

trap - EXIT INT TERM
wait "${virtuoso_pid}"
