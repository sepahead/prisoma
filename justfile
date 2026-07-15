# Task runner for prisoma (macOS-first; M4 Max).
#
# Install just:
#   cargo install just

default:
    @just --list

build:
    cargo build

test:
    cargo test

fmt:
    cargo fmt

lint:
    cargo clippy --workspace -- -D warnings

# Docset audits (offline). audit_grandplan.py validates the R1-R112 reference ledger.
docs-audit:
    python scripts/audit_ci_pins.py
    python scripts/generate_capability_matrix.py --check
    python scripts/audit_release_requirements.py
    python scripts/audit_release_review.py
    python scripts/audit_candidate_release.py
    python scripts/audit_research_governance.py
    python scripts/audit_grandplan.py
    python scripts/audit_grandplan_claims.py
    python scripts/audit_docset_claims.py
    python scripts/audit_repo_truth.py

# Machine-checked abstract invariants and required countermodels. These prove only
# the stated SMT abstractions; see formal/README.md for the refinement boundary.
formal:
    python scripts/check_formal_models.py

# The reviewed catalog is the source; both the machine-readable resolved matrix and
# the human-readable table are deterministic, content-hash-bound generated outputs.
capability-matrix:
    python scripts/generate_capability_matrix.py --write

capability-matrix-check:
    python scripts/generate_capability_matrix.py --check

# Fail-closed integrity audit for the frozen 0.9 review intake. This validates the
# tracked baseline and imported task graph; it deliberately does not claim that any
# substantive task, file, human review, or scientific gate is complete.
release-review-audit:
    python scripts/audit_release_review.py

# Validate the unpublished, content-bound current candidate. This checks exact source
# coverage and legal progress transitions; it does not promote any open disposition.
release-candidate-audit:
    python scripts/audit_candidate_release.py

# Verify the complete imported task procedures and all 4,800 open lens dispositions.
# The external handoff path is never inferred; pass it explicitly for byte-level regeneration.
release-requirements-audit:
    python scripts/audit_release_requirements.py

release-requirements-check handoff_dir:
    python scripts/generate_release_requirements.py --handoff-dir {{handoff_dir}} --check
    python scripts/audit_release_requirements.py --handoff-dir {{handoff_dir}}

# Honest current-state M0 scaffolds. Passing validates structure and cross-file
# consistency; it does not mean the preregistration or scientific freeze is ready.
research-governance:
    python scripts/audit_research_governance.py

# Dependency firebreak (grandplan.md §8.9.3): prove the minimum path needs neither
# NCP nor PID atoms.
#   (1) the core builds with NCP disabled — `ncp-observer` is workspace-excluded, so a
#       default `--workspace` build never compiles it (no NCP/Zenoh on the critical path);
#   (2) static factual-outcome label baselines (majority + the SAFE-class held-out logistic
#       regression) are emitted independently of PID. This is dependency groundwork only;
#       it does not implement H1 response scoring or prospective H2 landmark prediction.
firebreak:
    cargo build --workspace
    cargo run -p pid-sim --bin pid-offline-harness -- \
      --input crates/pid-sim/fixtures/offline_vlda_fixture.json \
      --summary-json outputs/firebreak_summary.json --runlog outputs/firebreak_runlog.jsonl \
      --pid-mode none
    grep -q '"majority_success_accuracy"' outputs/firebreak_summary.json
    grep -q '"heldout_logreg_vlda_success_accuracy"' outputs/firebreak_summary.json
    grep -q '"pid": "disabled"' outputs/firebreak_summary.json
    grep -q '"requested": 0' outputs/firebreak_summary.json
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/firebreak_runlog.jsonl
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/firebreak_runlog.jsonl | grep -q 'pid_metrics=0'
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/firebreak_runlog.jsonl | grep -q 'pid_metric_events=0'
    @echo "firebreak OK: core builds NCP-disabled; static label baselines emitted without PID atoms"

