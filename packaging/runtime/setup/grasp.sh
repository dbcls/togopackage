#!/usr/bin/env bash
set -euo pipefail

RESOURCES_DIR="${RESOURCES_DIR:-${GRASP_RESOURCES_DIR:-/data/grasp}}"
RDF_CONFIG_BASE_DIR="${RDF_CONFIG_BASE_DIR:-/data/rdf-config}"
GRASP_SPARQL_ENDPOINT="${GRASP_SPARQL_ENDPOINT:-http://localhost:7002/sparql}"
: "${TOGOPACKAGE_DEFAULTS_DIR:=/togo/defaults}"

mkdir -p "${RESOURCES_DIR}"

log_grasp() {
  local message="$1"
  printf '%s\n' "${message}" >&2
}

# Respect existing hand-written Grasp resources.
if find "${RESOURCES_DIR}" -maxdepth 1 -type f -name "*.graphql" | grep -q .; then
  log_grasp "Existing Grasp resources found in ${RESOURCES_DIR}. Skipped RDF-config generation."
elif [ -d "${RDF_CONFIG_BASE_DIR}" ]; then
  tmp_rdf_config_root=""
  mapfile -t rdf_config_dirs < <(find "${RDF_CONFIG_BASE_DIR}" -mindepth 1 -maxdepth 1 -type d | sort)
  config_args=()
  for config_dir in "${rdf_config_dirs[@]:-}"; do
    if [ -f "${config_dir}/model.yaml" ]; then
      if [ -z "${tmp_rdf_config_root}" ]; then
        tmp_rdf_config_root="$(mktemp -d /tmp/grasp-rdf-config.XXXXXX)"
        cp -a "${RDF_CONFIG_BASE_DIR}/." "${tmp_rdf_config_root}/"
      fi
      config_name="$(basename "${config_dir}")"
      tmp_config_dir="${tmp_rdf_config_root}/${config_name}"
      if [ ! -f "${tmp_config_dir}/endpoint.yaml" ]; then
        cat > "${tmp_config_dir}/endpoint.yaml" <<EOF
endpoint: ${GRASP_SPARQL_ENDPOINT}
EOF
        log_grasp "Injected temporary endpoint.yaml for RDF-config: ${config_name}"
      fi
      config_args+=(--config "${tmp_config_dir}")
    fi
  done

  if [ "${#config_args[@]}" -gt 0 ]; then
    log_grasp "Grasp resource generation started."
    tmp_root_dir="$(mktemp -d /tmp/grasp-generated.XXXXXX)"
    tmp_grasp_dir="${tmp_root_dir}/resources"
    if rdf-config "${config_args[@]}" --grasp-ns "${tmp_grasp_dir}"; then
      rm -rf "${RESOURCES_DIR}"
      mkdir -p "${RESOURCES_DIR}"
      cp -a "${tmp_grasp_dir}/." "${RESOURCES_DIR}/"
      log_grasp "Generated Grasp resources from RDF-config: ${RDF_CONFIG_BASE_DIR}"
    else
      log_grasp "Failed to generate Grasp resources from RDF-config."
    fi
    rm -rf "${tmp_root_dir}"
    if [ -n "${tmp_rdf_config_root}" ]; then
      rm -rf "${tmp_rdf_config_root}"
    fi
  else
    if [ -d "${TOGOPACKAGE_DEFAULTS_DIR}/grasp" ]; then
      cp -a "${TOGOPACKAGE_DEFAULTS_DIR}/grasp/." "${RESOURCES_DIR}/"
      log_grasp "No RDF-config directory with model.yaml found. Copied default Grasp resources."
    else
      log_grasp "No RDF-config directory with model.yaml found. Skipped Grasp resource generation."
    fi
  fi
else
  if [ -d "${TOGOPACKAGE_DEFAULTS_DIR}/grasp" ]; then
    cp -a "${TOGOPACKAGE_DEFAULTS_DIR}/grasp/." "${RESOURCES_DIR}/"
    log_grasp "RDF_CONFIG_BASE_DIR not found: ${RDF_CONFIG_BASE_DIR}. Copied default Grasp resources."
  else
    log_grasp "RDF_CONFIG_BASE_DIR not found: ${RDF_CONFIG_BASE_DIR}. Skipped Grasp resource generation."
  fi
fi
