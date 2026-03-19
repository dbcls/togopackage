#!/usr/bin/env python3
import os
import sys
from pathlib import Path

from runtime_state import ensure_current_generated_state, write_manifest
from source_data import prepare_input_manifest


def log(message: str) -> None:
    print(message, file=sys.stderr)


def virtuoso_config_text(
    *,
    db_dir: Path,
    data_dir: Path,
    source_dir: Path,
    http_port: str,
    isql_port: str,
) -> str:
    return f"""[Database]
DatabaseFile = {db_dir}/virtuoso.db
TransactionFile = {db_dir}/virtuoso.trx
ErrorLogFile = /proc/self/fd/2
LockFile = {db_dir}/virtuoso.lck
xa_persistent_file = {db_dir}/virtuoso.pxa
FileExtend = 200
MaxCheckpointRemap = 2000
Striping = 0

[TempDatabase]
DatabaseFile = {db_dir}/virtuoso-temp.db
TransactionFile = {db_dir}/virtuoso-temp.trx
MaxCheckpointRemap = 2000
Striping = 0

[Parameters]
ServerPort = {isql_port}
LiteMode = 0
DisableUnixSocket = 1
NumberOfBuffers = 3000000
MaxDirtyBuffers = 2250000
MaxCheckpointRemap = 2000
CheckpointInterval = 60
O_DIRECT = 0
CaseMode = 2
SchedulerInterval = 10
DirsAllowed = ., {data_dir}, {source_dir}
PrefixResultNames = 0
RdfFreeTextRulesSize = 100
IndexTreeMaps = 64
MaxStaticCursorRows = 5000
MaxQueryMem = 4G
DefaultHost = localhost:{http_port}

[HTTPServer]
ServerPort = {http_port}
ServerThreads = 10
MaxClientConnections = 10
EnabledDavVSP = 0
HTTPEnable = 1
MaintenancePage = atomic.html
DefaultClientCharset = UTF-8

[SPARQL]
ResultSetMaxRows = 10000
MaxQueryCostEstimationTime = 400
MaxQueryExecutionTime = 60
"""


def ensure_virtuoso_config(
    *,
    config_path: Path,
    db_dir: Path,
    data_dir: Path,
    source_dir: Path,
    http_port: str,
    isql_port: str,
) -> None:
    generated = virtuoso_config_text(
        db_dir=db_dir,
        data_dir=data_dir,
        source_dir=source_dir,
        http_port=http_port,
        isql_port=isql_port,
    )
    previous = config_path.read_text(encoding="utf-8") if config_path.exists() else None
    config_path.parent.mkdir(parents=True, exist_ok=True)
    config_path.write_text(generated, encoding="utf-8")
    if previous is None:
        log(f"Generated Virtuoso config at {config_path}.")


def reset_virtuoso_state(db_dir: Path, stamp_path: Path, load_sql_path: Path) -> None:
    if db_dir.exists():
        for path in db_dir.iterdir():
            if path.is_file():
                path.unlink()
    if stamp_path.exists():
        stamp_path.unlink()
    if load_sql_path.exists():
        load_sql_path.unlink()


def main() -> int:
    virtuoso_data_dir = Path(os.environ.get("VIRTUOSO_DATA_DIR", "/data/virtuoso"))
    qlever_data_dir = Path(os.environ.get("QLEVER_DATA_DIR", "/data/sources"))
    source_manifest_path = Path(os.environ.get("SOURCE_MANIFEST_PATH", f"{qlever_data_dir}/source-manifest.json"))
    virtuoso_ini_path = Path(os.environ.get("VIRTUOSO_INI_PATH", f"{virtuoso_data_dir}/virtuoso.ini"))
    virtuoso_load_sql_path = Path(os.environ.get("VIRTUOSO_LOAD_SQL_PATH", f"{virtuoso_data_dir}/load.sql"))
    virtuoso_http_port = os.environ.get("VIRTUOSO_HTTP_PORT", "8890")
    virtuoso_isql_port = os.environ.get("VIRTUOSO_ISQL_PORT", "1111")
    config_path = Path(os.environ.get("TOGOPACKAGE_CONFIG", "/data/config.yaml"))

    db_dir = virtuoso_data_dir / "db"
    stamp_path = virtuoso_data_dir / ".loaded-input-hash"
    db_dir.mkdir(parents=True, exist_ok=True)

    ensure_virtuoso_config(
        config_path=virtuoso_ini_path,
        db_dir=db_dir,
        data_dir=virtuoso_data_dir,
        source_dir=qlever_data_dir,
        http_port=virtuoso_http_port,
        isql_port=virtuoso_isql_port,
    )

    try:
        manifest = prepare_input_manifest(config_path, qlever_data_dir)
    except (FileNotFoundError, ValueError) as error:
        print(str(error), file=sys.stderr)
        return 1

    write_manifest(source_manifest_path, manifest)
    input_hash = str(manifest["input_hash"])

    ensure_current_generated_state(
        component="Virtuoso",
        stamp_path=stamp_path,
        input_hash=input_hash,
        state_exists=lambda: any(path.is_file() for path in db_dir.iterdir()),
        reset_state=lambda: reset_virtuoso_state(db_dir, stamp_path, virtuoso_load_sql_path),
        log=log,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