# Deterministic, offline NCP wire-0.8 fault suite. Published artifacts must
# reconstruct exactly; explicit retry alone may clean writer-reserved crash scratch.
ncp-fault-observatory out="outputs/ncp_fault_observatory":
    cargo run --locked --manifest-path crates/ncp-observer/Cargo.toml --bin ncp-fault-observatory -- --out-dir {{out}}
    cargo run --locked --manifest-path crates/ncp-observer/Cargo.toml --bin ncp-fault-observatory -- --verify {{out}}
    cargo run --locked --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{out}}/observatory-runlog.jsonl

# Experiment 0 gate (Rust-side smoke subset).
# Full Experiment 0 will later be orchestrated via python/experiments/.
exp0:
    cargo test --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all exp0 -- --nocapture

exp0-bin:
    cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0

exp0-runlog path="outputs/exp0_runlog.jsonl" summary="outputs/exp0_summary.json" seeds="1":
    cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0 -- --seeds {{seeds}} --summary-json {{summary}} --runlog {{path}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{path}}

# Exp0 with opt-in uncertainty quantification: subsample-bootstrap CIs +
# single-source permutation null tests at d=10 (the favourable regime). The
# permutation tests must recover the preregistered marginal-informativeness truth
# table (8/8 on healthy data); build --release, this is compute-heavy.
exp0-uncertainty path="outputs/exp0_uncertainty_runlog.jsonl" summary="outputs/exp0_uncertainty_summary.json" boot="200" perm="200":
    cargo run --release --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0 -- --seeds 1 --bootstrap {{boot}} --permutation {{perm}} --summary-json {{summary}} --runlog {{path}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{path}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{path}} | grep -q 'pid_metrics=7'

toy-harness runlog="outputs/toy_vla_runlog.jsonl" summary="outputs/toy_vla_summary.json" episodes="32":
    cargo run -p pid-sim --bin pid-toy-harness -- --episodes {{episodes}} --summary-json {{summary}} --runlog {{runlog}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}

# H1 shared structural/noninterference preflight. These are content-addressed software fixtures,
# not Protocol A/B response estimates and not H1 evidence. Both rejection paths must still write a
# schema-valid failed run log.
h1-preflight valid="crates/pid-sim/fixtures/h1_preflight_valid.json" invalid="crates/pid-sim/fixtures/h1_preflight_invalid.json" parse_invalid="crates/pid-sim/fixtures/h1_preflight_parse_invalid.json":
    cargo run -p pid-sim --bin pid-h1-preflight -- --input {{valid}} --summary-json outputs/h1_preflight_summary.json --runlog outputs/h1_preflight_runlog.jsonl
    grep -q '"passed": true' outputs/h1_preflight_summary.json
    grep -q '"establishes_h1_evidence": false' outputs/h1_preflight_summary.json
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/h1_preflight_runlog.jsonl
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h1_preflight_runlog.jsonl | grep -F 'pid_metrics=0' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h1_preflight_runlog.jsonl | grep -F 'pid_metric_events=0' >/dev/null
    cp outputs/h1_preflight_runlog.jsonl outputs/h1_preflight_runlog.first.jsonl
    cargo run -p pid-sim --bin pid-h1-preflight -- --input {{valid}} --summary-json outputs/h1_preflight_summary.json --runlog outputs/h1_preflight_runlog.jsonl
    cmp -s outputs/h1_preflight_runlog.first.jsonl outputs/h1_preflight_runlog.jsonl
    if cargo run -p pid-sim --bin pid-h1-preflight -- --input {{invalid}} --summary-json outputs/h1_preflight_invalid_summary.json --runlog outputs/h1_preflight_invalid_runlog.jsonl; then echo "expected H1 semantic/artifact preflight failure"; exit 1; fi
    grep -q '"artifact_hash_mismatch"' outputs/h1_preflight_invalid_summary.json
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/h1_preflight_invalid_runlog.jsonl
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h1_preflight_invalid_runlog.jsonl | grep -F 'errors=1' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h1_preflight_invalid_runlog.jsonl | grep -F 'pid_metric_events=0' >/dev/null
    if cargo run -p pid-sim --bin pid-h1-preflight -- --input {{parse_invalid}} --summary-json outputs/h1_preflight_parse_invalid_summary.json --runlog outputs/h1_preflight_parse_invalid_runlog.jsonl; then echo "expected H1 contract parse failure"; exit 1; fi
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/h1_preflight_parse_invalid_runlog.jsonl
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h1_preflight_parse_invalid_runlog.jsonl | grep -F 'errors=1' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h1_preflight_parse_invalid_runlog.jsonl | grep -F 'pid_metric_events=0' >/dev/null

