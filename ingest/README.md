# togopackage-ingest

`togopackage-ingest` is a small Rust CLI for preparing shared runtime data for TogoPackage.

It handles the common setup steps that are used by both QLever and Virtuoso:

- read the dataset configuration
- fetch source files
- decompress source files when needed
- build a source manifest with content hashes
- prepare the QLever index
- prepare Virtuoso configuration, load RDF data into a temporary Virtuoso process, and leave a ready-to-run database on disk

The crate is designed to be run by `supervisor`, not as an independently orchestrated service.

## Usage

Running `togopackage-ingest` executes the full shared setup flow.

The command:

- reads the config file
- downloads and normalizes source files into the shared source directory
- writes `source-manifest.json`
- builds the QLever index when the current input differs from the last prepared state
- starts a temporary Virtuoso process when the current input differs from the last prepared state
- loads RDF data into Virtuoso and shuts the temporary process down again

This binary is used by the `prepare-data` setup-only service in `supervisor`.

## Configuration

All commands use the same set of CLI options, and every option can also be provided through an environment variable.

| Option | Environment variable | Default |
| --- | --- | --- |
| `--config-path` | `TOGOPACKAGE_CONFIG` | `/data/config.yaml` |
| `--qlever-data-dir` | `QLEVER_DATA_DIR` | `/data/sources` |
| `--source-manifest-path` | `SOURCE_MANIFEST_PATH` | `${QLEVER_DATA_DIR}/source-manifest.json` |
| `--qlever-index-base` | `QLEVER_INDEX_BASE` | `/data/qlever/index/default` |
| `--virtuoso-data-dir` | `VIRTUOSO_DATA_DIR` | `/data/virtuoso` |
| `--virtuoso-ini-path` | `VIRTUOSO_INI_PATH` | `${VIRTUOSO_DATA_DIR}/virtuoso.ini` |
| `--virtuoso-http-port` | `VIRTUOSO_HTTP_PORT` | `8890` |
| `--virtuoso-isql-port` | `VIRTUOSO_ISQL_PORT` | `1111` |
| `--virtuoso-dba-password` | `VIRTUOSO_DBA_PASSWORD` | `dba` |
| `--virtuoso-number-of-buffers` | `VIRTUOSO_NUMBER_OF_BUFFERS` | `3000000` |
| `--virtuoso-max-dirty-buffers` | `VIRTUOSO_MAX_DIRTY_BUFFERS` | `2250000` |
| `--virtuoso-max-checkpoint-remap` | `VIRTUOSO_MAX_CHECKPOINT_REMAP` | `2000` |
| `--virtuoso-checkpoint-interval` | `VIRTUOSO_CHECKPOINT_INTERVAL` | `60` |
| `--virtuoso-max-query-mem` | `VIRTUOSO_MAX_QUERY_MEM` | `4G` |
| `--virtuoso-server-threads` | `VIRTUOSO_SERVER_THREADS` | `10` |
| `--virtuoso-max-client-connections` | `VIRTUOSO_MAX_CLIENT_CONNECTIONS` | `10` |

When `togopackage-ingest` is run through `supervisor`, these Virtuoso tuning values are normally sourced from `/data/config.yaml` under `virtuoso.server`.
Port and path variables still exist for the CLI, but TogoPackage intentionally keeps them fixed at the runtime level because other services and published container ports depend on them.

## Files Produced by the Setup Flow

The setup flow writes and updates these files under `/data` by default:

- source files in `/data/sources`
- source manifest at `/data/sources/source-manifest.json`
- QLever index files under `/data/qlever/index`
- Virtuoso config at `/data/virtuoso/virtuoso.ini`
- Virtuoso database files under `/data/virtuoso/db`
- state stamps such as `.loaded-input-hash`

## Supervisor Integration

`supervisor` controls startup order.

The intended flow is:

1. run `togopackage-ingest`
2. start QLever and Virtuoso after preparation succeeds
3. start services that depend on the SPARQL proxy after the SPARQL proxy is ready

Because `supervisor` owns the execution order, `togopackage-ingest` intentionally keeps its internal control flow simple and does not implement cross-process build locking.

## Examples

Run the full setup with defaults:

```bash
togopackage-ingest
```

Use a different config file:

```bash
togopackage-ingest --config-path /data/config.yaml
```
