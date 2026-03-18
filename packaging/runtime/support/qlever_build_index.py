#!/usr/bin/env python3
import json
import os
import subprocess
import sys
from pathlib import Path

from runtime_state import read_manifest


def build_index(manifest: dict, index_base: str) -> None:
    index_dir = Path(index_base).parent
    index_dir.mkdir(parents=True, exist_ok=True)

    cmd = ["/qlever/qlever-index", "-i", index_base]
    for source in manifest["sources"]:
        input_file = source["path"]
        graph = source["graph"]
        cmd.extend(["-f", input_file, "-F", source["format"]])
        if graph:
            cmd.extend(["-g", graph])

    subprocess.run(cmd, check=True, cwd=index_dir)


def main() -> int:
    manifest_path = Path(os.environ.get("SOURCE_MANIFEST_PATH", "/data/sources/source-manifest.json"))
    try:
        manifest = read_manifest(manifest_path)
    except (FileNotFoundError, json.JSONDecodeError) as error:
        print(str(error), file=sys.stderr)
        return 1

    index_base = os.environ.get("QLEVER_INDEX_BASE", "/data/qlever/index/default")
    build_index(manifest, index_base)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
