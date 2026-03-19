#!/usr/bin/env python3
import fcntl
import json
import os
from contextlib import contextmanager
from pathlib import Path
from typing import Callable, Iterator


def read_manifest(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def read_manifest_hash(path: Path) -> str:
    return str(read_manifest(path)["input_hash"])


def write_manifest(path: Path, manifest: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def read_stamp(path: Path) -> str | None:
    if not path.exists():
        return None
    return path.read_text(encoding="utf-8").strip()


def write_stamp(path: Path, input_hash: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"{input_hash}\n", encoding="utf-8")


def log_up_to_date(component: str, log: Callable[[str], None]) -> None:
    log(f"{component} is up to date for current input. Skipped rebuild.")


def database_build_lock_path() -> Path:
    return Path(os.environ.get("DATABASE_BUILD_LOCK_PATH", "/data/.database-build.lock"))


@contextmanager
def hold_database_build_lock(
    component: str, log: Callable[[str], None]
) -> Iterator[None]:
    lock_path = database_build_lock_path()
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with lock_path.open("a+", encoding="utf-8") as lock_file:
        try:
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            log(f"{component} waiting for database build lock.")
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        log(f"{component} acquired database build lock.")
        try:
            yield
        finally:
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_UN)


def ensure_current_generated_state(
    *,
    component: str,
    stamp_path: Path,
    input_hash: str,
    state_exists: Callable[[], bool],
    reset_state: Callable[[], None],
    log: Callable[[str], None],
) -> None:
    stamp = read_stamp(stamp_path)
    if stamp is None:
        if state_exists():
            log(f"{component} state exists without input stamp. Resetting generated state.")
            reset_state()
        return

    if stamp == input_hash:
        return

    log(f"{component} input changed. Resetting generated state.")
    reset_state()
