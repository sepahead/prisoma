# Task runner for pid_vla (macOS-first; M4 Max).
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

# Docset audits (offline; uses outputs/arxiv_ref_cache.json).
docs-audit:
    python scripts/audit_grandplan.py --check-italic-titles
    python scripts/audit_grandplan_claims.py
    python scripts/audit_docset_claims.py

# Experiment 0 gate (Rust-side smoke subset).
# Full Experiment 0 will later be orchestrated via python/experiments/.
exp0:
    cargo test -p pid-core exp0 -- --nocapture

exp0-bin:
    cargo run -p pid-core --bin exp0

exp0-runlog path="outputs/exp0_runlog.jsonl" summary="outputs/exp0_summary.json" seeds="1":
    cargo run -p pid-core --bin exp0 -- --seeds {{seeds}} --summary-json {{summary}} --runlog {{path}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{path}}

# Exp0 with opt-in uncertainty quantification: subsample-bootstrap CIs +
# single-source permutation null tests at d=10 (the favourable regime). The
# permutation tests must recover the preregistered marginal-informativeness truth
# table (8/8 on healthy data); build --release, this is compute-heavy.
exp0-uncertainty path="outputs/exp0_uncertainty_runlog.jsonl" summary="outputs/exp0_uncertainty_summary.json" boot="200" perm="200":
    cargo run --release -p pid-core --bin exp0 -- --seeds 1 --bootstrap {{boot}} --permutation {{perm}} --summary-json {{summary}} --runlog {{path}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{path}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{path}} | grep -q 'pid_metrics=8'

toy-harness runlog="outputs/toy_vla_runlog.jsonl" summary="outputs/toy_vla_summary.json" episodes="32":
    cargo run -p pid-sim --bin pid-toy-harness -- --episodes {{episodes}} --summary-json {{summary}} --runlog {{runlog}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}

# M3 physics-backed manipulation: real Rapier3D push-to-goal episode with a
# success label and real Flow_gt. Requires the `rapier` feature.
rapier-harness runlog="outputs/rapier_push_runlog.jsonl" summary="outputs/rapier_push_summary.json" impulse="0.18":
    cargo run -p pid-sim --features rapier --bin pid-rapier-harness -- --runlog {{runlog}} --summary-json {{summary}} --push-impulse {{impulse}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{runlog}} | grep -q 'errors=0'

# Rapier feature build + physics/manipulation tests (heavy dependency compile).
rapier-test:
    cargo test -p pid-sim --features rapier physics:: manipulation::

# M5 capture: SAFE rollouts -> (V,L,D,A) contract, then the real harness with gates.
safe-adapter out="outputs/safe_vlda.json" rollouts="outputs/safe_synth":
    python -m experiments.safe_adapter synth --out {{rollouts}}
    python -m experiments.safe_adapter convert --rollouts {{rollouts}} --out {{out}} --seen-tasks 0,1
    python -m experiments.safe_adapter verify --input {{out}}
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{out}} --summary-json outputs/safe_vlda_summary.json --runlog outputs/safe_vlda_runlog.jsonl --require-heldout-split --require-heldout-class-coverage --require-heldout-episode-disjoint
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate outputs/safe_vlda_runlog.jsonl

# H9 attribution probe: faithfulness-checked attribution -> attribution_logged run log.
attribution-probe runlog="outputs/attribution_runlog.jsonl" artifacts="outputs/attribution":
    python -m experiments.attribution demo --runlog {{runlog}} --artifacts {{artifacts}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{runlog}} | grep -q 'attributions=2'

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
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{runlog}} | grep -q 'pid_metrics=42'
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{runlog}} | grep -q 'pid_metric_events=42'
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{runlog}} | grep -q 'evaluation_metrics=142'
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{runlog}} | grep -q 'evaluation_metric_events=223'

offline-harness-require-labels input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_labeled_runlog.jsonl" summary="outputs/offline_vlda_labeled_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --require-success-labels
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}

# Opt-in PID-screen uncertainty: subsample-bootstrap CIs + single-source permutation
# p-values on the continuous (V,L)/(V,D)/(L,D)->A atoms, written to a dedicated file
# (the canonical runlog/summary counts are untouched). The default counts assert here
# to prove that invariant.
offline-harness-uncertainty input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_unc_runlog.jsonl" summary="outputs/offline_vlda_unc_summary.json" unc="outputs/offline_vlda_uncertainty.json" boot="200" perm="200":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --bootstrap {{boot}} --permutation {{perm}} --uncertainty-json {{unc}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{runlog}} | grep -q 'evaluation_metrics=142'
    test -s {{unc}}

offline-harness-require-heldout input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_heldout_runlog.jsonl" summary="outputs/offline_vlda_heldout_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --require-heldout-split
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{runlog}} | grep -q 'errors=0'

offline-harness-require-heldout-class-coverage input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_heldout_class_coverage_runlog.jsonl" summary="outputs/offline_vlda_heldout_class_coverage_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --require-heldout-class-coverage
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{runlog}} | grep -q 'errors=0'