# Deterministic H1 Protocol-A software reference. The checked finite benchmark binds the exact
# passed common-preflight chain, restores independent clone states, reverses treatment order, and
# performs fixed leave-one-outer-fold-out proper scoring. It is synthetic, PID-free, and produces
# no H1 scientific evidence. Readable invalid inputs must still produce schema-valid failed logs.
h1-protocol-a valid="crates/pid-sim/fixtures/h1_protocol_a_valid.json" parse_invalid="crates/pid-sim/fixtures/h1_protocol_a_parse_invalid.json": h1-preflight
    cargo run -p pid-sim --bin pid-h1-protocol-a -- --input {{valid}} --preflight-input crates/pid-sim/fixtures/h1_preflight_valid.json --preflight-summary outputs/h1_preflight_summary.json --preflight-runlog outputs/h1_preflight_runlog.jsonl --summary-json outputs/h1_protocol_a_summary.json --runlog outputs/h1_protocol_a_runlog.jsonl
    grep -q '"passed": true' outputs/h1_protocol_a_summary.json
    grep -q '"synthetic_fixture_only": true' outputs/h1_protocol_a_summary.json
    grep -q '"establishes_h1_evidence": false' outputs/h1_protocol_a_summary.json
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/h1_protocol_a_runlog.jsonl
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h1_protocol_a_runlog.jsonl | grep -F 'pid_metrics=0' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h1_protocol_a_runlog.jsonl | grep -F 'pid_metric_events=0' >/dev/null
    cp outputs/h1_protocol_a_runlog.jsonl outputs/h1_protocol_a_runlog.first.jsonl
    cargo run -p pid-sim --bin pid-h1-protocol-a -- --input {{valid}} --preflight-input crates/pid-sim/fixtures/h1_preflight_valid.json --preflight-summary outputs/h1_preflight_summary.json --preflight-runlog outputs/h1_preflight_runlog.jsonl --summary-json outputs/h1_protocol_a_summary.json --runlog outputs/h1_protocol_a_runlog.jsonl
    cmp -s outputs/h1_protocol_a_runlog.first.jsonl outputs/h1_protocol_a_runlog.jsonl
    if cargo run -p pid-sim --bin pid-h1-protocol-a -- --input {{valid}} --preflight-input crates/pid-sim/fixtures/h1_preflight_valid.json --preflight-summary outputs/h1_preflight_invalid_summary.json --preflight-runlog outputs/h1_preflight_invalid_runlog.jsonl --summary-json outputs/h1_protocol_a_invalid_preflight_summary.json --runlog outputs/h1_protocol_a_invalid_preflight_runlog.jsonl; then echo "expected Protocol-A preflight binding failure"; exit 1; fi
    grep -q '"preflight_summary_not_eligible"' outputs/h1_protocol_a_invalid_preflight_summary.json
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/h1_protocol_a_invalid_preflight_runlog.jsonl
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h1_protocol_a_invalid_preflight_runlog.jsonl | grep -F 'evaluation_metric_events=0' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h1_protocol_a_invalid_preflight_runlog.jsonl | grep -F 'pid_metric_events=0' >/dev/null
    if cargo run -p pid-sim --bin pid-h1-protocol-a -- --input {{parse_invalid}} --preflight-input crates/pid-sim/fixtures/h1_preflight_valid.json --preflight-summary outputs/h1_preflight_summary.json --preflight-runlog outputs/h1_preflight_runlog.jsonl --summary-json outputs/h1_protocol_a_parse_invalid_summary.json --runlog outputs/h1_protocol_a_parse_invalid_runlog.jsonl; then echo "expected Protocol-A contract parse failure"; exit 1; fi
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/h1_protocol_a_parse_invalid_runlog.jsonl
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h1_protocol_a_parse_invalid_runlog.jsonl | grep -F 'evaluation_metric_events=0' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h1_protocol_a_parse_invalid_runlog.jsonl | grep -F 'pid_metric_events=0' >/dev/null

