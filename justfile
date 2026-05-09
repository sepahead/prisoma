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

toy-harness runlog="outputs/toy_vla_runlog.jsonl" summary="outputs/toy_vla_summary.json" episodes="32":
    cargo run -p pid-sim --bin pid-toy-harness -- --episodes {{episodes}} --summary-json {{summary}} --runlog {{runlog}}
    cargo run -p pid-runlog --bin pid-runlog-replay -- --validate {{runlog}}

# M1 run-log smoke path.
runlog-demo:
    cargo run -p pid-sim --bin pid-sim-demo -- outputs/demo_runlog.jsonl

runlog-bridge-demo:
    cargo run -p pid-sim --bin pid-sim-bridge-demo -- outputs/demo_bridge_runlog.jsonl

runlog-bridge-stdio path="outputs/demo_bridge_stdio_runlog.jsonl":
    printf '%s\n' '{"jsonrpc":"2.0","id":"status","method":"sim.status","params":{}}' '{"jsonrpc":"2.0","id":"step","method":"sim.step","params":{"dt":0.1}}' | cargo run -p pid-sim --bin pid-sim-bridge-stdio -- {{path}}

runlog-bridge-stdio-safe path="outputs/demo_bridge_stdio_safe_runlog.jsonl":
    printf '%s\n' '{"jsonrpc":"2.0","id":"status","method":"sim.status","params":{}}' '{"jsonrpc":"2.0","id":"step","method":"sim.step","params":{"dt":0.1}}' | cargo run -p pid-sim --bin pid-sim-bridge-stdio -- --safe-mode {{path}}

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

runlog-sim-verify path="outputs/demo_bridge_runlog.jsonl":
    cargo run -p pid-sim --bin pid-sim-verify -- {{path}}

runlog-rerun path="outputs/demo_runlog.jsonl" out="outputs/demo_runlog.rrd":
    cargo run -p pid-rerun --bin runlog-to-rerun -- {{path}} --save {{out}}

runlog-rerun-bridge path="outputs/demo_bridge_runlog.jsonl" out="outputs/demo_bridge_runlog.rrd":
    cargo run -p pid-rerun --bin runlog-to-rerun -- {{path}} --save {{out}}
