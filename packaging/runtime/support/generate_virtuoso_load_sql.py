#!/usr/bin/env python3
import os
import sys
from pathlib import Path

from runtime_state import read_manifest


def sql_string(value: str | None) -> str:
    if value is None:
        return "NULL"
    return "'" + value.replace("'", "''") + "'"


def load_sql_lines(manifest: dict) -> list[str]:
    lines = []
    for source in manifest["sources"]:
        path = Path(source["path"])
        graph = sql_string(source["graph"])
        data_format = source["format"]

        if data_format == "ttl":
            lines.append(
                f"DB.DBA.TTLP_MT(file_to_string_output({sql_string(str(path))}), '', {graph}, 0, 0, 0, 0);"
            )
        elif data_format in {"nt", "nq"}:
            lines.append(
                f"ld_dir({sql_string(str(path.parent))}, {sql_string(path.name)}, {graph});"
            )
            lines.append("rdf_loader_run();")
        else:
            raise ValueError(f"Unsupported format in source manifest: {data_format}")

        lines.append("checkpoint;")

    return lines


def main() -> int:
    source_manifest_path = Path(os.environ.get("SOURCE_MANIFEST_PATH", "/data/sources/source-manifest.json"))
    load_sql_path = Path(os.environ.get("VIRTUOSO_LOAD_SQL_PATH", "/data/virtuoso/load.sql"))

    try:
        manifest = read_manifest(source_manifest_path)
    except (FileNotFoundError, OSError, ValueError) as error:
        print(str(error), file=sys.stderr)
        return 1

    try:
        lines = load_sql_lines(manifest)
    except ValueError as error:
        print(str(error), file=sys.stderr)
        return 1

    load_sql_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
