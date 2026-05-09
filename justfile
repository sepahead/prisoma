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

# M1 run-log smoke path.
runlog-demo:
    cargo run -p pid-sim --bin pid-sim-demo -- outputs/demo_runlog.jsonl

runlog-replay path="outputs/demo_runlog.jsonl":
    cargo run -p pid-runlog --bin pid-runlog-replay -- {{path}}

runlog-rerun path="outputs/demo_runlog.jsonl" out="outputs/demo_runlog.rrd":
    cargo run -p pid-rerun --bin runlog-to-rerun -- {{path}} --save {{out}}