# Deterministic H2 fixed-horizon cumulative-incidence software reference. Four planning
# artifacts are exact-byte frozen separately from outcomes. The checked complete and censored
# fixtures exercise grouped train-only fitting, stratified reverse-KM IPCW, competing events,
# HT Brier scoring, reliability arithmetic, alarm/lead-time accounting, and readable failure.
# This is PID-free synthetic protocol arithmetic and is not prospective capture or H2 evidence.
h2-reference complete="crates/pid-sim/fixtures/h2_reference/dataset_complete.json" censored="crates/pid-sim/fixtures/h2_reference/dataset_censored.json" parse_invalid="crates/pid-sim/fixtures/h2_reference/dataset_parse_invalid.json":
    cargo run -p pid-sim --bin pid-h2-reference -- --dataset {{complete}} --analysis-plan crates/pid-sim/fixtures/h2_reference/analysis_plan.json --event-ontology crates/pid-sim/fixtures/h2_reference/event_ontology.json --feature-contract crates/pid-sim/fixtures/h2_reference/feature_contract.json --split-manifest crates/pid-sim/fixtures/h2_reference/split_manifest.json --summary-json outputs/h2_reference_summary.json --runlog outputs/h2_reference_runlog.jsonl
    grep -q '"passed": true' outputs/h2_reference_summary.json
    grep -q '"synthetic_fixture_only": true' outputs/h2_reference_summary.json
    grep -q '"establishes_h2_evidence": false' outputs/h2_reference_summary.json
    grep -q '"prospective_capture": false' outputs/h2_reference_summary.json
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/h2_reference_runlog.jsonl
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h2_reference_runlog.jsonl | grep -F 'actions=0' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h2_reference_runlog.jsonl | grep -F 'interventions=0' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h2_reference_runlog.jsonl | grep -F 'pid_metric_events=0' >/dev/null
    cp outputs/h2_reference_runlog.jsonl outputs/h2_reference_runlog.first.jsonl
    cargo run -p pid-sim --bin pid-h2-reference -- --dataset {{complete}} --analysis-plan crates/pid-sim/fixtures/h2_reference/analysis_plan.json --event-ontology crates/pid-sim/fixtures/h2_reference/event_ontology.json --feature-contract crates/pid-sim/fixtures/h2_reference/feature_contract.json --split-manifest crates/pid-sim/fixtures/h2_reference/split_manifest.json --summary-json outputs/h2_reference_summary.json --runlog outputs/h2_reference_runlog.jsonl
    cmp -s outputs/h2_reference_runlog.first.jsonl outputs/h2_reference_runlog.jsonl
    cargo run -p pid-sim --bin pid-h2-reference -- --dataset {{censored}} --analysis-plan crates/pid-sim/fixtures/h2_reference/analysis_plan.json --event-ontology crates/pid-sim/fixtures/h2_reference/event_ontology.json --feature-contract crates/pid-sim/fixtures/h2_reference/feature_contract.json --split-manifest crates/pid-sim/fixtures/h2_reference/split_manifest.json --summary-json outputs/h2_reference_censored_summary.json --runlog outputs/h2_reference_censored_runlog.jsonl
    grep -q '"censored_outcomes": 1' outputs/h2_reference_censored_summary.json
    grep -q '"status": "outcome_unobserved_censored"' outputs/h2_reference_censored_summary.json
    grep -q '"ipcw_weight": null' outputs/h2_reference_censored_summary.json
    grep -q '"reason": "alarm_followup_incomplete"' outputs/h2_reference_censored_summary.json
    python -c 'import json; d=json.load(open("outputs/h2_reference_censored_summary.json")); p=[x for f in d["report"]["fold_outcomes"] if f["status"]=="produced" for x in f["score"]["predictions"]]; w=lambda e: next(x["ipcw_weight"] for x in p if x["episode_id"]==e); assert w("episode-4")==w("episode-5")==1.0; assert abs(w("episode-6")-4/3)<1e-12 and abs(w("episode-7")-4/3)<1e-12'
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/h2_reference_censored_runlog.jsonl
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h2_reference_censored_runlog.jsonl | grep -F 'actions=0' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h2_reference_censored_runlog.jsonl | grep -F 'interventions=0' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h2_reference_censored_runlog.jsonl | grep -F 'pid_metric_events=0' >/dev/null
    if cargo run -p pid-sim --bin pid-h2-reference -- --dataset {{parse_invalid}} --analysis-plan crates/pid-sim/fixtures/h2_reference/analysis_plan.json --event-ontology crates/pid-sim/fixtures/h2_reference/event_ontology.json --feature-contract crates/pid-sim/fixtures/h2_reference/feature_contract.json --split-manifest crates/pid-sim/fixtures/h2_reference/split_manifest.json --summary-json outputs/h2_reference_invalid_summary.json --runlog outputs/h2_reference_invalid_runlog.jsonl; then echo "expected H2 contract parse failure"; exit 1; fi
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/h2_reference_invalid_runlog.jsonl
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h2_reference_invalid_runlog.jsonl | grep -F 'evaluation_metric_events=0' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h2_reference_invalid_runlog.jsonl | grep -F 'pid_metric_events=0' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h2_reference_invalid_runlog.jsonl | grep -F 'actions=0' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h2_reference_invalid_runlog.jsonl | grep -F 'interventions=0' >/dev/null
    perl -0pe 's/"censoring_stratum_frozen_at_ns": 0/"censoring_stratum_frozen_at_ns": 11/' {{complete}} > outputs/h2_reference_semantic_invalid.json
    if cargo run -p pid-sim --bin pid-h2-reference -- --dataset outputs/h2_reference_semantic_invalid.json --analysis-plan crates/pid-sim/fixtures/h2_reference/analysis_plan.json --event-ontology crates/pid-sim/fixtures/h2_reference/event_ontology.json --feature-contract crates/pid-sim/fixtures/h2_reference/feature_contract.json --split-manifest crates/pid-sim/fixtures/h2_reference/split_manifest.json --summary-json outputs/h2_reference_semantic_invalid_summary.json --runlog outputs/h2_reference_semantic_invalid_runlog.jsonl; then echo "expected H2 semantic lineage failure"; exit 1; fi
    grep -q '"feature_unavailable_at_landmark"' outputs/h2_reference_semantic_invalid_summary.json
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/h2_reference_semantic_invalid_runlog.jsonl
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h2_reference_semantic_invalid_runlog.jsonl | grep -F 'evaluation_metric_events=0' >/dev/null
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/h2_reference_semantic_invalid_runlog.jsonl | grep -F 'pid_metric_events=0' >/dev/null

