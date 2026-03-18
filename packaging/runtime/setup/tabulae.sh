#!/usr/bin/env bash
set -euo pipefail

TABULAE_QUERIES_DIR="${TABULAE_QUERIES_DIR:-/data/tabulae/queries}"
TABULAE_DIST_DIR="${TABULAE_DIST_DIR:-/data/tabulae/dist}"
RDF_CONFIG_BASE_DIR="${RDF_CONFIG_BASE_DIR:-/data/rdf-config}"
TABULAE_SPARQL_ENDPOINT="${TABULAE_SPARQL_ENDPOINT:-http://localhost:7002/sparql}"
: "${TOGOPACKAGE_DEFAULTS_DIR:=/togo/defaults}"
export HOME=/data
export DUCKDB_HOME=/data/.duckdb
generator_script="/togo/runtime/support/generate_tabulae_queries_from_rdf_config.rb"

mkdir -p "${TABULAE_QUERIES_DIR}/layer1" "${TABULAE_QUERIES_DIR}/layer2"
mkdir -p "${TABULAE_DIST_DIR}"
mkdir -p "${DUCKDB_HOME}"

log_tabulae() {
  local message="$1"
  printf '%s\n' "${message}" >&2
}

wait_for_sparql_endpoint() {
  local endpoint="$1"
  local attempt
  local response_file
  response_file="$(mktemp)"
  trap 'rm -f "${response_file}"' RETURN

  for attempt in $(seq 1 60); do
    if curl -fsS -o "${response_file}" \
      --get \
      --data-urlencode 'query=ASK {}' \
      --data-urlencode 'format=application/sparql-results+json' \
      "${endpoint}"; then
      return 0
    fi
    sleep 1
  done

  log_tabulae "Timed out waiting for SPARQL endpoint: ${endpoint}"
  return 1
}

has_tabulae_queries() {
  find "${TABULAE_QUERIES_DIR}" -type f \( -name '*.rq' -o -name '*.sql' \) -print -quit 2>/dev/null | grep -q .
}

if ! has_tabulae_queries; then
  log_tabulae "Tabulae query generation started."
  generated=false
  if [ -d "${RDF_CONFIG_BASE_DIR}" ]; then
    tmp_root_dir="$(mktemp -d /tmp/tabulae-generated.XXXXXX)"
    trap 'rm -rf "${tmp_root_dir}"' EXIT
    tmp_layer1_dir="${tmp_root_dir}/layer1"
    mkdir -p "${tmp_layer1_dir}"

    if ruby "${generator_script}" --rdf-config-base-dir "${RDF_CONFIG_BASE_DIR}" --output-dir "${tmp_layer1_dir}" --sparql-endpoint "${TABULAE_SPARQL_ENDPOINT}"; then
      if find "${tmp_layer1_dir}" -type f -name '*.rq' -print -quit | grep -q .; then
        cp -a "${tmp_layer1_dir}/." "${TABULAE_QUERIES_DIR}/layer1/"
        generated=true
        log_tabulae "Generated Tabulae layer1 queries from RDF-config: ${RDF_CONFIG_BASE_DIR}"
      fi
    else
      log_tabulae "Failed to generate Tabulae queries from RDF-config."
    fi
  else
    log_tabulae "RDF_CONFIG_BASE_DIR not found: ${RDF_CONFIG_BASE_DIR}. Skipped query generation."
  fi

  if [ "${generated}" = false ] && [ -d "${TOGOPACKAGE_DEFAULTS_DIR}/tabulae" ]; then
    cp -a "${TOGOPACKAGE_DEFAULTS_DIR}/tabulae/layer1/." "${TABULAE_QUERIES_DIR}/layer1/"
    cp -a "${TOGOPACKAGE_DEFAULTS_DIR}/tabulae/layer2/." "${TABULAE_QUERIES_DIR}/layer2/"
    log_tabulae "Copied default Tabulae queries."
  fi
fi

# Only build tabulae assets when queries exist; otherwise Caddy would crash-loop.
if has_tabulae_queries; then
  log_tabulae "Waiting for SPARQL endpoint: ${TABULAE_SPARQL_ENDPOINT}"
  wait_for_sparql_endpoint "${TABULAE_SPARQL_ENDPOINT}"
  log_tabulae "Tabulae build started."
  if tabulae --queries-dir "${TABULAE_QUERIES_DIR}" --dist-dir "${TABULAE_DIST_DIR}" build; then
    log_tabulae "Tabulae build completed successfully."
  else
    log_tabulae "Tabulae build failed; starting Caddy without refreshed tabulae assets."
  fi
else
  log_tabulae "Skipping tabulae build: no queries found in ${TABULAE_QUERIES_DIR}"
fi
