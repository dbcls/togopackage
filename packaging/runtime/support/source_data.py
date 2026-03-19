#!/usr/bin/env python3
import bz2
import fcntl
import gzip
import glob
import hashlib
import json
import lzma
import shutil
import subprocess
import tempfile
import urllib.request
from contextlib import contextmanager
from pathlib import Path
from urllib.parse import urlparse


def unique_tmp_path(dest: Path) -> Path:
    dest.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(
        prefix=f"{dest.name}.",
        suffix=".tmp",
        dir=str(dest.parent),
    )
    Path(tmp_name).unlink()
    return Path(tmp_name)


@contextmanager
def hold_output_lock(dest: Path):
    dest.parent.mkdir(parents=True, exist_ok=True)
    lock_path = dest.parent / f".{dest.name}.lock"
    with lock_path.open("a+", encoding="utf-8") as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_UN)


def download(url: str, dest: Path) -> None:
    with hold_output_lock(dest):
        if dest.exists():
            return
        tmp = unique_tmp_path(dest)
        try:
            with urllib.request.urlopen(url) as response, tmp.open("wb") as out:
                shutil.copyfileobj(response, out)
            tmp.replace(dest)
        finally:
            tmp.unlink(missing_ok=True)


def maybe_decompress(path: Path) -> Path:
    decompressor = None
    if path.suffix == ".gz":
        decompressor = gzip.open
    elif path.suffix == ".bz2":
        decompressor = bz2.open
    elif path.suffix == ".xz":
        decompressor = lzma.open

    if decompressor is None and path.suffix not in {".zst", ".zstd"}:
        return path

    target = path.with_suffix("")
    with hold_output_lock(target):
        tmp = unique_tmp_path(target)
        try:
            if decompressor is not None:
                with decompressor(path, "rb") as src, tmp.open("wb") as dst:
                    shutil.copyfileobj(src, dst)
            else:
                with tmp.open("wb") as dst:
                    subprocess.run(["zstd", "-d", "-q", "-c", str(path)], check=True, stdout=dst)
            tmp.replace(target)
        finally:
            tmp.unlink(missing_ok=True)
    return target


def local_copy_target(source_path: Path, idx: int, match_idx: int, data_dir: Path) -> Path:
    suffix = "".join(source_path.suffixes)
    return data_dir / f"source_{idx}_{match_idx}{suffix}"


def remote_download_target(url: str, idx: int, data_dir: Path) -> Path:
    parsed = urlparse(url)
    name = Path(parsed.path).name
    if not name:
        return data_dir / f"source_{idx}"
    return data_dir / f"source_{idx}_{name}"


def resolve_local_path(path_value: str, config_path: Path) -> Path:
    path = Path(path_value)
    if path.is_absolute():
        return path
    return config_path.parent / path


def resolve_local_paths(path_value: str, config_path: Path, idx: int) -> list[Path]:
    resolved_path = resolve_local_path(path_value, config_path)
    matches = sorted(Path(path) for path in glob.glob(str(resolved_path), recursive=True))
    if not matches:
        raise FileNotFoundError(f"Local source not found for source #{idx}: {resolved_path}")
    file_matches = [path for path in matches if path.is_file()]
    if not file_matches:
        raise ValueError(f"Local source does not match any files for source #{idx}: {resolved_path}")
    return file_matches


def load_config(config_path: Path) -> dict:
    import yaml

    if not config_path.exists():
        raise FileNotFoundError(f"Config not found: {config_path}")
    with config_path.open("r", encoding="utf-8") as handle:
        return yaml.safe_load(handle) or {}


def normalize_format(data_format: object, *, context: str) -> str:
    if data_format is None:
        return "ttl"
    if not isinstance(data_format, str) or not data_format.strip():
        raise ValueError(f"Invalid format in config.yaml: {context} must be a non-empty string when specified")
    normalized = data_format.strip()
    if normalized not in {"nt", "ttl", "nq"}:
        raise ValueError(
            f"Invalid format in config.yaml: {context} supports only nt, ttl, and nq"
        )
    return normalized


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def prepare_input_specs(config: dict, config_path: Path, data_dir: Path) -> list[tuple[str, str | None, str]]:
    sources = config.get("source", [])
    if not sources:
        raise ValueError("No sources found in config.yaml")

    data_dir.mkdir(parents=True, exist_ok=True)
    input_specs: list[tuple[str, str | None, str]] = []

    for idx, source in enumerate(sources):
        url = source.get("url")
        path_value = source.get("path")
        graph = source.get("graph")
        data_format = normalize_format(source.get("format"), context=f"source #{idx} format")

        if data_format == "nq" and graph is not None:
            raise ValueError(
                f"Invalid graph in config.yaml: source #{idx} must not specify graph when format is nq"
            )

        if bool(url) == bool(path_value):
            if url and path_value:
                raise ValueError(f"Specify only one of url or path for source #{idx}")
            raise ValueError(f"Missing url/path for source #{idx}")

        if url:
            target = remote_download_target(str(url), idx, data_dir)
            download(str(url), target)
            input_file = maybe_decompress(target)
            input_specs.append((str(input_file), graph, data_format))
            continue

        local_paths = resolve_local_paths(str(path_value), config_path, idx)
        for match_idx, local_path in enumerate(local_paths):
            target = local_copy_target(local_path, idx, match_idx, data_dir)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(local_path, target)
            input_file = maybe_decompress(target)
            input_specs.append((str(input_file), graph, data_format))

    return input_specs


def build_input_manifest(input_specs: list[tuple[str, str | None, str]]) -> dict:
    sources = []
    for path_value, graph, data_format in input_specs:
        path = Path(path_value)
        sources.append(
            {
                "path": str(path),
                "graph": graph,
                "format": data_format,
                "sha256": file_sha256(path),
            }
        )

    return {"sources": sources}


def manifest_signature(manifest: dict) -> str:
    payload = json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def prepare_input_manifest(config_path: Path, data_dir: Path) -> dict:
    config = load_config(config_path)
    input_specs = prepare_input_specs(config, config_path, data_dir)
    manifest = build_input_manifest(input_specs)
    manifest["input_hash"] = manifest_signature(manifest)
    return manifest