# Physics-backed manipulation software smoke: real Rapier3D push-to-goal episode with a
# success label and real Flow_gt. Requires the `rapier` feature.
rapier-harness runlog="outputs/rapier_push_runlog.jsonl" summary="outputs/rapier_push_summary.json" impulse="0.18":
    cargo run -p pid-sim --features rapier --bin pid-rapier-harness -- --runlog {{runlog}} --summary-json {{summary}} --push-impulse {{impulse}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'errors=0'

# Rapier feature build + physics/manipulation tests (heavy dependency compile).
rapier-test:
    cargo test -p pid-sim --features rapier physics::
    cargo test -p pid-sim --features rapier manipulation::

# Exact data-parallel KSG kNN path (rayon-backed; identical results to serial).
parallel-test:
    cargo test --manifest-path pid-rs/crates/pid-core/Cargo.toml --features parallel

# S2/EC1 reference-adapter software smoke: bounded synthetic SAFE bundle + exact
# source/split/rights/file-hash manifest -> (V,L,D,A) contract -> harness.
safe-adapter out="outputs/safe_vlda_v2.json":
    #!/usr/bin/env bash
    set -euo pipefail
    rollouts="$(mktemp -d "${TMPDIR:-/tmp}/prisoma-safe.XXXXXX")"
    trap 'rm -rf "$rollouts"' EXIT
    python -m experiments.safe_adapter synth --out "$rollouts"
    test -s "$rollouts/safe_bundle_manifest.json"
    python -m experiments.safe_adapter convert --rollouts "$rollouts" --out {{out}} --seen-tasks 0,1 --overwrite
    python -c 'import json; d=json.load(open("{{out}}", encoding="utf-8")); assert d["samples"] and all(s["metadata"].get("bundle_manifest_sha256") for s in d["samples"])'
    python -m experiments.safe_adapter verify --input {{out}}
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{out}} --summary-json outputs/safe_vlda_summary.json --runlog outputs/safe_vlda_runlog.jsonl --require-heldout-split --require-heldout-class-coverage --require-heldout-episode-disjoint --require-axis-provenance-honest
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/safe_vlda_runlog.jsonl

