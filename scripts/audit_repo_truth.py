#!/usr/bin/env python3
"""Fail closed on repository-truth drift missed by prose heuristics.

This audit binds active commands and claims to checked-out dependencies and living protocol
ledgers rather than remembered version strings. It covers the pid-rs 1.0 migration, excluded NCP
consumer lock, firebreak wording, generated notices, current claim boundaries, and the dated
ecosystem-overlay reconciliation. Network refresh remains deliberate and manual; normal CI proves
that the reviewed offline state and active prose agree.
"""

from __future__ import annotations

import csv
import hashlib
import json
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
        if "H1/H2 baseline predictors" in text:
            problems.append(
                f"{relative} overstates the static label-baseline firebreak as H1/H2 execution"
            )
        for required in (
            "pid-h1-preflight",
            "h1_preflight_valid.json",
            "h1_preflight_invalid.json",
            "h1_preflight_parse_invalid.json",
            "establishes_h1_evidence",
        ):
            if required not in text:
                problems.append(f"{relative} H1 preflight smoke is missing {required!r}")

    if not (ROOT / "crates/pid-sim/src/bin/h1_preflight.rs").is_file():
        problems.append("pid-h1-preflight binary is missing")
    for fixture in (
        "h1_preflight_valid.json",
        "h1_preflight_invalid.json",
        "h1_preflight_parse_invalid.json",
    ):
        if not (ROOT / "crates/pid-sim/fixtures" / fixture).is_file():
            problems.append(f"H1 preflight fixture is missing: {fixture}")

    power_doc = (ROOT / "docs/power-gate/README.md").read_text(encoding="utf-8")
    if "Historical idealized grid outputs — withdrawn; not capture requirements" not in power_doc:
        problems.append("docs/power-gate/README.md does not withdraw retired capture counts")
    if "## Capture-scale requirements" in power_doc:
        problems.append("docs/power-gate/README.md revives retired capture requirements")
    power_artifact = json.loads(
        (ROOT / "docs/power-gate/power-gate-2026-07-10.json").read_text(
            encoding="utf-8"
        )
    )
    power_verdicts = {
        verdict["endpoint_id"]: (
            verdict["smallest_passing_grid_n"],
            verdict["null_rate_at_smallest_passing_grid_n"],
            verdict["passed"],
        )
        for verdict in power_artifact["verdicts"]
    }
    expected_power_verdicts = {
        "h1": (None, None, False),
        "h2": (64, 0.05, True),
        "h3": (40, 0.0675, True),
        "h4": (96, 0.0675, True),
    }
    if power_verdicts != expected_power_verdicts:
        problems.append("power-gate artifact verdicts changed; reconcile active interpretation")
    for required in (
        "artifact verdict `passed=false`",
        "Smallest same-n passing grid point: 64",
        "Smallest same-n passing grid point: 40",
        "Smallest same-n passing grid point: 96",
    ):
        if required not in power_doc:
            problems.append(
                f"docs/power-gate/README.md omits artifact-backed verdict {required!r}"
            )

    splat_spec = (ROOT / "pidsplatspecs.md").read_text(encoding="utf-8")
    if "Schema 2; partial M2/EC1 groundwork" not in splat_spec:
        problems.append("pidsplatspecs.md does not record the current run-log schema/M2 status")
    if "This is partial M4" in splat_spec:
        problems.append("pidsplatspecs.md mislabels run-log groundwork as M4")
    if "pid-h1-preflight" not in splat_spec:
        problems.append("pidsplatspecs.md omits the implemented H1 common preflight")
    for relative in ("pidsplatspecs.md", "ARCHITECTURE.md"):
        text = (ROOT / relative).read_text(encoding="utf-8")
        if re.search(r"partial\s+M4\s+groundwork", text, flags=re.IGNORECASE):
            problems.append(f"{relative} mislabels viewer/run-log groundwork as M4")

    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    for required in (
        "H1 did not reach a passing grid point",
        "H2 first passed\n  at 64 tasks",
        "H3 first passed at 40 matched cases",
        "H4 first passed at 96 tasks",
    ):
        if required not in changelog:
            problems.append(f"CHANGELOG.md omits corrected power verdict {required!r}")
    for stale in (
        "using the *preregistered\n  procedures themselves*",
        "H3 powered at 30 matched cases",
        "H2/H4 at 96 tasks",
    ):
        if stale in changelog:
            problems.append(f"CHANGELOG.md retains stale power claim {stale!r}")

    grandplan = (ROOT / "grandplan.md").read_text(encoding="utf-8")
    for false_engram_claim in (
        "a6f2c5f973783373db9d90769d981b1d549a5b6b",
        "contains an implemented neurocontrol/NCP reference stack",
    ):
        if false_engram_claim in grandplan:
            problems.append(
                "grandplan.md attributes unreachable implementation evidence to the public "
                f"sepahead/engram repository: {false_engram_claim!r}"
            )
    for required in (
        "a4ce6ab9897dd3f1265b4cacc53f0afc349087cd",
        "README-only placeholder",
    ):
        if required not in grandplan:
            problems.append(
                f"grandplan.md omits the verified public Engram boundary {required!r}"
            )
    if "current public main `2a55f3d" in grandplan:
        problems.append("grandplan.md mislabels a reviewed Haldir revision as current main")
    for project, dated_revision in {
        "galadriel": "ff272dc814080c6766a53c872ca4d0e24bcd5132",
        "crebain": "09dd5ec1556bd56e6934e1ef019f95de84cf9b4f",
        "manwe": "0f6505bb5dadf5d70359b5ea6d545216342bd30a",
        "haldir": "fbf5a5308da1c6a82eebe1afb56635bf0d6fd798",
        "cortexel": "16f2da71a5beb863235a90e552e6772639638be3",
        "melkor": "21c8fb53f58e19a78d92a4b01ce479374a7b8633",
    }.items():
        if dated_revision not in grandplan:
            problems.append(
                "grandplan.md omits the dated currentness revision for "
                f"{project}: {dated_revision}"
            )

    ecosystem_ledger_path = ROOT / "protocols/ecosystem_evidence_current_v1.json"
    if not ecosystem_ledger_path.is_file():
        problems.append("current ecosystem evidence overlay is missing")
    else:
        ecosystem_ledger = json.loads(ecosystem_ledger_path.read_text(encoding="utf-8"))
        baseline = ecosystem_ledger.get("baseline", {})
        baseline_path = ROOT / str(baseline.get("path", ""))
        if not baseline_path.is_file():
            problems.append("ecosystem evidence overlay baseline is missing")
        else:
            baseline_bytes = baseline_path.read_bytes()
            baseline_sha256 = hashlib.sha256(baseline_bytes).hexdigest()
            if baseline_sha256 != baseline.get("sha256"):
                problems.append("ecosystem evidence overlay baseline hash does not match")
            with baseline_path.open(encoding="utf-8", newline="") as handle:
                baseline_rows = sum(1 for _ in csv.DictReader(handle))
            if baseline_rows != baseline.get("row_count"):
                problems.append("ecosystem evidence overlay baseline row count does not match")

        overrides = {
            entry.get("project"): entry
            for entry in ecosystem_ledger.get("overrides", [])
            if isinstance(entry, dict)
        }
        required_overrides = {
            "pid-rs",
            "NCP",
            "galadriel",
            "crebain",
            "manwe",
            "engram",
            "haldir",
            "cortexel",
            "melkor",
        }
        missing_overrides = sorted(required_overrides - overrides.keys())
        if missing_overrides:
            problems.append(
                f"ecosystem evidence overlay omits current edges: {missing_overrides}"
            )
        expected_revisions = {
            "pid-rs": "ac4a7803c5a77408f5e9176c60cda71c65c38260",
            "NCP": "v0.8.0",
            "galadriel": "ff272dc814080c6766a53c872ca4d0e24bcd5132",
            "crebain": "09dd5ec1556bd56e6934e1ef019f95de84cf9b4f",
            "manwe": "0f6505bb5dadf5d70359b5ea6d545216342bd30a",
            "engram": "a4ce6ab9897dd3f1265b4cacc53f0afc349087cd",
            "haldir": "fbf5a5308da1c6a82eebe1afb56635bf0d6fd798",
            "cortexel": "16f2da71a5beb863235a90e552e6772639638be3",
            "melkor": "21c8fb53f58e19a78d92a4b01ce479374a7b8633",
        }
        for project, expected_revision in expected_revisions.items():
            if overrides.get(project, {}).get("observed_revision") != expected_revision:
                problems.append(
                    "ecosystem evidence overlay has an unreconciled reviewed revision for "
                    f"{project}"
                )
        if overrides.get("NCP", {}).get("resolved_revision") != (
            "2f5bd586d4bb20c90362bb6f5698b7f64057ba4e"
        ):
            problems.append("ecosystem evidence overlay omits the peeled NCP v0.8.0 commit")
        if overrides.get("crebain", {}).get("prior_frame_capability_revision") != (
            "49d7b3614f24d21a40fe2af6dbeac082338ae9d7"
        ):
            problems.append(
                "ecosystem evidence overlay omits Crebain's prior frame-capability revision"
            )
        boundary_requirements = {
            "galadriel": (
                "optional evidence-sender component",
                "deployed receiver-verified Crebain-to-Galadriel path",
                "no direct Prisoma adapter",
            ),
            "crebain": (
                "standard release omits ncp",
                "no release candidate or exact-head release evidence is sealed",
                "local put is not receiver receipt",
                "not called by admission",
                "action/control commands remain unregistered",
                "no direct Prisoma adapter",
            ),
            "manwe": ("no drop-in Prisoma adapter",),
            "haldir": (
                "declaration is cooperative, process-local",
                "not durably bound",
                "test-only seams",
                "no live Zenoh session",
                "no runnable service",
                "direct Prisoma route",
            ),
            "cortexel": ("lacks headless export", "does not supersede"),
            "melkor": ("no calibrated", "Prisoma adapter"),
        }
        for project, required_phrases in boundary_requirements.items():
            boundary = str(overrides.get(project, {}).get("current_boundary", ""))
            for required_phrase in required_phrases:
                if required_phrase not in boundary:
                    problems.append(
                        f"ecosystem evidence overlay overstates {project}; "
                        f"missing boundary {required_phrase!r}"
                    )
        engram_override = overrides.get("engram", {})
        if engram_override.get("observed_revision") != (
            "a4ce6ab9897dd3f1265b4cacc53f0afc349087cd"
        ) or "README-only" not in str(engram_override.get("current_boundary", "")):
            problems.append("ecosystem evidence overlay revives unsupported Engram maturity")

    claim_registry_path = ROOT / "protocols/research_claim_registry_v1.json"
    if not claim_registry_path.is_file():
        problems.append("current research claim registry is missing")
    else:
        claim_registry = json.loads(claim_registry_path.read_text(encoding="utf-8"))
        claims = claim_registry.get("claims", [])
        claim_ids = [claim.get("claim_id") for claim in claims if isinstance(claim, dict)]
        if claim_ids != ["EC1", "H1", "H2", "H3", "H4"]:
            problems.append("research claim registry must contain EC1 and H1-H4 exactly once in order")
        claims_by_id = {
            claim.get("claim_id"): claim for claim in claims if isinstance(claim, dict)
        }
        h1 = claims_by_id.get("H1", {})
        if h1.get("execution_status") != (
            "deterministic_protocol_a_software_reference_fixture_runnable_"
            "protocol_b_unimplemented"
        ):
            problems.append("research claim registry misstates the H1 execution boundary")
        h1_artifacts = {
            artifact.get("path")
            for artifact in h1.get("current_artifacts", [])
            if isinstance(artifact, dict)
        }
        required_h1_artifacts = {
            "crates/pid-sim/src/h1_preflight.rs",
            "crates/pid-sim/src/bin/h1_preflight.rs",
            "crates/pid-sim/fixtures/h1_preflight_valid.json",
            "crates/pid-sim/src/h1_protocol_a.rs",
            "crates/pid-sim/src/bin/h1_protocol_a.rs",
            "crates/pid-sim/fixtures/h1_protocol_a_valid.json",
        }
        if not required_h1_artifacts.issubset(h1_artifacts):
            problems.append("research claim registry omits implemented H1 software artifacts")
        for artifact in required_h1_artifacts:
            if not (ROOT / artifact).is_file():
                problems.append(f"research claim registry names missing H1 artifact: {artifact}")
        if "just h1-protocol-a" not in h1.get("proof_commands", []):
            problems.append("research claim registry omits the H1 Protocol-A proof command")
        h2 = claims_by_id.get("H2", {})
        if h2.get("execution_status") != (
            "deterministic_synthetic_fixed_horizon_software_reference_fixture_"
            "runnable_real_execution_unimplemented"
        ):
            problems.append("research claim registry misstates the H2 execution boundary")
        h2_artifacts = {
            artifact.get("path")
            for artifact in h2.get("current_artifacts", [])
            if isinstance(artifact, dict)
        }
        required_h2_files = {
            "crates/pid-sim/src/h2_reference.rs",
            "crates/pid-sim/src/bin/h2_reference.rs",
            "crates/pid-sim/fixtures/h2_reference/analysis_plan.json",
            "crates/pid-sim/fixtures/h2_reference/dataset_complete.json",
            "crates/pid-sim/fixtures/h2_reference/dataset_censored.json",
        }
        if not {
            "crates/pid-sim/src/h2_reference.rs",
            "crates/pid-sim/src/bin/h2_reference.rs",
            "crates/pid-sim/fixtures/h2_reference",
        }.issubset(h2_artifacts):
            problems.append("research claim registry omits implemented H2 software artifacts")
        for artifact in required_h2_files:
            if not (ROOT / artifact).is_file():
                problems.append(f"research claim registry names missing H2 artifact: {artifact}")
        if "just h2-reference" not in h2.get("proof_commands", []):
            problems.append("research claim registry omits the H2 reference proof command")
        h2_boundary_text = " ".join(
            str(h2.get(field, ""))
            for field in ("permitted_language", "prohibited_language")
        )
        for required in (
            "synthetic",
            "prospective",
            "H2 passed",
            "deployment validity",
        ):
            if required not in h2_boundary_text:
                problems.append(
                    f"research claim registry H2 boundary omits {required!r}"
                )
        if claims_by_id.get("H3", {}).get("execution_status") != "not_eligible":
            problems.append("research claim registry overstates H3 eligibility")

    for relative in ("justfile", ".github/workflows/ci.yml"):
        text = (ROOT / relative).read_text(encoding="utf-8")
        for required in (
            "pid-h1-protocol-a",
            "h1_protocol_a_valid.json",
            "h1_protocol_a_parse_invalid.json",
            "synthetic_fixture_only",
            "establishes_h1_evidence",
            "evaluation_metric_events=0",
            "pid_metric_events=0",
        ):
            if required not in text:
                problems.append(
                    f"{relative} H1 Protocol-A smoke is missing {required!r}"
                )
        for required in (
            "pid-h2-reference",
            "dataset_complete.json",
            "dataset_censored.json",
            "establishes_h2_evidence",
            "prospective_capture",
            "pid_metric_events=0",
        ):
            if required not in text:
                problems.append(f"{relative} H2 reference smoke is missing {required!r}")
    if "H1/H2 baselines execute with PID disabled" in grandplan:
        problems.append("grandplan.md overstates the static label-baseline firebreak")
    for relative in ("grandplan.md", "RESEARCH_VLA_D_NCP.md"):
        text = (ROOT / relative).read_text(encoding="utf-8")
        if "PID-disabled H1/H2 path" in text:
            problems.append(f"{relative} overstates the dependency firebreak as H1/H2 execution")
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
