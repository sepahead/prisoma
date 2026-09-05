"""Positive and negative admission/projection controls using named synthetic source bytes."""

import json
from contextlib import contextmanager
from zipfile import ZipFile

import pytest

from experiments.lewm import assets


def expected(data):
    return {"bytes": len(data), "sha256": assets.digest_bytes(data)}


def test_exact_artifact_snapshot(tmp_path):
    source = tmp_path / "source"
    source.write_bytes(b"synthetic fixture bytes")
    destination = tmp_path / "copy"
    assets.copy_verified(source, destination, expected(source.read_bytes()))
    assert destination.read_bytes() == source.read_bytes()


def test_digest_accepts_exact_byte_limit(tmp_path, monkeypatch):
    monkeypatch.setattr(assets, "MAX_ASSET_BYTES", 8)
    source = tmp_path / "source"
    source.write_bytes(b"12345678")
    assert assets.sha(source) == assets.digest_bytes(b"12345678")


def test_digest_stops_when_open_file_grows(tmp_path, monkeypatch):
    monkeypatch.setattr(assets, "MAX_ASSET_BYTES", 8)
    source = tmp_path / "source"
    source.write_bytes(b"1234")
    original_open = assets.regular_file
    consumed = []

    @contextmanager
    def growing_file(path, maximum):
        with original_open(path, maximum) as stream:
            with source.open("ab") as writer:
                writer.write(b"567890123456")

            class CountedReader:
                def read(self, count):
                    block = stream.read(count)
                    consumed.append(len(block))
                    return block

            yield CountedReader()

    monkeypatch.setattr(assets, "regular_file", growing_file)
    with pytest.raises(ValueError, match="digest byte bound"):
        assets.sha(source)
    assert sum(consumed) == 9


def test_decoder_consumes_admitted_bytes_after_in_place_source_mutation(tmp_path):
    source = tmp_path / "source"
    original = b"synthetic admitted checkpoint bytes"
    source.write_bytes(original)

    def decoder(stream):
        # This changes the inode after admission, before the decoder reads.
        with source.open("r+b") as mutable:
            mutable.write(b"x" * len(original))
        return stream.read()

    decoded = assets.decode_verified_snapshot(source, expected(original), decoder)
    assert source.read_bytes() != original
    assert decoded == original


def test_changed_snapshot_rejects_without_calling_decoder(tmp_path):
    source = tmp_path / "source"
    source.write_bytes(b"changed")
    calls = []
    with pytest.raises(ValueError, match="identity mismatch"):
        assets.decode_verified_snapshot(
            source, expected(b"correct"), lambda stream: calls.append(stream.read())
        )
    assert calls == []


@pytest.mark.parametrize("change", ["digest", "size", "symlink", "directory"])
def test_artifact_rejects_invalid_identity_or_kind(tmp_path, change):
    source = tmp_path / "source"
    source.write_bytes(b"synthetic fixture bytes")
    spec = expected(source.read_bytes())
    if change == "digest":
        spec["sha256"] = "0" * 64
    elif change == "size":
        spec["bytes"] += 1
    elif change == "symlink":
        link = tmp_path / "link"
        link.symlink_to(source)
        source = link
    else:
        source = tmp_path
    with pytest.raises((ValueError, OSError)):
        assets.copy_verified(source, tmp_path / "copy", spec)


@pytest.mark.parametrize("name", ["../escape", "/absolute", "a/./b", "a//b", "a\\b"])
def test_artifact_path_is_closed(tmp_path, name):
    with pytest.raises(ValueError):
        assets.relative_path(tmp_path, name)


@pytest.mark.parametrize("text", ['{"a":1,"a":2}', '{"value":NaN}', "[]"])
def test_closed_json_rejects_ambiguous_payload(tmp_path, text):
    path = tmp_path / "input.json"
    path.write_text(text)
    with pytest.raises(ValueError):
        assets.read_json(path)


@pytest.fixture
def staged(tmp_path, monkeypatch):
    source = tmp_path / "inputs"
    source.mkdir()
    data = b"VALUE = 7\n"
    wheel = source / "synthetic.whl"
    with ZipFile(wheel, "w") as archive:
        archive.writestr("upstream/module.py", data)
    spec = {
        "assets": {wheel.name: expected(wheel.read_bytes())},
        "files": {
            "synthetic/module.py": {
                "asset": wheel.name,
                "member": "upstream/module.py",
                **expected(data),
            }
        },
        "generated": {"synthetic/__init__.py": '"""Synthetic admission control."""\n'},
        "rights": "synthetic_control_only",
    }
    monkeypatch.setattr(assets, "projection", lambda: spec)
    output = tmp_path / "staged"
    assets.stage(source, output)
    return source, output


def test_projection_retains_exact_bytes_and_ordinary_package(staged):
    _, output = staged
    manifest = assets.verify_stage(output)
    assert manifest["files"]["synthetic/module.py"]["kind"] == "unchanged"
    assert (output / "packages/synthetic/module.py").read_bytes() == b"VALUE = 7\n"
    assert (output / "packages/synthetic/__init__.py").is_file()


@pytest.mark.parametrize(
    "change", ["source", "extra", "missing", "manifest", "directory_symlink"]
)
def test_projection_drift_fails(staged, change):
    _, output = staged
    module = output / "packages/synthetic/module.py"
    if change == "source":
        module.write_bytes(b"VALUE = 8\n")
    elif change == "extra":
        (output / "packages/synthetic/extra.py").write_text("VALUE = 0\n")
    elif change == "missing":
        module.unlink()
    elif change == "directory_symlink":
        actual = output / "actual"
        (output / "packages/synthetic").rename(actual)
        (output / "packages/synthetic").symlink_to(actual, target_is_directory=True)
    else:
        path = output / "source-manifest.json"
        payload = json.loads(path.read_text())
        payload["files"]["synthetic/module.py"]["sha256"] = "0" * 64
        path.write_text(json.dumps(payload))
    with pytest.raises(ValueError):
        assets.verify_stage(output)


def test_projection_never_overwrites(staged):
    source, output = staged
    with pytest.raises(FileExistsError):
        assets.stage(source, output)
