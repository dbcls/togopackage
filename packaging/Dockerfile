FROM docker.io/library/ubuntu:24.04 AS base
# Packages needed for both both building and running the binaries.
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8
ENV LC_CTYPE=C.UTF-8
ENV DEBIAN_FRONTEND=noninteractive

# Install the packages needed for building the binaries (this is a separate
# stage to keep the final image small).
FROM docker.io/adfreiburg/qlever:latest AS qlever-prebuilt
FROM ghcr.io/dbcls/tabulae:latest AS tabulae-prebuilt

FROM docker.io/library/rust:1.94-bullseye AS supervisor-build
WORKDIR /supervisor
COPY supervisor/Cargo.toml ./
COPY supervisor/Cargo.lock ./
COPY supervisor/src ./src
RUN cargo build --release

# Use prebuilt binaries to avoid building from source.
FROM base AS runtime
WORKDIR /qlever
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y wget python3-yaml unzip curl bzip2 xz-utils zstd pkg-config libicu74 python3-icu libgomp1 uuid-runtime make lbzip2 libjemalloc2 libzstd1 libboost-program-options1.83.0 libboost-iostreams1.83.0 libboost-url1.83.0 pipx bash-completion vim sudo ruby ruby-bundler ruby-dev build-essential git virtuoso-server virtuoso-opensource-7-bin && rm -rf /var/lib/apt/lists/*

# Set up user `togo` with temporary sudo rights (which will be removed again
# by the `docker-entrypoint.sh` script, see there).
RUN groupadd -r togo && useradd --no-log-init -d /togo -r -g togo togo
RUN mkdir -p /togo && chown togo:togo /qlever /togo
RUN echo "togo ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Set up a profile script that is sourced whenever a new login shell is
# started (by any user, hence in `/etc/profile.d/`). For some reason, podman
# wants it in the user's home `.bashrc`, so we also copy it there.
ENV QLEVER_PROFILE=/etc/profile.d/qlever.sh
RUN echo "eval \"\$(register-python-argcomplete qlever)\"" >> $QLEVER_PROFILE
RUN echo "export QLEVER_ARGCOMPLETE_ENABLED=1 && export QLEVER_IS_RUNNING_IN_CONTAINER=1" >> $QLEVER_PROFILE
RUN echo "PATH=/qlever:/togo/.local/bin:$PATH && PS1=\"\u@docker:\W\$ \"" >> $QLEVER_PROFILE
RUN echo 'alias ll="ls -l"' >> $QLEVER_PROFILE
RUN echo "if [ -d /data ]; then cd /data; fi" >> $QLEVER_PROFILE
RUN cp $QLEVER_PROFILE /togo/.bashrc

# Install the `qlever` command line tool. We have to set the two environment
# variables again here because in batch mode, the profile script above is not
# sourced. The `PATH` is set again to avoid a warning from `pipx`.
RUN mkdir -p /togo/.local && chown -R togo:togo /togo
USER togo
ENV HOME=/togo
ENV PATH=/qlever:/togo/.local/bin:$PATH
RUN pipx install qlever
ENV QLEVER_ARGCOMPLETE_ENABLED=1
ENV QLEVER_IS_RUNNING_IN_CONTAINER=1

# Copy the binaries.
COPY --from=qlever-prebuilt /qlever/qlever-index /qlever/qlever-index
COPY --from=qlever-prebuilt /qlever/qlever-server /qlever/qlever-server
COPY --from=qlever-prebuilt /qlever/*Main /qlever/
COPY --from=qlever-prebuilt /qlever/e2e/* /qlever/e2e/


# Switch to root for installing system binaries.
USER root

# Install Caddy.
ARG TARGETARCH
RUN curl -fsSL "https://caddyserver.com/api/download?os=linux&arch=${TARGETARCH}" -o /usr/bin/caddy && chmod +x /usr/bin/caddy

# Install uv (Python package manager) system-wide.
RUN curl -LsSf https://astral.sh/uv/install.sh | env UV_INSTALL_DIR=/usr/local/bin sh

# Install latest Node.js for sparql-proxy and grasp.
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash - \
  && apt-get install -y nodejs \
  && npm install -g tsx \
  && rm -rf /var/lib/apt/lists/*

COPY --chown=togo:togo vendor /vendor
ENV SPARQL_PROXY_DIR=/vendor/sparql-proxy
ENV SPARQLIST_DIR=/vendor/sparqlist
ENV GRASP_DIR=/vendor/grasp
ENV TOGOMCP_DIR=/vendor/togomcp

RUN install -d -m 0755 -o togo -g togo /togo/defaults
COPY --chown=togo:togo packaging/defaults /togo/defaults
COPY --from=tabulae-prebuilt /usr/local/bin/tabulae /usr/local/bin/tabulae
COPY --from=supervisor-build /supervisor/target/release/togopackage-supervisor /usr/local/bin/togopackage-supervisor

# Install sparql-proxy.
RUN cd /vendor/sparql-proxy && npm ci && npm run build

# Install sparqlist.
RUN cd /vendor/sparqlist && npm ci && ROOT_PATH=/sparqlist/ npm run build && rm -rf /vendor/sparqlist/repository

# Install grasp.
RUN cd /vendor/grasp && npm ci
RUN cp -a /vendor/grasp/examples/. /togo/defaults/grasp/

# Install togomcp.
RUN install -d -m 0777 /vendor/togomcp/mie
RUN chmod 0666 /vendor/togomcp/resources/endpoints.csv
RUN install -d -m 0755 -o togo -g togo /togo/defaults/togomcp/mie \
  && cp -a /vendor/togomcp/mie/. /togo/defaults/togomcp/mie/ \
  && cp /vendor/togomcp/resources/endpoints.csv /togo/defaults/togomcp/endpoints.csv \
  && chown togo:togo /togo/defaults/togomcp/endpoints.csv
USER togo
RUN cd /vendor/togomcp && uv sync --no-dev

# Install rdf-config.
USER root
RUN rm -f /vendor/rdf-config/.git \
  && cd /vendor/rdf-config \
  && git init \
  && git config --global --add safe.directory /vendor/rdf-config \
  && git add -A \
  && bundle install
ENV PATH="/vendor/rdf-config/bin:${PATH}"
ENV RUBYLIB="/vendor/rdf-config/lib"

# Copy runtime setup scripts after all subpackage setup to avoid invalidating
# heavy build layers during iterative config changes.
COPY --chmod=755 packaging/runtime /togo/runtime
COPY packaging/Caddyfile /etc/caddy/Caddyfile

# Switch back to togo for runtime.
USER togo

EXPOSE 10005 7001 7004 8890
ENTRYPOINT ["/usr/local/bin/togopackage-supervisor", "run"]
