# togopackage-ingest

`togopackage-ingest` is a small Rust CLI for preparing shared runtime data for TogoPackage.

It handles the common setup steps that are used by both QLever and Virtuoso:

- read the dataset configuration
- fetch source files
- decompress source files when needed
- build a source manifest with content hashes
- prepare the QLever index
- prepare Virtuoso configuration and input state

The crate is designed to be run by `supervisor`, not as an independently orchestrated service.

## Commands

### `prepare-data`

Runs the full shared setup flow.

This command:

- reads the config file
- downloads and normalizes source files into the shared source directory
- writes `source-manifest.json`
- builds the QLever index when the current input differs from the last prepared state
- resets and prepares Virtuoso state when the current input differs from the last prepared state

This is the command used by the `prepare-data` setup-only service in `supervisor`.

### `generate-virtuoso-load-sql`

Reads the source manifest, generates `load.sql` for Virtuoso, and prints the current `input_hash` to standard output.

The generated SQL reflects the file format declared in the manifest:

- Turtle sources use `DB.DBA.TTLP_MT`
- N-Triples and N-Quads sources use `ld_dir` and `rdf_loader_run`

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
| `--virtuoso-load-sql-path` | `VIRTUOSO_LOAD_SQL_PATH` | `${VIRTUOSO_DATA_DIR}/load.sql` |
| `--virtuoso-http-port` | `VIRTUOSO_HTTP_PORT` | `8890` |
| `--virtuoso-isql-port` | `VIRTUOSO_ISQL_PORT` | `1111` |

## Files Produced by the Setup Flow

The setup flow writes and updates these files under `/data` by default:

- source files in `/data/sources`
- source manifest at `/data/sources/source-manifest.json`
- QLever index files under `/data/qlever/index`
- Virtuoso config at `/data/virtuoso/virtuoso.ini`
- Virtuoso load script at `/data/virtuoso/load.sql`
- state stamps such as `.loaded-input-hash`

## Supervisor Integration

`supervisor` controls startup order.

The intended flow is:

1. run `togopackage-ingest prepare-data`
2. start QLever and Virtuoso after preparation succeeds
3. start services that depend on the SPARQL proxy after the SPARQL proxy is ready

Because `supervisor` owns the execution order, `togopackage-ingest` intentionally keeps its internal control flow simple and does not implement cross-process build locking.

## Examples

Run the full setup with defaults:

```bash
togopackage-ingest prepare-data
```

Use a different config file:

```bash
togopackage-ingest prepare-data --config-path /data/config.yaml
```

Generate Virtuoso load SQL:

```bash
togopackage-ingest generate-virtuoso-load-sql
```
