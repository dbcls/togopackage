#!/usr/bin/env bash
set -euo pipefail

REPOSITORY_PATH="${REPOSITORY_PATH:-${SPARQLIST_REPOSITORY_PATH:-/data/sparqlist}}"
RDF_CONFIG_BASE_DIR="${RDF_CONFIG_BASE_DIR:-/data/rdf-config}"
SPARQLIST_SPARQL_ENDPOINT="${SPARQLIST_SPARQL_ENDPOINT:-http://localhost:7002/sparql}"
: "${TOGOPACKAGE_DEFAULTS_DIR:=/togo/defaults}"

mkdir -p "${REPOSITORY_PATH}"

log_sparqlist() {
  local message="$1"
  printf '%s\n' "${message}" >&2
}

copy_default_repository() {
  rm -rf "${REPOSITORY_PATH}"
  mkdir -p "${REPOSITORY_PATH}"

  if [ -d "${TOGOPACKAGE_DEFAULTS_DIR}/sparqlist" ]; then
    cp -a "${TOGOPACKAGE_DEFAULTS_DIR}/sparqlist/." "${REPOSITORY_PATH}/"
    log_sparqlist "Copied default SPARQList repository."
    return 0
  fi

  log_sparqlist "Default SPARQList repository not found: ${TOGOPACKAGE_DEFAULTS_DIR}/sparqlist"
  return 1
}

generator_script="/togo/runtime/support/generate_sparqlist_from_rdf_config.rb"

if [ ! -d "${RDF_CONFIG_BASE_DIR}" ]; then
  log_sparqlist "RDF-config base directory does not exist. Falling back to default repository: ${RDF_CONFIG_BASE_DIR}"
  copy_default_repository
  exit $?
fi

if [ ! -x "${generator_script}" ]; then
  log_sparqlist "Generator script is not executable: ${generator_script}"
  exit 1
fi

log_sparqlist "SPARQList repository generation started."
tmp_root_dir="$(mktemp -d /tmp/sparqlist-generated.XXXXXX)"
tmp_repository_dir="${tmp_root_dir}/repository"
if "${generator_script}" --rdf-config-base-dir "${RDF_CONFIG_BASE_DIR}" --output-dir "${tmp_repository_dir}" --sparql-endpoint "${SPARQLIST_SPARQL_ENDPOINT}"; then
  if find "${tmp_repository_dir}" -type f -print -quit | grep -q .; then
    rm -rf "${REPOSITORY_PATH}"
    mkdir -p "${REPOSITORY_PATH}"
    cp -a "${tmp_repository_dir}/." "${REPOSITORY_PATH}/"
    log_sparqlist "Generated SPARQList repository from RDF-config: ${RDF_CONFIG_BASE_DIR}"
  else
    log_sparqlist "Generated SPARQList repository is empty. Falling back to default repository."
    rm -rf "${tmp_root_dir}"
    copy_default_repository
    exit $?
  fi
else
  status=$?
  rm -rf "${tmp_root_dir}"
  if [ "${status}" -eq 10 ]; then
    log_sparqlist "No SPARQList files generated from RDF-config. Falling back to default repository."
    copy_default_repository
    exit $?
  fi
  log_sparqlist "Failed to generate SPARQList repository from RDF-config."
  exit "${status}"
fi
rm -rf "${tmp_root_dir}"
