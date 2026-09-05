"""Bounded local asset admission and deterministic ordinary-package projection."""

from __future__ import annotations

from contextlib import contextmanager
import hashlib
import importlib.metadata
from io import BytesIO
import json
import os
from pathlib import Path
import platform
import stat
from typing import BinaryIO, Iterator
from zipfile import ZipFile

PACKAGE = Path(__file__).resolve().parent
MAX_JSON_BYTES = 1_048_576
MAX_ASSET_BYTES = 80 * 1024 * 1024


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha(path: Path | str) -> str:
    with regular_file(Path(path), MAX_ASSET_BYTES) as stream:
        result = hashlib.sha256()
        total = 0
        while block := stream.read(min(1024 * 1024, MAX_ASSET_BYTES - total + 1)):
            total += len(block)
            if total > MAX_ASSET_BYTES:
                raise ValueError("File grew beyond its digest byte bound")
            result.update(block)
        return result.hexdigest()


@contextmanager
def regular_file(path: Path, maximum: int) -> Iterator[BinaryIO]:
    """Open a bounded regular file without following its final symlink."""
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        info = os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_size > maximum:
            raise ValueError("Expected a bounded regular file")
        yield stream


def read_bytes(path: Path, maximum: int = MAX_JSON_BYTES) -> bytes:
    with regular_file(path, maximum) as stream:
        value = stream.read(maximum + 1)
    if len(value) > maximum:
        raise ValueError("File grew beyond its bound")
    return value


def decode_verified_snapshot(path: Path, expected: dict, decoder):
    """Decode only the immutable byte snapshot whose complete identity was checked."""
    size = expected["bytes"]
    if type(size) is not int or not 0 < size <= MAX_ASSET_BYTES:
        raise ValueError("Invalid artifact byte bound")
    snapshot = read_bytes(path, size)
    if len(snapshot) != size or digest_bytes(snapshot) != expected["sha256"]:
        raise ValueError("Artifact snapshot identity mismatch")
    return decoder(BytesIO(snapshot))


def _unique_object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Duplicate JSON key")
        result[key] = value
    return result


def read_json(path: Path) -> dict:
    value = json.loads(
        read_bytes(path),
        object_pairs_hook=_unique_object,
        parse_constant=lambda _: (_ for _ in ()).throw(ValueError("Nonfinite JSON")),
    )
    if not isinstance(value, dict):
        raise ValueError("Expected a JSON object")
    return value


def save_json(path: Path, value: dict) -> None:
    data = (
        json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n"
    ).encode()
    if len(data) > MAX_JSON_BYTES:
        raise ValueError("JSON output exceeds its bound")
    write_bytes(path, data)