# H4/exploratory attribution companion: faithfulness check -> attribution_logged run log.
attribution-probe runlog="outputs/attribution_runlog.jsonl" artifacts="outputs/attribution":
    python -m experiments.attribution demo --runlog {{runlog}} --artifacts {{artifacts}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'attributions=2'

# Python experiment tests (SAFE adapter + attribution probe; numpy only).
experiments-test:
    python -m pytest tests/python/test_safe_adapter.py tests/python/test_attribution.py -q

# Regenerate the direct-dependency third-party notices (Rust + Python).
notices:
    python scripts/generate_third_party_notices.py --write

notices-check:
    python scripts/generate_third_party_notices.py --check

offline-harness input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_runlog.jsonl" summary="outputs/offline_vlda_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'pid_metrics=4'
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'pid_metric_events=4'
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'geometry_metrics=20'
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'geometry_metric_events=20'
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'evaluation_metrics=142'
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'evaluation_metric_events=223'

offline-harness-require-labels input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_labeled_runlog.jsonl" summary="outputs/offline_vlda_labeled_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --require-success-labels
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}

# Opt-in PID-screen uncertainty: subsample-bootstrap CIs + single-source permutation
# p-values on the continuous (V,L)/(V,D)/(L,D)->A atoms, written to a dedicated file
# (the canonical runlog/summary counts are untouched). The default counts assert here
# to prove that invariant.
offline-harness-uncertainty input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_unc_runlog.jsonl" summary="outputs/offline_vlda_unc_summary.json" unc="outputs/offline_vlda_uncertainty.json" boot="200" perm="200":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --bootstrap {{boot}} --permutation {{perm}} --uncertainty-json {{unc}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'evaluation_metrics=142'
    test -s {{unc}}

offline-harness-require-heldout input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_heldout_runlog.jsonl" summary="outputs/offline_vlda_heldout_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --require-heldout-split
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'errors=0'

offline-harness-require-heldout-class-coverage input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_heldout_class_coverage_runlog.jsonl" summary="outputs/offline_vlda_heldout_class_coverage_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --require-heldout-class-coverage
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'errors=0'

offline-harness-require-heldout-episode-disjoint input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_heldout_episode_disjoint_runlog.jsonl" summary="outputs/offline_vlda_heldout_episode_disjoint_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --require-heldout-episode-disjoint
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'errors=0'

offline-harness-strict input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_strict_runlog.jsonl" summary="outputs/offline_vlda_strict_summary.json":
    if cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --require-geometry-pass; then echo "expected strict offline geometry gate failure"; exit 1; fi
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{runlog}} | grep -q 'errors=1'

offline-harness-highdim input="crates/pid-sim/fixtures/offline_vlda_highdim_fixture.json" runlog="outputs/offline_vlda_highdim_runlog.jsonl" summary="outputs/offline_vlda_highdim_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}

# Positive-path continuous fixture: every axis DECLARED absolutely continuous, equal ambient source
# dimensions (continuous shared exclusions requires them), tie-free. All 6 requested estimates are
# produced — the counterpart to `offline-harness`, whose binary-L fixture abstains.
offline-harness-continuous input="crates/pid-sim/fixtures/offline_vlda_continuous_fixture.json" runlog="outputs/offline_vlda_continuous_runlog.jsonl" summary="outputs/offline_vlda_continuous_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}

