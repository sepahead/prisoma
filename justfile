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
    cargo clippy -- -D warnings

# Experiment 0 gate (Rust-side smoke subset).
# Full Experiment 0 will later be orchestrated via python/experiments/.
exp0:
    cargo test -p pid-core exp0 -- --nocapture
