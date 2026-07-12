#!/usr/bin/env python3
"""Fail closed on repository-truth drift missed by prose heuristics.

This audit binds active commands and claims to the checked-out pid-rs gitlink/package rather than
to a remembered version string. It intentionally checks the exact regressions that accompanied the
1.0 migration: stale pins, omitted required features, an excluded consumer lock, false PID-off
firebreak wording, and generated notices that name the wrong local package version.
"""

from __future__ import annotations

import re
import subprocess
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PID_RS = ROOT / "pid-rs"
EXP0_COMMAND_FILES = [
    ROOT / "Cargo.toml",
    ROOT / "flake.nix",
    ROOT / "justfile",
    ROOT / ".github/workflows/ci.yml",
    ROOT / "README.md",
    ROOT / "AGENTS.md",
    ROOT / "EXPERIMENTS.md",
]


def git_output(*args: str) -> str:
    return subprocess.check_output(
        ["git", *args], cwd=ROOT, text=True, stderr=subprocess.STDOUT
    ).strip()


def package_version() -> str:
    data = tomllib.loads((PID_RS / "Cargo.toml").read_text(encoding="utf-8"))
    return str(data["workspace"]["package"]["version"])


def gitlink_revision() -> str:
    fields = git_output("ls-files", "--stage", "pid-rs").split()
    if len(fields) < 4 or fields[0] != "160000":
        raise RuntimeError("pid-rs is not recorded as a gitlink in the index")
    return fields[1]


def locked_package_version(lock_path: Path, name: str) -> str | None:
    data = tomllib.loads(lock_path.read_text(encoding="utf-8"))
    matches = [
        package
        for package in data.get("package", [])
        if package.get("name") == name and "source" not in package
    ]
    if len(matches) != 1:
        return None
    return str(matches[0]["version"])


def locked_git_packages(lock_path: Path, names: set[str]) -> list[dict[str, object]]:
    data = tomllib.loads(lock_path.read_text(encoding="utf-8"))
    return [
        package
        for package in data.get("package", [])
        if package.get("name") in names
        and str(package.get("source", "")).startswith("git+")
    ]


def exp0_command_problems() -> list[str]:
    problems: list[str] = []
    for path in EXP0_COMMAND_FILES:
        for line_no, line in enumerate(
            path.read_text(encoding="utf-8").splitlines(), 1
        ):
            if "pid-rs/crates/pid-core/Cargo.toml" not in line:
                continue
            if "--bin exp0" not in line and not re.search(r"\s+exp0\s+--", line):
                continue
            if (
                "--features experimental-all" not in line
                and "--all-features" not in line
            ):
                problems.append(
                    f"{path.relative_to(ROOT)}:{line_no}: Exp0 command omits "
                    "`--features experimental-all`"
                )
    return problems


def main() -> int:
    problems: list[str] = []
    if not (PID_RS / "Cargo.toml").is_file():
        print("Repository-truth problems: 1")
        print("- pid-rs submodule is not checked out")
        return 1

    version = package_version()
    revision = gitlink_revision()
    short = revision[:7]

    worktree_revision = git_output("-C", "pid-rs", "rev-parse", "HEAD")
    if worktree_revision != revision:
        problems.append(
            f"pid-rs worktree {worktree_revision} does not match gitlink {revision}"
        )

    required_claims = {
        ROOT / "grandplan.md": (version, revision),
        ROOT / "AGENTS.md": (version, short),
        ROOT / "CHANGELOG.md": (version, short),
    }
    for path, needles in required_claims.items():
        text = path.read_text(encoding="utf-8")
        for needle in needles:
            if needle not in text:
                problems.append(
                    f"{path.relative_to(ROOT)} does not record current pid-rs identity {needle!r}"
                )

    harness = (ROOT / "crates/pid-sim/src/offline_harness.rs").read_text(
        encoding="utf-8"
    )
    expected_stamp = f"pid-core {version} (pid-rs {short})"
    if expected_stamp not in harness:
        problems.append(
            "offline harness estimator revision stamp does not match "
            f"{expected_stamp!r}"
        )

    observer_version = locked_package_version(
        ROOT / "crates/ncp-observer/Cargo.lock", "pid-runlog"
    )
    if observer_version != version:
        problems.append(
            "crates/ncp-observer/Cargo.lock resolves local pid-runlog "
            f"{observer_version!r}, expected {version!r}"
        )

    ncp_packages = locked_git_packages(
        ROOT / "crates/ncp-observer/Cargo.lock", {"ncp-core", "ncp-zenoh"}
    )
    ncp_revisions = {
        str(package["source"]).rsplit("#", 1)[-1] for package in ncp_packages
    }
    if len(ncp_packages) != 2 or len(ncp_revisions) != 1:
        problems.append(
            "ncp-observer must lock ncp-core and ncp-zenoh to one exact revision"
        )
    else:
        ncp_revision = next(iter(ncp_revisions))
        observer_source = (ROOT / "crates/ncp-observer/src/lib.rs").read_text(
            encoding="utf-8"
        )
        if ncp_revision not in observer_source:
            problems.append(
                "ncp-observer canonical configuration does not record its exact locked NCP revision"
            )
        for constant in ("NCP_VERSION", "CONTRACT_HASH"):
            if constant not in observer_source:
                problems.append(
                    f"ncp-observer canonical configuration does not use ncp-core::{constant}"
                )

    notices = (ROOT / "THIRD_PARTY_NOTICES.generated.md").read_text(encoding="utf-8")
    for package in ("pid-core", "pid-runlog"):
        if f"| `{package}` | {version} |" not in notices:
            problems.append(
                f"THIRD_PARTY_NOTICES.generated.md does not record {package} {version}"
            )

    problems.extend(exp0_command_problems())

    for relative in ("justfile", ".github/workflows/ci.yml"):
        text = (ROOT / relative).read_text(encoding="utf-8")
        for required in ("--pid-mode none", "pid_metrics=0", "pid_metric_events=0"):
            if required not in text:
                problems.append(f"{relative} firebreak is missing {required!r}")

    grandplan = (ROOT / "grandplan.md").read_text(encoding="utf-8")
    for stale in (
        "while its submodule remains pinned to `8a5a9d",
        "`pid-rs@70b45f7…` as a **candidate upgrade**",
    ):
        if stale in grandplan:
            problems.append(
                f"grandplan.md retains stale active estimator claim: {stale}"
            )

    if problems:
        print(f"Repository-truth problems: {len(problems)}")
        for problem in problems:
            print(f"- {problem}")
        return 1

    print(
        "OK: repo truth matches pid-rs "
        f"{version} ({revision}); Exp0/firebreak/consumer-lock/notices checks pass."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