# Exercise both report outcomes. The mixed-support fixture must abstain without numeric
# placeholders; its all-continuous counterpart must produce every requested estimate.
estimate-report-contract: offline-harness offline-harness-continuous
    python -c 'import json; a=json.load(open("outputs/offline_vlda_summary.json", encoding="utf-8")); b=json.load(open("outputs/offline_vlda_continuous_summary.json", encoding="utf-8")); collect=lambda d: [d["metrics"]["mi_v_action"], d["metrics"]["mi_l_action"], d["metrics"]["mi_d_action"], *d["metrics"]["pid_pairs"].values()]; ao=collect(a); bo=collect(b); numeric={"value", "co_information", "mi_joint_action", "mi_source_1_action", "mi_source_2_action", "redundancy", "synergy", "unique_source_1", "unique_source_2"}; assert len(ao)==6 and sum(x["status"]=="abstained" for x in ao)==4 and sum(x["status"]=="produced" for x in ao)==2; assert all(numeric.isdisjoint(x) for x in ao if x["status"]=="abstained"); assert len(bo)==6 and all(x["status"]=="produced" for x in bo); assert a["metrics"]["estimate_denominators"]["abstained"]==4 and b["metrics"]["estimate_denominators"]["abstained"]==0'
    python -c 'import json, math; expected={"offline_vlda.pid.mi_v_action", "offline_vlda.pid.mi_d_action", "offline_vlda.pid.train_split.mi_v_action", "offline_vlda.pid.train_split.mi_d_action"}; events=[event for line in open("outputs/offline_vlda_runlog.jsonl", encoding="utf-8") if (event:=json.loads(line)).get("type")=="pid_metric"]; assert len(events)==len(expected)==4 and {event["name"] for event in events}==expected; assert all(event["metadata"].get("computation_status")=="produced" and math.isfinite(event["value"]) for event in events)'

# Discrete (quantized I_min) PID mode; results carry saturation diagnostics (grandplan §8.1.6).
offline-harness-discrete input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_discrete_runlog.jsonl" summary="outputs/offline_vlda_discrete_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --pid-mode discrete --discrete-bins 8
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}

# PLS-project sources toward A, then discrete PID (high-dim escape hatch).
offline-harness-discrete-pls input="crates/pid-sim/fixtures/offline_vlda_highdim_fixture.json" runlog="outputs/offline_vlda_highdim_dpls_runlog.jsonl" summary="outputs/offline_vlda_highdim_dpls_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --pid-mode discrete-pls --pls-components 2 --discrete-bins 8
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{runlog}}

# M1 run-log smoke path.
runlog-demo:
    cargo run -p pid-sim --bin pid-sim-demo -- outputs/demo_runlog.jsonl

runlog-bridge-demo:
    cargo run -p pid-sim --bin pid-sim-bridge-demo -- outputs/demo_bridge_runlog.jsonl

runlog-bridge-stdio path="outputs/demo_bridge_stdio_runlog.jsonl":
    printf '%s\n' '{"jsonrpc":"2.0","id":"status","method":"sim.status","params":{}}' '{"jsonrpc":"2.0","id":"intervention","method":"intervention.apply","params":{"intervention_type":"set_velocity","payload":{"object_id":"red_cube","velocity":[0.2,0.0,0.0]}}}' '{"jsonrpc":"2.0","id":"step","method":"sim.step","params":{"dt":0.1}}' '{"jsonrpc":"2.0","id":"stop","method":"log.stop","params":{}}' | cargo run -p pid-sim --bin pid-sim-bridge-stdio -- {{path}}
    cargo run -p pid-sim --bin pid-sim-verify -- {{path}}

runlog-bridge-stdio-safe path="outputs/demo_bridge_stdio_safe_runlog.jsonl":
    cargo run -p pid-sim --bin pid-sim-bridge-demo -- outputs/demo_bridge_runlog.jsonl
    printf '%s\n' '{"jsonrpc":"2.0","id":"status","method":"sim.status","params":{}}' '{"jsonrpc":"2.0","id":"replay","method":"log.replay","params":{"run_log_uri":"demo_bridge_runlog.jsonl"}}' '{"jsonrpc":"2.0","id":"step","method":"sim.step","params":{"dt":0.1}}' | cargo run -p pid-sim --bin pid-sim-bridge-stdio -- --safe-mode {{path}}

