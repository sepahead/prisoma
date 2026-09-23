"""Compile the installed Darwin SDK observer and retain its scoped controls.

The gate snapshots its five source files into a new evidence directory. It
retains failed outputs and never overwrites previous evidence. These controls
qualify sampled process observations, not an application or hostile containment.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import resource
import signal
import subprocess
import sys
import tempfile

SOURCES = (
    "darwin_observer.c",
    "owned_observer.py",
    "test_owned_observer.py",
    "observer_api_control.c",
    "check_observer.py",
)
CASES = (
    ("negative_roster", "snapshot", 3, None),
    ("zero_roster", "snapshot", 3, None),
    ("full_roster", "snapshot", 3, None),
    ("max_admitted_roster", "snapshot", 0, (32767, 0, 32767)),
    ("one_valid", "snapshot", 0, (1, 0, 1)),
    ("unavailable", "snapshot", 0, (0, 1, 0)),
    ("missing", "snapshot", 0, (0, 0, 0)),
    ("unknown_zero", "snapshot", 4, None),
    ("negative_pid", "snapshot", 4, None),
    ("zero_pid", "snapshot", 0, (0, 0, 0)),
    ("negative_after_valid", "snapshot", 4, None),
    ("duplicate_pid", "snapshot", 0, (2, 0, 1)),
    ("short_info", "snapshot", 4, None),
    ("pid_mismatch", "snapshot", 4, None),
    ("zero_birth", "snapshot", 4, None),
    ("bad_microseconds", "snapshot", 4, None),
    ("one_valid", "selected", 0, None),
    ("birth_drift", "selected", 8, None),
    ("parent_drift", "selected", 8, None),
    ("uid_drift", "selected", 8, None),
    ("full_path", "selected", 7, None),
    ("empty_path", "selected", 9, None),
)


def sha256(raw):
    return hashlib.sha256(raw).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(sys.platform == "darwin", "the installed Darwin SDK is required")
    output = args.output.resolve()
    output.mkdir(mode=0o700)
    source = Path(__file__).resolve().parent
    frozen = output / "source"
    frozen.mkdir(mode=0o700)
    result = {
        "schema": "local.darwin-observer-controls.v1",
        "status": "FAILED",
        "source_sha256": {},
        "checks": [],
        "application_qualified": False,
        "hostile_process_containment": False,
    }

    def command(label, argv, *, expected=0, timeout=30):
        completed = subprocess.run(argv, capture_output=True, timeout=timeout)
        (output / f"{label}.stdout").write_bytes(completed.stdout)
        (output / f"{label}.stderr").write_bytes(completed.stderr)
        result["checks"].append(
            {
                "label": label,
                "argv": list(map(str, argv)),
                "exit_code": completed.returncode,
                "stdout_bytes": len(completed.stdout),
                "stdout_sha256": sha256(completed.stdout),
                "stderr_sha256": sha256(completed.stderr),
            }
        )
        require(completed.returncode == expected, f"{label}: unexpected exit")
        return completed.stdout

    try:
        for name in SOURCES:
            raw = (source / name).read_bytes()
            (frozen / name).write_bytes(raw)
            result["source_sha256"][name] = sha256(raw)
        clang = command("compiler-path", ["xcrun", "--find", "clang"]).decode().strip()
        sdk = command("sdk-path", ["xcrun", "--show-sdk-path"]).decode().strip()
        command("compiler-version", [clang, "--version"])
        result["sdk_headers_sha256"] = {
            name: sha256((Path(sdk) / "usr/include" / name).read_bytes())
            for name in ("libproc.h", "sys/proc_info.h", "sys/proc.h")
        }
        flags = ["-std=c11", "-Wall", "-Wextra", "-Werror", "-O2", "-isysroot", sdk]
        for name in ("darwin_observer", "observer_api_control"):
            command(
                f"compile-{name}",
                [clang, *flags, str(frozen / f"{name}.c"), "-o", str(frozen / name)],
            )
        result["binary_sha256"] = sha256((frozen / "darwin_observer").read_bytes())
        for case, operation, expected, roster in CASES:
            raw = command(
                f"api-{operation}-{case}",
                [str(frozen / "observer_api_control"), case, operation],
                expected=expected,
                timeout=10,
            )
            if roster is not None:
                value = json.loads(raw)
                observed = (
                    len(value["rows"]),
                    len(value["unavailable"]),
                    len({row["pid"] for row in value["rows"]}),
                )
                require(observed == roster, f"{case}: roster differs")
        command(
            "python-controls",
            [sys.executable, "-B", str(frozen / "test_owned_observer.py")],
            timeout=90,
        )

        def zero_file_limit():
            signal.signal(signal.SIGXFSZ, signal.SIG_IGN)
            resource.setrlimit(resource.RLIMIT_FSIZE, (0, 0))

        # The limit and disposition apply only to this test's observer child.
        with tempfile.TemporaryFile() as target:
            completed = subprocess.run(
                [str(frozen / "darwin_observer"), "selected", str(os.getpid())],
                stdout=target,
                stderr=subprocess.PIPE,
                preexec_fn=zero_file_limit,
                timeout=10,
            )
            target.seek(0)
            raw = target.read()
        result["stdout_flush_control"] = {
            "exit_code": completed.returncode,
            "stdout_bytes": len(raw),
            "stderr_sha256": sha256(completed.stderr),
        }
        (output / "stdout-flush.stderr").write_bytes(completed.stderr)
        require(
            completed.returncode == 5 and not raw,
            "final stdout flush did not fail closed",
        )
        require(
            all(
                sha256((source / name).read_bytes()) == digest
                for name, digest in result["source_sha256"].items()
            ),
            "source changed during observer controls",
        )
        result["status"] = "PASS_SCOPED_OBSERVER_CONTROLS"
        result["api_control_count"] = len(CASES)
    except BaseException as error:
        result["error_type"] = type(error).__name__
        result["error"] = str(error)[:2048]
        raise
    finally:
        (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(
        json.dumps({"status": result["status"], "output": str(output)}, sort_keys=True)
    )


if __name__ == "__main__":
    main()
