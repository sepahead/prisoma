# Faithfulness-checked attribution probe (grandplan §14.7.1 / H9)

The H9 attribution protocol, end to end: attribute a **declared scalar target** of a
transformer to its inputs, **faithfulness-check** the attribution against a random
control, and emit schema-conformant `attribution_logged` run-log events with artifact
provenance. H9 attributions are triangulation evidence for (or against) PID claims —
and §14.7.1 is explicit that an attribution which fails its own faithfulness check
cannot corroborate or falsify a PID signature. This package enforces exactly that
guard.

## What is real here, and what is a stand-in

Reusable / production-relevant (implemented for real, tested):

* **faithfulness check** (`faithfulness.py`) — deletion AOPC vs a random control, in
  a sign-robust form for a signed regression target. This is the load-bearing guard.
* **run-log emission** (`runlog.py`) — writes `run_started` / `config_logged` /
  `attribution_logged` / `run_ended` JSONL that passes `pid-runlog-replay --validate`,
  with relevance arrays saved as `.npy` artifacts plus `sha256` and a `score_hash`.
  The `config_hash` is computed to match `serde_json`'s canonical serialization.
* **probe orchestration** (`probe.py`) — attribute, check, assemble records.

Stand-in (swap for production):

* the **model** (`model.py`) is a small numpy self-attention block producing a scalar
  target, so the pipeline runs without GPUs or a VLA checkpoint;
* `lrp_epsilon` (`attribute.py`) is epsilon-LRP with the **detached-softmax**
  attention rule (relevance routed through the value path) — the core AttnLRP
  simplification. For a real transformer VLA, replace the model with the checkpoint
  and the method with the **LXT / AttnLRP** library
  (`rachtibat/LRP-eXplains-Transformers`, arXiv:2402.05602; pin the revision). The
  faithfulness check, provenance, and run-log contract are unchanged.

`grad_x_input` computes the gradient by central finite differences (numpy-only, and
self-checking); a test verifies it against torch autograd when torch is installed.

## Usage

```bash
python -m experiments.attribution demo \
    --runlog outputs/attribution_runlog.jsonl --artifacts outputs/attribution
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/attribution_runlog.jsonl
```

Each method prints its faithfulness verdict and AOPCs; the run log carries one
`attribution_logged` event per method with `faithfulness_check` set accordingly.

## Status

Implemented and tested (`tests/python/test_attribution.py`): epsilon-LRP relevance
conservation, gradient agreement with torch autograd (when available), the
faithfulness check distinguishing a real attribution from an uninformative one, and
the emitted run log validating with the real Rust replay tool. Applying it to a real
VLA is gated on capture hardware / the LXT integration — the same honest gating as the
SAFE capture adapter; no contract change is expected.

**Preregister against the v10.7 H9 criterion** (`grandplan.md` §14.1): per matched case,
Kendall τ between the PID-derived and attribution-derived modality orderings; the H9
statistic is the mean per-case τ across ≥ 20 cases with a family-blocked case-resampling
bootstrap — supported only if the CI is entirely > 0 AND the top-modality match rate is
≥ 70%; a CI entirely < 0 is affirmative disconfirmation.