runlog-bridge-tcp path="outputs/demo_bridge_tcp_runlog.jsonl" addr="127.0.0.1:38472":
    cargo run -p pid-sim --bin pid-sim-bridge-tcp -- --bind {{addr}} {{path}}

runlog-bridge-ws path="outputs/demo_bridge_ws_runlog.jsonl" addr="127.0.0.1:38473":
    cargo run -p pid-sim --bin pid-sim-bridge-ws -- --bind {{addr}} {{path}}

bridge-contract out="outputs/bridge_runlog_contract.json":
    cargo run -p pid-bridge --bin pid-bridge-contract -- --out {{out}}

# Offline/local unit proof only: bind/safe defaults, the enumerated JSON-RPC/WebSocket
# checks and per-message/per-operation caps, plus non-adversarial canonical/no-replace
# file behavior. Not remote-security, forwarding/proxy, or adversarial-filesystem validation.
bridge-security:
    cargo test -p pid-bridge
    cargo test -p pid-rerun
    cargo test -p pid-sim --bin pid-sim-bridge-tcp
    cargo test -p pid-sim --bin pid-sim-bridge-ws
    cargo test -p pid-sim --lib

runlog-replay path="outputs/demo_runlog.jsonl":
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- {{path}}

runlog-validate path="outputs/demo_runlog.jsonl":
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{path}}

runlog-summary path="outputs/demo_runlog.jsonl" out="outputs/demo_runlog_summary.json":
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --summary-json {{path}} {{out}}

runlog-manifest path="outputs/demo_runlog.jsonl" out="outputs/demo_runlog_manifest.json":
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --manifest-json {{path}} {{out}}

runlog-sidecars path="outputs/demo_runlog.jsonl":
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --write-sidecars {{path}}
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --verify-sidecars {{path}}

runlog-sidecars-proof: runlog-demo
    just runlog-validate
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/demo_runlog.jsonl | grep -F 'logical_trace_hash_v3=ac219c93ce9c56eaba8df44c1ae77fff2717432f2ae92973bbb5124d05352574' >/dev/null
    just runlog-sidecars

runlog-sim-verify path="outputs/demo_bridge_runlog.jsonl":
    cargo run -p pid-sim --bin pid-sim-verify -- {{path}}

runlog-rerun path="outputs/demo_runlog.jsonl" out="outputs/demo_runlog.rrd":
    cargo run -p pid-rerun --bin runlog-to-rerun -- {{path}} --save {{out}}

runlog-rerun-proof: runlog-demo
    just runlog-validate
    # Use a private new destination so the proof is repeatable without deleting or replacing
    # an operator's prior recording; the converter itself requires no-clobber output.
    proof_dir="$(mktemp -d "${TMPDIR:-/tmp}/prisoma-rerun-proof.XXXXXX")"; trap 'rm -rf "$proof_dir"' EXIT; cargo run -p pid-rerun --bin runlog-to-rerun -- outputs/demo_runlog.jsonl --save "$proof_dir/demo.rrd"; test -s "$proof_dir/demo.rrd"; test "$(dd if="$proof_dir/demo.rrd" bs=4 count=1 2>/dev/null)" = RRF2

runlog-rerun-bridge path="outputs/demo_bridge_runlog.jsonl" out="outputs/demo_bridge_runlog.rrd":
    cargo run -p pid-rerun --bin runlog-to-rerun -- {{path}} --save {{out}}

runlog-bridge-export-rerun source="outputs/demo_bridge_runlog.jsonl" path="outputs/demo_bridge_export_rerun_runlog.jsonl" out="outputs/demo_bridge_export_rerun.rrd":
    cargo run -p pid-sim --bin pid-sim-bridge-demo -- {{source}}
    python -c 'import json, os, sys; print(json.dumps({"jsonrpc":"2.0","id":"export","method":"export.rerun","params":{"run_log_uri":os.path.realpath(sys.argv[1]),"output_uri":os.path.realpath(sys.argv[2])}}, separators=(",", ":")))' "{{source}}" "{{out}}" | cargo run -p pid-sim --bin pid-sim-bridge-stdio -- "{{path}}"
    cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate {{path}}