def write_bytes(path: Path, data: bytes) -> None:
    with path.open("xb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())


def relative_path(root: Path, name: str) -> Path:
    path = Path(name)
    if (
        path.is_absolute()
        or not path.parts
        or any(p in (".", "..") for p in path.parts)
    ):
        raise ValueError("Unsafe relative artifact path")
    if path.as_posix() != name or "\\" in name:
        raise ValueError("Noncanonical artifact path")
    return root / path


def copy_verified(source: Path, destination: Path, expected: dict) -> None:
    """Hash the same bytes copied into the private, exclusive staging file."""
    size = expected["bytes"]
    if type(size) is not int or not 0 < size <= MAX_ASSET_BYTES:
        raise ValueError("Invalid artifact byte bound")
    with regular_file(source, size) as incoming:
        if os.fstat(incoming.fileno()).st_size != size:
            raise ValueError("Artifact size mismatch")
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open("xb") as outgoing:
            total = 0
            hasher = hashlib.sha256()
            while block := incoming.read(min(1024 * 1024, size - total + 1)):
                total += len(block)
                if total > size:
                    raise ValueError("Artifact grew during copy")
                hasher.update(block)
                outgoing.write(block)
            outgoing.flush()
            os.fsync(outgoing.fileno())
        if total != size or hasher.hexdigest() != expected["sha256"]:
            raise ValueError("Artifact digest mismatch")


def projection() -> dict:
    return read_json(PACKAGE / "projection.json")


def stage(asset_root: Path, output: Path) -> dict:
    """Stage exact inputs and unchanged upstream modules without importing them."""
    spec = projection()
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    try:
        for name, expected in spec["assets"].items():
            copy_verified(
                relative_path(asset_root, name),
                relative_path(output / "assets", name),
                expected,
            )
        source_root = output / "packages"
        files = {}
        for target, item in spec["files"].items():
            source = relative_path(output / "assets", item["asset"])
            if item["member"] is None:
                data = read_bytes(source, item["bytes"])
            else:
                with (
                    regular_file(source, MAX_ASSET_BYTES) as stream,
                    ZipFile(stream) as archive,
                ):
                    members = archive.namelist()
                    if len(members) != len(set(members)):
                        raise ValueError("Duplicate archive member")
                    info = archive.getinfo(item["member"])
                    if info.file_size != item["bytes"]:
                        raise ValueError("Unexpected projected member size")
                    data = archive.read(info)
            if len(data) != item["bytes"] or digest_bytes(data) != item["sha256"]:
                raise ValueError("Projected source identity mismatch")
            path = relative_path(source_root, target)
            path.parent.mkdir(parents=True, exist_ok=True)
            write_bytes(path, data)
            files[target] = {
                "sha256": digest_bytes(data),
                "bytes": len(data),
                "kind": "unchanged",
            }
        for target, text in spec["generated"].items():
            data = text.encode()
            path = relative_path(source_root, target)
            path.parent.mkdir(parents=True, exist_ok=True)
            write_bytes(path, data)
            files[target] = {
                "sha256": digest_bytes(data),
                "bytes": len(data),
                "kind": "initializer",
            }
        manifest = {
            "schema": "prisoma.lewm.staged-sources.v1",
            "projection_sha256": sha(PACKAGE / "projection.json"),
            "assets": spec["assets"],
            "files": files,
            "rights": spec["rights"],
            "meaning": "observed source bytes, not loaded-code attestation",
        }
        save_json(output / "source-manifest.json", manifest)
        verify_stage(output)
        return manifest
    except Exception as error:
        save_json(output / "failure.json", {"status": "failed", "error": str(error)})
        raise


def verify_stage(output: Path) -> dict:
    """Rejoin the complete source roster before any upstream import."""
    spec = projection()
    manifest = read_json(output / "source-manifest.json")
    if (
        set(manifest)
        != {"schema", "projection_sha256", "assets", "files", "rights", "meaning"}
        or manifest["schema"] != "prisoma.lewm.staged-sources.v1"
    ):
        raise ValueError("Invalid staged-source manifest shape")
    if manifest["projection_sha256"] != sha(PACKAGE / "projection.json"):
        raise ValueError("Changed projection identity")
    if manifest["assets"] != spec["assets"] or manifest["rights"] != spec["rights"]:
        raise ValueError("Changed source asset roster")
    expected = {
        name: {"sha256": item["sha256"], "bytes": item["bytes"], "kind": "unchanged"}
        for name, item in spec["files"].items()
    }
    expected.update(
        {
            name: {
                "sha256": digest_bytes(text.encode()),
                "bytes": len(text.encode()),
                "kind": "initializer",
            }
            for name, text in spec["generated"].items()
        }
    )
    if manifest["files"] != expected:
        raise ValueError("Changed source file roster")
    for folder, roster in (("packages", expected), ("assets", spec["assets"])):
        root = output / folder
        actual = {
            p.relative_to(root).as_posix()
            for p in root.rglob("*")
            if p.is_file() or p.is_symlink()
        }
        if actual != set(roster):
            raise ValueError("Missing or extra staged files")
        for name, item in roster.items():
            path = relative_path(root, name)
            if any(
                parent.is_symlink()
                for parent in path.parents
                if parent.is_relative_to(output)
            ):
                raise ValueError("Symlink in staged source path")
            if path.stat().st_size != item["bytes"] or sha(path) != item["sha256"]:
                raise ValueError("Changed staged source bytes")
    return manifest


def verify_runtime() -> dict:
    spec = read_json(PACKAGE / "runtime-profile.json")
    if (platform.python_version(), platform.system(), platform.machine()) != (
        spec["python"],
        spec["system"],
        spec["machine"],
    ):
        raise ValueError("Unsupported runtime profile")
    versions = {name: importlib.metadata.version(name) for name in spec["versions"]}
    if versions != spec["versions"]:
        raise ValueError("Runtime dependency version mismatch")
    return {
        "profile_sha256": sha(PACKAGE / "runtime-profile.json"),
        "versions": versions,
    }
