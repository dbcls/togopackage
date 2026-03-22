#!/usr/bin/env bash
set -euo pipefail

: "${VIRTUOSO_DATA_DIR:=/data/virtuoso}"
: "${VIRTUOSO_INI_PATH:=${VIRTUOSO_DATA_DIR}/virtuoso.ini}"
: "${VIRTUOSO_DBA_PASSWORD:=dba}"

exec /usr/bin/virtuoso-t -f -c "${VIRTUOSO_INI_PATH}" +pwddba "${VIRTUOSO_DBA_PASSWORD}" +pwddav "${VIRTUOSO_DBA_PASSWORD}"
