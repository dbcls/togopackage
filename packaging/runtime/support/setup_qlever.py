#!/usr/bin/env python3
import os
import sys
from pathlib import Path

from qlever_build_index import build_index
from runtime_state import (
    ensure_current_generated_state,
    hold_database_build_lock,
    log_up_to_date,
    write_manifest,
    write_stamp,
)
from source_data import prepare_input_manifest


def log(message: str) -> None:
    print(message, file=sys.stderr)


def reset_index(index_base: str) -> None:
    index_dir = Path(index_base).parent
    prefix = Path(index_base).name
    for path in index_dir.glob(f"{prefix}.*"):
        path.unlink()


def main() -> int:
    config_path = Path(os.environ.get("TOGOPACKAGE_CONFIG", "/data/config.yaml"))
    data_dir = Path(os.environ.get("QLEVER_DATA_DIR", "/data/sources"))
    source_manifest_path = Path(os.environ.get("SOURCE_MANIFEST_PATH", f"{data_dir}/source-manifest.json"))
    index_base = os.environ.get("QLEVER_INDEX_BASE", "/data/qlever/index/default")
    index_path = Path(f"{index_base}.index.pso")
    index_dir = index_path.parent
    stamp_path = index_dir / ".loaded-input-hash"

    try:
        manifest = prepare_input_manifest(config_path, data_dir)
    except (FileNotFoundError, ValueError) as error:
        print(str(error), file=sys.stderr)
        return 1

    write_manifest(source_manifest_path, manifest)
    input_hash = str(manifest["input_hash"])
    index_dir.mkdir(parents=True, exist_ok=True)

    ensure_current_generated_state(
        component="QLever",
        stamp_path=stamp_path,
        input_hash=input_hash,
        state_exists=index_path.exists,
        reset_state=lambda: reset_index(index_base),
        log=log,
    )

    if index_path.exists():
        log_up_to_date("QLever", log)
        return 0

    try:
        with hold_database_build_lock("QLever", log):
            if index_path.exists():
                log_up_to_date("QLever", log)
                return 0

            log("QLever indexing started.")
            build_index(manifest, index_base)
    except Exception as error:
        print(str(error), file=sys.stderr)
        log("QLever indexing failed.")
        return 1

    write_stamp(stamp_path, input_hash)
    log("QLever indexing completed successfully.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
