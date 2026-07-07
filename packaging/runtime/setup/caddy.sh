#!/usr/bin/env bash
set -euo pipefail

PUBLIC_PATH="${PUBLIC_PATH:-/}"
CADDY_GENERATED_CONFIG="${CADDY_GENERATED_CONFIG:-/tmp/togopackage-caddy/Caddyfile}"

mkdir -p "$(dirname "${CADDY_GENERATED_CONFIG}")"

write_common_routes() {
  local prefix="$1"
  local strip_prefix="$2"
  local proxy_strip=""

  if [ -n "${strip_prefix}" ]; then
    proxy_strip="    uri strip_prefix ${strip_prefix}"
  fi

  cat <<EOF
  handle ${prefix}/tabulae {
    redir * ${prefix}/tabulae/
  }
  handle ${prefix}/tabulae* {
    root * /data/tabulae/dist
    uri strip_prefix ${prefix}/tabulae
    file_server
  }
  @sparql_proxy_assets {
    path ${prefix}/sparql.html
    path ${prefix}/admin
    path ${prefix}/admin/*
    path ${prefix}/socket.io*
    path ${prefix}/*.js
    path ${prefix}/*.css
    path ${prefix}/*.svg
    path ${prefix}/*.eot
    path ${prefix}/*.ttf
    path ${prefix}/*.woff
    path ${prefix}/*.woff2
    path ${prefix}/*.map
  }
  handle @sparql_proxy_assets {
${proxy_strip}
    reverse_proxy 127.0.0.1:{\$SPARQL_PROXY_PORT}
  }
  handle ${prefix}/sparqlist* {
    reverse_proxy 127.0.0.1:{\$SPARQLIST_PORT}
  }
  handle ${prefix}/grasp* {
    reverse_proxy 127.0.0.1:{\$GRASP_PORT}
  }
  handle ${prefix}/rdf-config-mcp/mcp* {
    uri strip_prefix ${prefix}/rdf-config-mcp
    reverse_proxy 127.0.0.1:{\$RDF_CONFIG_MCP_PORT}
  }
  handle ${prefix}/mcp* {
${proxy_strip}
    reverse_proxy 127.0.0.1:{\$MCP_SERVER_PORT}
  }
  handle ${prefix}/sse {
${proxy_strip}
    reverse_proxy 127.0.0.1:{\$TOGOMCP_PORT}
  }
  handle ${prefix}/messages* {
${proxy_strip}
    reverse_proxy 127.0.0.1:{\$TOGOMCP_PORT}
  }
  handle ${prefix}/sparql* {
${proxy_strip}
    reverse_proxy 127.0.0.1:{\$SPARQL_PROXY_PORT}
  }
EOF
}

{
  cat <<'EOF'
{
  log default {
    output stdout
    format json
  }
}

:10005 {
EOF

  if [ "${PUBLIC_PATH}" = "/" ]; then
    write_common_routes "" ""
    cat <<'EOF'
  reverse_proxy 127.0.0.1:{$SUPERVISOR_HTTP_PORT}
}
EOF
  else
    cat <<EOF
  handle / {
    redir * ${PUBLIC_PATH}/
  }
  handle ${PUBLIC_PATH} {
    redir * ${PUBLIC_PATH}/
  }
EOF
    write_common_routes "${PUBLIC_PATH}" "${PUBLIC_PATH}"
    cat <<EOF
  handle ${PUBLIC_PATH}/* {
    uri strip_prefix ${PUBLIC_PATH}
    reverse_proxy 127.0.0.1:{\$SUPERVISOR_HTTP_PORT}
  }
  respond 404
}
EOF
  fi
} > "${CADDY_GENERATED_CONFIG}"
