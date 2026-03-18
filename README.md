# TogoPackage

TogoPackage is a container image that bundles RDF and bioinformatics services into one runtime.
You prepare a bind-mounted data directory, start the container, and access the services through `http://localhost:7000`.
The default directory is `./data`, and you can override it with `DATA_DIR=/path/to/data`.

Available services:

- `sparql-proxy`: SPARQL endpoint and admin interface
- `QLever`: SPARQL backend
- `Virtuoso`: additional SPARQL backend
- `sparqlist`: SPARQL-based API builder
- `grasp`: GraphQL service for RDF resources
- `tabulae`: query-driven tabular data publisher
- `togomcp`: MCP server for RDF Portal databases
- dashboard: runtime status and log viewer

## Table of Contents

- [Quick Start](#quick-start)
- [Prepare config.yaml](#prepare-configyaml)
- [Open the Services](#open-the-services)
- [Data Directories](#data-directories)
- [Update Input Data](#update-input-data)
- [Generated Artifacts](#generated-artifacts)
- [Use Podman](#use-podman)
- [Stop and Restart](#stop-and-restart)
- [Common Pitfalls](#common-pitfalls)
- [Repository Layout](#repository-layout)
- [Component READMEs](#component-readmes)

## Quick Start

Prerequisite: Docker or Podman.

1. Prepare `config.yaml` and source files in your bind-mounted data directory, or use `./data.example` for the bundled demo.
2. Start the container with `make start`.
3. Open `http://localhost:7000/`.
4. If startup is still in progress, inspect logs with your container runtime, for example `podman logs -f togopackage` or `docker logs -f togopackage`.

`make start` runs the container as the calling user because the runtime writes generated files, caches, and database state back into the bind-mounted `./data` directory.

Minimal example:

```bash
make start DATA_DIR=./data.example
```

This uses the tracked files under `./data.example/` as a demo input, including a small RDF-config example under `./data.example/rdf-config/`.

To use a different bind-mounted directory:

```bash
make start DATA_DIR=./data.example
```

The bundled demo configuration is:

```yaml
source:
  - name: Demo dataset
    path: ./sources/demo.ttl
    format: ttl
    graph: http://example.org/graph/demo
```

The default `make start` command publishes:

- `7000`: public entrypoint through Caddy
- `7001`: direct QLever port
- `8890`: direct Virtuoso HTTP port

## Prepare config.yaml

`/data/config.yaml` is the main runtime input definition.
Each `source` entry must specify exactly one of `url` or `path`.
You can also choose which backend `sparql-proxy` forwards to with `sparql_backend`.

By default, the host-side bind-mounted directory is `./data`.
To use another directory, pass `DATA_DIR` to `make` commands.

The repository includes demo input files under `./data.example/`, including a small RDF-config example under `./data.example/rdf-config/`.
You can either use `./data.example` directly or use it as a reference when preparing your own bind-mounted directory.

```yaml
sparql_backend: qlever

source:
  - name: Example RDF source
    url: https://example.org/example.ttl.gz
    format: ttl
    graph: http://example.org/graph/main
  - name: Local RDF file
    path: ./sources/local.ttl.gz
    format: ttl
    graph: http://example.org/graph/local
  - name: Local RDF files by glob
    path: ./sources/**/*.ttl.gz
    format: ttl
    graph: http://example.org/graph/batch
  - name: BZip2-compressed RDF source
    path: ./sources/local.ttl.bz2
    format: ttl
  - name: XZ-compressed RDF source
    path: ./sources/local.ttl.xz
    format: ttl
  - name: N-Triples source
    path: ./sources/local.nt
    format: nt
  - name: N-Quads source
    path: ./sources/local.nq.zst
    format: nq
```

Rules:

- `sparql_backend` is optional. Supported values: `qlever`, `virtuoso`
- `sparql_backend` controls which backend `sparql-proxy` uses for `/sparql`
- Default `sparql_backend`: `qlever`
- `format` can be specified for each `source`
- `source.format` is optional. Default: `ttl`
- Supported formats: `nt`, `ttl`, `nq`
- Supported compressed source suffixes: `.gz`, `.bz2`, `.xz`, `.zst`, `.zstd`
- When a source `format` is `nq`, that source must not specify `graph`
- Relative `path` values are resolved from the directory containing `config.yaml`
- Glob matches are expanded in sorted order
- Directories matched by a glob are ignored
- `graph` is optional. If omitted, data is loaded into the default graph

## Open the Services

Open these URLs after `make start`:

- `/` -> supervisor dashboard
- `/logs` -> supervisor log viewer
- `/sparql` -> `sparql-proxy`
- `/sparqlist` -> `sparqlist`
- `/grasp` -> `grasp`
- `/tabulae` -> static files from `tabulae`
- `/mcp` -> `togomcp`
- `/sse` -> `togomcp`
- `/messages` -> `togomcp`

Direct container ports:

- `7001` -> `QLever`
- `8890` -> `Virtuoso`

## Data Directories

Main mounted runtime directory on the host: `./data` by default

- `./data/config.yaml`: main source definition
- `./data/qlever`: QLever index data
- `./data/virtuoso`: Virtuoso configuration, DB files, and load metadata
- `./data/sources`: downloaded or prepared source files
- `./data/sparqlist`: generated SPARQList repository
- `./data/grasp`: generated Grasp resources
- `./data/tabulae/queries`: Tabulae query files
- `./data/tabulae/dist`: generated Tabulae output
- `./data/togomcp/mie`: user-provided MIE files
- `./data/togomcp/endpoints.csv`: user-provided extra endpoints
- `./data/rdf-config`: RDF-config models used by generators

If you start with `DATA_DIR=...`, replace `./data` in the list above with that directory.

If `./data/rdf-config` contains model directories with `model.yaml`, TogoPackage generates derived assets for supported services at startup.
If `./data/grasp` already contains `.graphql` files, TogoPackage keeps them and skips Grasp generation from RDF-config.

## Update Input Data

Normal workflow:

1. Update `./data/config.yaml` or files under `./data`.
2. Run `make restart`.
3. Check `docker logs -f togopackage` if indexing or generation takes time.

When using a non-default bind-mounted directory, pass the same `DATA_DIR` value to `make restart`.

Important behavior:

- Remote `url` sources are cached under `./data/sources`
- Restarting the container does not automatically re-download an already cached URL
- To refresh upstream content at the same URL, remove the cached file first and then run `make restart`

## Generated Artifacts

This section summarizes what TogoPackage prepares at startup.

- `QLever`
  - Resolves source files into `./data/sources/source-manifest.json`
  - Builds or reuses indexes under `./data/qlever/index`
  - Tracks the current input hash in `./data/qlever/index/.loaded-input-hash`
  - Rebuilds when `/data/config.yaml` or resolved source files change
- Input refresh
  - Recreating a cached `.gz` source also refreshes the decompressed file used by loaders
- `Virtuoso`
  - Generates `./data/virtuoso/virtuoso.ini` on first startup
  - Stores DB files under `./data/virtuoso/db`
  - Reuses `./data/sources/source-manifest.json` directly
  - Writes `./data/virtuoso/load.sql`
  - Reloads when `/data/config.yaml` or resolved source files change
- `sparqlist`
  - Generates repository files under `./data/sparqlist` from `./data/rdf-config`
  - Falls back to `/togo/defaults/sparqlist` when generation produces no files
- `grasp`
  - Keeps existing `.graphql` files under `./data/grasp` if present
  - Otherwise generates resources under `./data/grasp` from `./data/rdf-config`
  - Uses `/togo/defaults/grasp` only when `./data/grasp` has no `.graphql` files and `./data/rdf-config` has no model directories
- `tabulae`
  - Generates query files under `./data/tabulae/queries/layer1` when queries are absent
  - Builds output under `./data/tabulae/dist`
  - Generated query files include pagination metadata comments such as `# Paginate: 10000`
- `togomcp`
  - Rebuilds runtime MIE files from bundled defaults plus `./data/togomcp/mie`
  - Rebuilds runtime endpoints from bundled defaults plus `./data/togomcp/endpoints.csv`
  - Removing a user-provided MIE file or endpoint row is reflected on the next `make restart`

To force regeneration for generated content, remove the corresponding directory under `./data` and run `make restart`.
For Grasp generated from RDF-config, remove `./data/grasp/*.graphql` first. If `.graphql` files remain there, they are treated as user-managed resources and are kept as-is.

## Use Podman

The Makefile uses `podman` by default when available, otherwise `docker`.
You can still override the runtime through `CONTAINER_RUNTIME`.
Both runtimes start the container as the calling user so bind-mounted `./data` stays writable.
With `podman`, `--userns keep-id` is added as well so rootless Podman preserves that mapping on the bind mount.

```bash
make build CONTAINER_RUNTIME=podman
make start CONTAINER_RUNTIME=podman
make stop CONTAINER_RUNTIME=podman
```

## Stop and Restart

```bash
make stop
make restart
```

With Podman:

```bash
make stop CONTAINER_RUNTIME=podman
make restart CONTAINER_RUNTIME=podman
```

## Common Pitfalls

- `./data/config.yaml` is required
- `source` must not be empty
- Cached files under `./data/sources` are reused unless you remove them
- `sparqlist`, `grasp`, and `tabulae` generate richer output when `./data/rdf-config` is provided
- `tabulae` requires query files under `./data/tabulae/queries` or enough RDF-config input to generate them
- When using `DATA_DIR=...`, keep using the same value for `start`, `stop`, and `restart`

## Repository Layout

Most users only need `./data`, `Makefile`, and the running services.
If you inspect the repository itself, these directories are the main entry points:

- `packaging/`: container build files, bundled defaults, and runtime setup scripts
- `supervisor/`: Rust-based process supervisor and dashboard server
- `vendor/`: bundled component repositories such as `sparql-proxy`, `sparqlist`, `grasp`, `tabulae`, and `togomcp`
- `data/`: bind-mounted runtime state, generated artifacts, caches, and local inputs

## Component READMEs

- [vendor/sparql-proxy/README.md](vendor/sparql-proxy/README.md)
- [vendor/sparqlist/README.md](vendor/sparqlist/README.md)
- [vendor/grasp/README.md](vendor/grasp/README.md)
- [vendor/tabulae/README.md](vendor/tabulae/README.md)
- [vendor/togomcp/README.md](vendor/togomcp/README.md)

QLever and Virtuoso do not currently have separate component READMEs in this repository.
