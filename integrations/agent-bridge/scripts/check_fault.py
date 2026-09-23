"""Retain scoped SDK, injected API, and directly owned process fault controls.

Only the native controls send SIGTERM, and only to their own waiting children.
The API substitution binary permits only a closed set of harmless imports.
"""

import argparse
import errno
import hashlib
import json
from pathlib import Path
import subprocess
import sys

SOURCES = (
    "darwin_fault.c",
    "fault_api_control.c",
    "audit_token_probe.c",
    "test_darwin_fault.py",
    "check_fault.py",
    "fault-decision.v1.json",
    "darwin_observer.c",
    "owned_observer.py",
)
CONTROL_IMPORTS = {
    "___chkstk_darwin",
    "___error",
    "___stack_chk_fail",
    "___stack_chk_guard",
    "___stderrp",
    "___stdoutp",
    "_bzero",
    "_exit",
    "_ferror",
    "_fflush",
    "_fprintf",
    "_memcpy",
    "_memset",
    "_printf",
    "_strcmp",
    "_strlen",
    "_strnlen",
    "_strtoull",
}
CASES = [
    *[
        (name, 10, 0)
        for name in (
            "argument_count",
            "argument_wrong_command",
            "argument_negative_pid",
            "argument_pid_overflow",
            "argument_signed_pid",
            "argument_space_pid",
            "argument_hex_pid",
            "argument_empty_birth",
            "argument_zero_birth",
            "argument_birth_overflow",
            "argument_bad_micros",
            "argument_zero_parent",
            "argument_relative_path",
            "argument_long_path",
        )
    ],
    *[
        (name, 20, 0)
        for name in (
            "unknown_observation",
            "missing_observation",
            "short_observation",
            "wrong_pid",
            "wrong_parent",
            "wrong_uid",
            "wrong_ruid",
            "zero_status",
            "zombie",
            "wrong_birth",
            "wrong_micros",
        )
    ],
    *[
        (name, 21, 0)
        for name in ("task_name_failure", "null_task_name", "dead_task_name")
    ],
    *[
        (name, 22, 0)
        for name in (
            "task_info_failure",
            "short_token",
            "long_token",
            "token_pid",
            "token_uid",
            "token_ruid",
            "release_failure",
        )
    ],
    *[
        (name, 23, 0)
        for name in (
            "path_failure",
            "path_negative",
            "path_full",
            "path_unterminated",
            "path_wrong",
            "path_length_mismatch",
            "path_token_mutation",
        )
    ],
    *[
        (name, 24, 0)
        for name in (
            "parent_drift",
            "uid_drift",
            "ruid_drift",
            "becomes_zombie",
            "birth_drift",
            "micros_drift",
        )
    ],
    ("healthy", 0, 1),
    ("argument_path_limit", 0, 1),
    ("kernel_version_reject", 26, 1),
]


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
    frozen = output / "source"
    frozen.mkdir(mode=0o700)
    source = Path(__file__).resolve().parent
    result = {
        "schema": "local.darwin-fault-controls.v1",
        "status": "FAILED",
        "source_sha256": {},
        "checks": [],
        "injected_controls": [],
        "application_cleanup_qualified": False,
        "hostile_process_containment": False,
    }

    def command(label, argv, *, expected=0, timeout=30):
        try:
            completed = subprocess.run(argv, capture_output=True, timeout=timeout)
        except subprocess.TimeoutExpired as error:
            (output / f"{label}.stdout").write_bytes(error.stdout or b"")
            (output / f"{label}.stderr").write_bytes(error.stderr or b"")
            result["checks"].append({"label": label, "timed_out": True})
            raise
        (output / f"{label}.stdout").write_bytes(completed.stdout)
        (output / f"{label}.stderr").write_bytes(completed.stderr)
        result["checks"].append(
            {
                "label": label,
                "argv": list(map(str, argv)),
                "exit_code": completed.returncode,
                "stdout_sha256": sha256(completed.stdout),
                "stderr_sha256": sha256(completed.stderr),
            }
        )
        require(completed.returncode == expected, f"{label}: unexpected exit")
        return completed

    try:
        for name in SOURCES:
            raw = (source / name).read_bytes()
            (frozen / name).write_bytes(raw)
            result["source_sha256"][name] = sha256(raw)
        clang = (
            command("compiler-path", ["xcrun", "--find", "clang"])
            .stdout.decode()
            .strip()
        )
        sdk = command("sdk-path", ["xcrun", "--show-sdk-path"]).stdout.decode().strip()
        command("compiler-version", [clang, "--version"])
        result["sdk_headers_sha256"] = {
            name: sha256((Path(sdk) / "usr/include" / name).read_bytes())
            for name in (
                "libproc.h",
                "sys/proc_info.h",
                "mach/task_info.h",
                "mach/message.h",
            )
        }
        flags = ["-std=c11", "-Wall", "-Wextra", "-Werror", "-O2", "-isysroot", sdk]
        for name in (
            "darwin_observer",
            "darwin_fault",
            "fault_api_control",
            "audit_token_probe",
        ):
            command(
                f"compile-{name}",
                [clang, *flags, str(frozen / f"{name}.c"), "-o", str(frozen / name)],
            )
        result["binary_sha256"] = {
            name: sha256((frozen / name).read_bytes())
            for name in (
                "darwin_fault",
                "fault_api_control",
                "audit_token_probe",
                "darwin_observer",
            )
        }
        # Reject any newly introduced real process operation before synthetic PIDs run.
        imports = command(
            "api-control-imports", ["nm", "-u", str(frozen / "fault_api_control")]
        )
        symbols = set(imports.stdout.decode().split())
        require(
            symbols <= CONTROL_IMPORTS, "synthetic API binary has unreviewed imports"
        )
        for name, exit_code, signals in CASES:
            completed = command(
                f"api-{name}",
                [str(frozen / "fault_api_control"), name],
                expected=exit_code,
                timeout=10,
            )
            calls = json.loads(completed.stderr)
            require(
                calls["signals"] == signals, f"{name}: unexpected signal call count"
            )
            if exit_code == 10:
                require(
                    not any(calls.values()),
                    f"{name}: malformed argument reached an API",
                )
            if signals:
                receipt = json.loads(completed.stdout)
                require(
                    calls
                    == {
                        "observations": 2,
                        "names": 1,
                        "infos": 1,
                        "releases": 1,
                        "paths": 1,
                        "signals": 1,
                        "requested_signal": 15,
                    },
                    f"{name}: incomplete pre-signal observations",
                )
                require(
                    receipt["pid"] == 777
                    and receipt["pid_version"] == 12
                    and receipt["signal"] == 15
                    and receipt["signal_result"]
                    == (errno.ESRCH if exit_code == 26 else 0),
                    f"{name}: signal receipt differs",
                )
            else:
                require(
                    not completed.stdout,
                    f"{name}: pre-signal failure emitted success evidence",
                )
            result["injected_controls"].append(
                {"case": name, "exit_code": exit_code, **calls}
            )
        command(
            "native-controls",
            [sys.executable, "-B", str(frozen / "test_darwin_fault.py")],
            timeout=60,
        )
        require(
            all(
                sha256((source / name).read_bytes()) == digest
                for name, digest in result["source_sha256"].items()
            ),
            "fault control source changed during the gate",
        )
        result["status"] = "PASS_SCOPED_FAULT_CONTROLS"
        result["injected_control_count"] = len(CASES)
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
