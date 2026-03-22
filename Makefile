SHELL := bash
.SHELLFLAGS := -eu -o pipefail -c

IMAGE ?= dbcls/togopackage
CONTAINER ?= togopackage
CONTAINER_RUNTIME ?= $(shell if command -v podman >/dev/null 2>&1; then echo podman; else echo docker; fi)
DATA_DIR ?= data
DATA_DIR_ABS := $(abspath $(DATA_DIR))
# The runtime writes generated files, caches, and database state under the
# bind-mounted data directory, so run the container as the calling user.
CONTAINER_RUN_USER_OPTIONS = -u "$$(id -u):$$(id -g)"
ifeq ($(CONTAINER_RUNTIME),podman)
# Rootless Podman also needs keep-id so the bind mount remains writable when
# host ownership is projected into the container.
CONTAINER_RUN_USER_OPTIONS = --userns keep-id -u "$$(id -u):$$(id -g)"
endif

.PHONY: default build start stop restart

default: build

build:
	$(if $(filter docker,$(CONTAINER_RUNTIME)),DOCKER_BUILDKIT=1 )$(CONTAINER_RUNTIME) build -f packaging/Dockerfile -t $(IMAGE) .

start:
	mkdir -p "$(DATA_DIR_ABS)/tabulae" "$(DATA_DIR_ABS)/virtuoso"
	$(CONTAINER_RUNTIME) run -d --name $(CONTAINER) $(CONTAINER_RUN_USER_OPTIONS) -p 10005:10005 -p 7001:7001 -p 8890:8890 -v "$(DATA_DIR_ABS):/data" $(IMAGE)

stop:
	$(CONTAINER_RUNTIME) stop $(CONTAINER) || true
	$(CONTAINER_RUNTIME) rm $(CONTAINER) || true

restart:
	$(MAKE) stop CONTAINER="$(CONTAINER)" CONTAINER_RUNTIME="$(CONTAINER_RUNTIME)" DATA_DIR="$(DATA_DIR)"
	$(MAKE) start CONTAINER="$(CONTAINER)" CONTAINER_RUNTIME="$(CONTAINER_RUNTIME)" DATA_DIR="$(DATA_DIR)"
