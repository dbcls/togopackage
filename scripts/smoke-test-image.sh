#!/usr/bin/env bash

set -euo pipefail

if (( $# != 3 )); then
  echo "Usage: $0 IMAGE CONTAINER HOST_PORT" >&2
  exit 2
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repository_root=$(cd -- "${script_dir}/.." && pwd)
image=$1
container=$2
host_port=$3
base_url="http://127.0.0.1:${host_port}"
smoke_data=$(mktemp -d)
container_started=false

cleanup() {
  result=$?
  if [[ "${container_started}" == "true" ]]; then
    if (( result != 0 )); then
      docker logs --tail 500 "${container}" || true
      curl --fail --silent --show-error --max-time 2 \
        "${base_url}/api/status" | jq . || true
    fi
    docker rm --force "${container}" >/dev/null 2>&1 || true
  fi
  rm -rf "${smoke_data}"
  exit "${result}"
}
trap cleanup EXIT

while IFS= read -r -d '' source; do
  relative_path=${source#data.example/}
  destination="${smoke_data}/${relative_path}"
  mkdir -p "$(dirname -- "${destination}")"
  cp -a "${repository_root}/${source}" "${destination}"
done < <(git -C "${repository_root}" ls-files -z -- data.example)

if ! docker image inspect "${image}" >/dev/null 2>&1; then
  docker pull "${image}"
fi
docker run --detach \
  --name "${container}" \
  --user "$(id -u):$(id -g)" \
  --publish "127.0.0.1:${host_port}:10005" \
  --volume "${smoke_data}:/data" \
  "${image}"
container_started=true

last_state=""
for _ in $(seq 1 150); do
  if [[ "$(docker inspect --format '{{.State.Running}}' "${container}")" != "true" ]]; then
    echo "Smoke test container stopped unexpectedly." >&2
    exit 1
  fi

  if status_json=$(curl --fail --silent --show-error --max-time 2 \
    "${base_url}/api/status" 2>/dev/null); then
    prepare_state=$(jq -r '.services["prepare-data"].state // "unknown"' <<<"${status_json}")
    qlever_state=$(jq -r '.services.qlever.state // "unknown"' <<<"${status_json}")
    current_state="prepare-data=${prepare_state} qlever=${qlever_state}"
    if [[ "${current_state}" != "${last_state}" ]]; then
      echo "${current_state}"
      last_state="${current_state}"
    fi

    if [[ "${prepare_state}" == "completed" && "${qlever_state}" == "running" ]]; then
      if response=$(curl --fail --silent --show-error --max-time 5 \
        --get \
        --header 'Accept: application/sparql-results+json' \
        --data-urlencode 'query=SELECT ?name WHERE { GRAPH <http://example.org/graph/demo> { <http://example.org/resource/alice> <http://xmlns.com/foaf/0.1/name> ?name } }' \
        "${base_url}/sparql" 2>/dev/null) \
        && jq -e '.results.bindings | any(.name.value == "Alice")' <<<"${response}" >/dev/null; then
        echo "Smoke test passed."
        exit 0
      fi
    fi
  fi

  sleep 2
done

echo "Timed out waiting for the demo SPARQL query to succeed." >&2
exit 1