offline-harness-require-heldout-episode-disjoint input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_heldout_episode_disjoint_runlog.jsonl" summary="outputs/offline_vlda_heldout_episode_disjoint_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --require-heldout-episode-disjoint
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{runlog}} | grep -q 'errors=0'

offline-harness-strict input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_strict_runlog.jsonl" summary="outputs/offline_vlda_strict_summary.json":
    if cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --require-geometry-pass; then echo "expected strict offline geometry gate failure"; exit 1; fi
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{runlog}} | grep -q 'errors=1'

offline-harness-highdim input="crates/pid-sim/fixtures/offline_vlda_highdim_fixture.json" runlog="outputs/offline_vlda_highdim_runlog.jsonl" summary="outputs/offline_vlda_highdim_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}

# Discrete (quantized I_min) PID mode; results carry saturation diagnostics (grandplan §8.1.6).
offline-harness-discrete input="crates/pid-sim/fixtures/offline_vlda_fixture.json" runlog="outputs/offline_vlda_discrete_runlog.jsonl" summary="outputs/offline_vlda_discrete_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --pid-mode discrete --discrete-bins 8
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}

# PLS-project sources toward A, then discrete PID (high-dim escape hatch).
offline-harness-discrete-pls input="crates/pid-sim/fixtures/offline_vlda_highdim_fixture.json" runlog="outputs/offline_vlda_highdim_dpls_runlog.jsonl" summary="outputs/offline_vlda_highdim_dpls_summary.json":
    cargo run -p pid-sim --bin pid-offline-harness -- --input {{input}} --summary-json {{summary}} --runlog {{runlog}} --pid-mode discrete-pls --pls-components 2 --discrete-bins 8
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}

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
    printf '%s\n' '{"jsonrpc":"2.0","id":"status","method":"sim.status","params":{}}' '{"jsonrpc":"2.0","id":"replay","method":"log.replay","params":{"run_log_uri":"outputs/demo_bridge_runlog.jsonl"}}' '{"jsonrpc":"2.0","id":"step","method":"sim.step","params":{"dt":0.1}}' | cargo run -p pid-sim --bin pid-sim-bridge-stdio -- --safe-mode {{path}}

runlog-bridge-tcp path="outputs/demo_bridge_tcp_runlog.jsonl" addr="127.0.0.1:38472":
    cargo run -p pid-sim --bin pid-sim-bridge-tcp -- --bind {{addr}} {{path}}

runlog-bridge-ws path="outputs/demo_bridge_ws_runlog.jsonl" addr="127.0.0.1:38473":
    cargo run -p pid-sim --bin pid-sim-bridge-ws -- --bind {{addr}} {{path}}

bridge-contract out="outputs/bridge_runlog_contract.json":
    cargo run -p pid-bridge --bin pid-bridge-contract -- --out {{out}}

runlog-replay path="outputs/demo_runlog.jsonl":
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{path}}

runlog-validate path="outputs/demo_runlog.jsonl":
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{path}}

runlog-summary path="outputs/demo_runlog.jsonl" out="outputs/demo_runlog_summary.json":
    cargo run -p pid-runlog --bin pid-runlog-replay -- --summary-json {{path}} {{out}}

runlog-manifest path="outputs/demo_runlog.jsonl" out="outputs/demo_runlog_manifest.json":
    cargo run -p pid-runlog --bin pid-runlog-replay -- --manifest-json {{path}} {{out}}

runlog-sidecars path="outputs/demo_runlog.jsonl":
    cargo run -p pid-runlog --bin pid-runlog-replay -- --write-sidecars {{path}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- --verify-sidecars {{path}}

runlog-sim-verify path="outputs/demo_bridge_runlog.jsonl":
    cargo run -p pid-sim --bin pid-sim-verify -- {{path}}

runlog-rerun path="outputs/demo_runlog.jsonl" out="outputs/demo_runlog.rrd":
    cargo run -p pid-rerun --bin runlog-to-rerun -- {{path}} --save {{out}}

runlog-rerun-bridge path="outputs/demo_bridge_runlog.jsonl" out="outputs/demo_bridge_runlog.rrd":
    cargo run -p pid-rerun --bin runlog-to-rerun -- {{path}} --save {{out}}

runlog-bridge-export-rerun source="outputs/demo_bridge_runlog.jsonl" path="outputs/demo_bridge_export_rerun_runlog.jsonl" out="outputs/demo_bridge_export_rerun.rrd":
    cargo run -p pid-sim --bin pid-sim-bridge-demo -- {{source}}
    printf '%s\n' '{"jsonrpc":"2.0","id":"export","method":"export.rerun","params":{"run_log_uri":"{{source}}","output_uri":"{{out}}"}}' | cargo run -p pid-sim --bin pid-sim-bridge-stdio -- {{path}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{path}}
