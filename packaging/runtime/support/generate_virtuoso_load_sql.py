#!/usr/bin/env python3
import os
import sys
from pathlib import Path

from runtime_state import read_manifest


def main() -> int:
    source_manifest_path = Path(os.environ.get("SOURCE_MANIFEST_PATH", "/data/sources/source-manifest.json"))
    load_sql_path = Path(os.environ.get("VIRTUOSO_LOAD_SQL_PATH", "/data/virtuoso/load.sql"))

    try:
        manifest = read_manifest(source_manifest_path)
    except (FileNotFoundError, OSError, ValueError) as error:
        print(str(error), file=sys.stderr)
        return 1

    def sql_string(value: str | None) -> str:
        if value is None:
            return "NULL"
        return "'" + value.replace("'", "''") + "'"

    lines = []
    for source in manifest["sources"]:
        path = sql_string(source["path"])
        graph = sql_string(source["graph"])
        lines.append(f"DB.DBA.TTLP_MT(file_to_string_output({path}), '', {graph}, 0, 0, 0, 0);")
        lines.append("checkpoint;")
    load_sql_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
