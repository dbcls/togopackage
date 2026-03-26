# TogoPackage

TogoPackage is a container image that bundles RDF and bioinformatics services into one runtime.
The primary way to use it is to pull `ghcr.io/dbcls/togopackage:latest`, bind-mount a data directory, and access the services through `http://localhost:10005`.
Building a local image from this repository is intended for development work on TogoPackage itself.

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
- [Use Docker or Podman](#use-docker-or-podman)
- [Developer Workflow](#developer-workflow)
- [Stop and Restart](#stop-and-restart)
- [Common Pitfalls](#common-pitfalls)
- [Repository Layout](#repository-layout)
- [Component READMEs](#component-readmes)

## Quick Start

Prerequisite: Docker or Podman.

1. Pull `ghcr.io/dbcls/togopackage:latest`.
2. Prepare `config.yaml` and source files in your bind-mounted data directory, or use `./data.example` for the bundled demo.
3. Start the container.
4. Open `http://localhost:10005/`.
5. If startup is still in progress, inspect logs with your container runtime.

The container should run as the calling user because the runtime writes generated files, caches, and database state back into the bind-mounted data directory.

Minimal example with Docker:

```bash
docker pull ghcr.io/dbcls/togopackage:latest
docker run -d --name togopackage \
  -u "$(id -u):$(id -g)" \
  -p 10005:10005 -p 7001:7001 -p 8890:8890 \
  -v "$(pwd)/data.example:/data" \
  ghcr.io/dbcls/togopackage:latest
```

Minimal example with Podman:

```bash
podman pull ghcr.io/dbcls/togopackage:latest
podman run -d --name togopackage \
  --userns keep-id -u "$(id -u):$(id -g)" \
  -p 10005:10005 -p 7001:7001 -p 8890:8890 \
  -v "$(pwd)/data.example:/data" \
  ghcr.io/dbcls/togopackage:latest
```

This uses the tracked files under `./data.example/` as a demo input, including a small RDF-config example under `./data.example/rdf-config/`.

To use a different bind-mounted directory with Docker:

```bash
docker run -d --name togopackage \
  -u "$(id -u):$(id -g)" \
  -p 10005:10005 -p 7001:7001 -p 8890:8890 \
  -v "/path/to/data:/data" \
  ghcr.io/dbcls/togopackage:latest
```

To use a different bind-mounted directory with Podman:

```bash
podman run -d --name togopackage \
  --userns keep-id -u "$(id -u):$(id -g)" \
  -p 10005:10005 -p 7001:7001 -p 8890:8890 \
  -v "/path/to/data:/data" \
  ghcr.io/dbcls/togopackage:latest
```

The bundled demo configuration is:

```yaml
source:
  - name: Demo dataset
    path: ./sources/demo.ttl
    format: ttl
    graph: http://example.org/graph/demo
```

The container publishes:

- `10005`: public entrypoint through Caddy
- `7001`: direct QLever port
- `8890`: direct Virtuoso HTTP port

## Prepare config.yaml

`/data/config.yaml` is the main runtime input definition.
Each `source` entry must specify `path`.
You can choose which SPARQL backend TogoPackage uses with `sparql_backend`.
You can choose which MCP server TogoPackage exposes on `/mcp` with `mcp_server`.

In the examples above, the host-side bind-mounted directory is `./data.example` or `/path/to/data`.
TogoPackage reads `config.yaml` from that mounted directory.

The repository includes demo input files under `./data.example/`, including a small RDF-config example under `./data.example/rdf-config/`.
You can either use `./data.example` directly or use it as a reference when preparing your own bind-mounted directory.

```yaml
sparql_backend: qlever
mcp_server: togomcp
sparql_proxy:
  ADMIN_PASSWORD: your-sparql-proxy-admin-password
sparqlist:
  ADMIN_PASSWORD: your-sparqlist-admin-password
qlever:
  server:
    ACCESS_TOKEN: your-access-token
    MEMORY_FOR_QUERIES: 2G
    TIMEOUT: 30s
    CACHE_MAX_SIZE: 2G
    CACHE_MAX_SIZE_SINGLE_ENTRY: 1G
    CACHE_MAX_NUM_ENTRIES: "200"
virtuoso:
  server:
    DBA_PASSWORD: dba
    NUMBER_OF_BUFFERS: 1500000
    MAX_DIRTY_BUFFERS: 1125000
    MAX_CHECKPOINT_REMAP: 1000
    CHECKPOINT_INTERVAL: 60
    MAX_QUERY_MEM: 2G
    SERVER_THREADS: 10
    MAX_CLIENT_CONNECTIONS: 10

source:
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
- `sparql_backend` selects the backend used by TogoPackage for SPARQL serving and data preparation
- `sparql-proxy` forwards `/sparql` to the backend selected by `sparql_backend`
- Default `sparql_backend`: `qlever`
- `mcp_server` is optional. Supported values: `togomcp`, `rdf-config-mcp`
- `mcp_server` selects the MCP service exposed at `/mcp`
- `/sse` and `/messages` are only provided by `togomcp`
- Default `mcp_server`: `togomcp`
- `sparql_proxy` is optional
- `sparql_proxy.ADMIN_PASSWORD` is optional. Default: `password`
- `qlever` is optional
- `qlever.server` is optional
- `qlever.server.ACCESS_TOKEN` is optional. If omitted, QLever uses its own default behavior
- `qlever.server.MEMORY_FOR_QUERIES` is optional. Default: `2G`
- `qlever.server.TIMEOUT` is optional. If omitted, TogoPackage does not pass `--timeout`
- `qlever.server.CACHE_MAX_SIZE` is optional. If omitted, TogoPackage does not pass `--cache-max-size`
- `qlever.server.CACHE_MAX_SIZE_SINGLE_ENTRY` is optional. If omitted, TogoPackage does not pass `--cache-max-size-single-entry`
- `qlever.server.CACHE_MAX_NUM_ENTRIES` is optional. If omitted, TogoPackage does not pass `--cache-max-num-entries`
- `qlever.server.PERSIST_UPDATES` is optional. Default: `false`. Only `true` adds `--persist-updates`
- `sparqlist` is optional
- `sparqlist.ADMIN_PASSWORD` is optional. If omitted, SPARQList admin features are disabled
- `virtuoso` is optional
- `virtuoso.server` is optional
- `virtuoso.server.DBA_PASSWORD` is optional. Default: `dba`
- `virtuoso.server.NUMBER_OF_BUFFERS` is optional. Default: `1500000`
- `virtuoso.server.MAX_DIRTY_BUFFERS` is optional. Default: `1125000`
- `virtuoso.server.MAX_CHECKPOINT_REMAP` is optional. Default: `1000`
- `virtuoso.server.CHECKPOINT_INTERVAL` is optional. Default: `60`
- `virtuoso.server.MAX_QUERY_MEM` is optional. Default: `2G`
- `virtuoso.server.SERVER_THREADS` is optional. Default: `10`
- `virtuoso.server.MAX_CLIENT_CONNECTIONS` is optional. Default: `10`
- TogoPackage revokes `SPARQL_UPDATE` from Virtuoso's public `SPARQL` account during setup so the public endpoint stays read-only
- Virtuoso numeric tuning values are YAML integers. Use strings only for values with units such as `MAX_QUERY_MEM`
- Virtuoso ports and data paths cannot be changed from `config.yaml` because they are tied to other runtime services and exposed port assumptions
- `format` can be specified for each `source`
- `source.format` is optional. Default: `ttl`
- Supported formats: `nt`, `ttl`, `nq`
- Supported compressed source suffixes: `.gz`, `.bz2`, `.xz`, `.zst`, `.zstd`
- When a source `format` is `nq`, that source must not specify `graph`
- Relative `path` values are resolved from the directory containing `config.yaml`
- Glob matches are expanded in sorted order
- Directories matched by a glob are ignored
- `graph` is optional. If omitted, data is loaded into the default graph
- Virtuoso does not pin a special `DefaultGraph`; its default SPARQL dataset uses all loaded graphs
- `config.yaml` is parsed strictly by the supervisor. Unknown keys or invalid YAML cause startup to fail

## Open the Services

Open these URLs after the container starts:

- `/` -> supervisor dashboard
- `/logs` -> supervisor log viewer
- `/sparql` -> `sparql-proxy`
- `/sparqlist` -> `sparqlist`
- `/grasp` -> `grasp`
- `/tabulae` -> static files from `tabulae`
- `/mcp` -> MCP server selected by `mcp_server`
- `/sse` -> `togomcp` only
- `/messages` -> `togomcp` only

Direct container ports:

- `7001` -> `QLever`
- `8890` -> `Virtuoso`

## Data Directories

Main mounted runtime directory on the host: `/path/to/data` in generic examples, or `./data.example` in the bundled demo

- `/path/to/data/config.yaml`: main source definition
- `/path/to/data/qlever`: QLever index data
- `/path/to/data/virtuoso`: Virtuoso configuration, DB files, and load metadata
- `/path/to/data/sources`: prepared source files
- `/path/to/data/sparqlist`: generated SPARQList repository
- `/path/to/data/grasp`: generated Grasp resources
- `/path/to/data/tabulae/queries`: Tabulae query files
- `/path/to/data/tabulae/dist`: generated Tabulae output
- `/path/to/data/togomcp/mie`: user-provided MIE files
- `/path/to/data/rdf-config`: RDF-config models used by generators

If `/path/to/data/rdf-config` contains model directories with `model.yaml`, TogoPackage generates derived assets for supported services at startup.
If `/path/to/data/grasp` already contains `.graphql` files, TogoPackage keeps them and skips Grasp generation from RDF-config.

## Update Input Data

Normal workflow:

1. Update `config.yaml` or files under the bind-mounted data directory.
2. Restart the container.
3. Check container logs if indexing or generation takes time.

Important behavior:

- Source files are copied under the mounted data directory, typically `/path/to/data/sources`
- Restarting the container reuses prepared local copies until the source files change

## Generated Artifacts

This section summarizes what TogoPackage prepares at startup.

- The source manifest at `/path/to/data/sources/source-manifest.json` is prepared regardless of the selected backend
- QLever or Virtuoso preparation runs according to `sparql_backend`
- `QLever`
  - Builds or reuses indexes under `/path/to/data/qlever/index`
  - Tracks the current input hash in `/path/to/data/qlever/index/.loaded-input-hash`
  - Rebuilds when `/data/config.yaml` or resolved source files change
- Input refresh
  - Updating a `.gz` source also refreshes the decompressed file used by loaders
- `Virtuoso`
  - Generates `/tmp/togopackage-virtuoso/virtuoso.ini` on first startup
  - Stores DB files under `/path/to/data/virtuoso/db`
  - Reuses `/path/to/data/sources/source-manifest.json` directly
  - Writes `/path/to/data/virtuoso/load.sql`
  - Inserts `checkpoint;` after each source file load in `load.sql`
  - Reloads when `/data/config.yaml` or resolved source files change
- `sparqlist`
  - Generates repository files under `/path/to/data/sparqlist` from `/path/to/data/rdf-config`
  - Falls back to `/togo/defaults/sparqlist` when generation produces no files
- `grasp`
  - Keeps existing `.graphql` files under `/path/to/data/grasp` if present
  - Otherwise generates resources under `/path/to/data/grasp` from `/path/to/data/rdf-config`
  - Uses `/togo/defaults/grasp` only when `/path/to/data/grasp` has no `.graphql` files and `/path/to/data/rdf-config` has no model directories
- `tabulae`
  - Generates query files under `/path/to/data/tabulae/queries/layer1` when queries are absent
  - Builds output under `/path/to/data/tabulae/dist`
  - Generated query files include pagination metadata comments such as `# Paginate: 10000`
- `togomcp`
  - Rebuilds runtime MIE files only from `/path/to/data/togomcp/mie`
  - Rebuilds runtime endpoints automatically from runtime MIE files
  - Generated TogoMCP endpoints always target the local `sparql-proxy` endpoint
  - If no user-provided TogoMCP MIE files exist, TogoMCP starts with no runtime MIE files and an empty endpoints list
  - Removing a user-provided MIE file is reflected on the next container restart

To force regeneration for generated content, remove the corresponding directory under `/path/to/data` and restart the container.
For Grasp generated from RDF-config, remove `/path/to/data/grasp/*.graphql` first. If `.graphql` files remain there, they are treated as user-managed resources and are kept as-is.

## Use Docker or Podman

Docker example:

```bash
docker pull ghcr.io/dbcls/togopackage:latest
docker run -d --name togopackage \
  -u "$(id -u):$(id -g)" \
  -p 10005:10005 -p 7001:7001 -p 8890:8890 \
  -v "$(pwd)/data:/data" \
  ghcr.io/dbcls/togopackage:latest
```

Podman example:

```bash
podman pull ghcr.io/dbcls/togopackage:latest
podman run -d --name togopackage \
  --userns keep-id -u "$(id -u):$(id -g)" \
  -p 10005:10005 -p 7001:7001 -p 8890:8890 \
  -v "$(pwd)/data:/data" \
  ghcr.io/dbcls/togopackage:latest
```

Both runtimes should start the container as the calling user so the bind-mounted data directory stays writable.
With rootless Podman, `--userns keep-id` preserves that mapping on the bind mount.

## Developer Workflow

Pulling `ghcr.io/dbcls/togopackage:latest` is the primary user workflow.
Building a local image from this repository is for development when you are changing TogoPackage itself.

The Makefile uses `podman` by default when available, otherwise `docker`.
You can still override the runtime through `CONTAINER_RUNTIME`.
The local image tag defaults to `dbcls/togopackage`.

```bash
make build CONTAINER_RUNTIME=podman
make start CONTAINER_RUNTIME=podman
make stop CONTAINER_RUNTIME=podman
```

You can override the local image tag if needed:

```bash
make build IMAGE=ghcr.io/YOUR_ORG/togopackage CONTAINER_RUNTIME=docker
```

## Stop and Restart

Docker:

```bash
docker stop togopackage
docker rm togopackage
docker run -d --name togopackage \
  -u "$(id -u):$(id -g)" \
  -p 10005:10005 -p 7001:7001 -p 8890:8890 \
  -v "$(pwd)/data:/data" \
  ghcr.io/dbcls/togopackage:latest
```

With Podman:

```bash
podman stop togopackage
podman rm togopackage
podman run -d --name togopackage \
  --userns keep-id -u "$(id -u):$(id -g)" \
  -p 10005:10005 -p 7001:7001 -p 8890:8890 \
  -v "$(pwd)/data:/data" \
  ghcr.io/dbcls/togopackage:latest
```

## Common Pitfalls

- `/data/config.yaml` is required inside the container, so the host bind mount must provide `config.yaml` at its root
- `source` must not be empty
- Cached files under the mounted data directory are reused unless you remove them
- `sparqlist`, `grasp`, and `tabulae` generate richer output when `/data/rdf-config` is provided
- `tabulae` requires query files under `/data/tabulae/queries` or enough RDF-config input to generate them
- Keep using the same host directory for `/data` across restarts
- `make build` and `make start` are developer-oriented local image workflows, not the primary user workflow

## Repository Layout

Most users only need a bind-mounted data directory and the published container image.
If you work on this repository itself, these directories are the main entry points:

- `packaging/`: container build files, bundled defaults, and runtime setup scripts
- `supervisor/`: Rust-based process supervisor and dashboard server
- `vendor/`: bundled component repositories such as `sparql-proxy`, `sparqlist`, `grasp`, `rdf-config`, `rdf-config-mcp`, and `togomcp`
- `data/`: bind-mounted runtime state, generated artifacts, caches, and local inputs

## Component READMEs

- [vendor/sparql-proxy/README.md](vendor/sparql-proxy/README.md)
- [vendor/sparqlist/README.md](vendor/sparqlist/README.md)
- [vendor/grasp/README.md](vendor/grasp/README.md)
- [vendor/rdf-config/README.md](vendor/rdf-config/README.md)
- [vendor/rdf-config-mcp/README.md](vendor/rdf-config-mcp/README.md)
- [vendor/togomcp/README.md](vendor/togomcp/README.md)

QLever and Virtuoso do not currently have separate component READMEs in this repository.
