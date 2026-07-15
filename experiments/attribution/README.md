# Faithfulness-checked attribution probe (exploratory triangulation; grandplan §3.8 kill rules, §13 Lens 8)

> **Docset v12.5 note:** the old H9 hypothesis id is retired. Attribution is not a
> confirmatory claim in the v12.5 registry (EC1, H1–H4; grandplan §4); it is an
> **exploratory triangulation** comparator whose disagreement-under-intervention with PID
> is itself a diagnostic, bounded by the PID kill rules (§3.8) and the mechanistic-
> interpretability lens (§13 Lens 8).

The attribution protocol, end to end: attribute a **declared scalar target** of a
transformer to its inputs, **faithfulness-check** the attribution against a random
control, and emit schema-conformant `attribution_logged` run-log events with artifact
provenance. These attributions are triangulation evidence for (or against) PID claims —
and grandplan §3.8 (PID kill rules) is explicit that an attribution which fails its own
faithfulness check cannot corroborate or falsify a PID signature. This package enforces
exactly that guard.

## What is real here, and what is a stand-in

Reusable / production-relevant (implemented for real, tested):

* **faithfulness check** (`faithfulness.py`) — deletion AOPC vs a random control, in
  a sign-robust form for a signed regression target. This is the load-bearing guard.
* **run-log emission** (`runlog.py`) — writes `run_started` / `config_logged` /
  `attribution_logged` / `run_ended` JSONL that passes `pid-runlog-replay --validate`,
  accepts at most 1024 finite relevance values per record, and, when artifact output is
  enabled, saves them as NumPy v1.0 little-endian `f64` C-order artifacts plus exact
  file `sha256`, canonical shape metadata, and a `score_hash`. Artifact names are their
  file digest, are installed without replacement,
  and remain immutable across later runs. Artifact URIs are relative to the run-log
  directory for the converter's confined loader; the run-log name is replaced only
  after every referenced artifact has been installed and verified.
  The `config_hash` is computed to match `serde_json`'s canonical serialization. Run ids
  are nonempty and normalization-stable; caller metadata is preserved only when its keys
  and values are exact strings, with producer-owned fields and normalization collisions
  rejected. Per-line, file, encoded-string, container, and nesting limits mirror the
  canonical Rust reader; the producer additionally adopts the viewer's stricter 100,000-event,
  64 MiB serialized-event, and 8 MiB unique prepared-artifact caps. Every limit fails before
  any publication output.
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
cargo run -p pid-rerun --bin runlog-to-rerun -- \
    outputs/attribution_runlog.jsonl --load-attribution-artifacts \
    --save outputs/attribution_runlog.rrd
```

Each method prints its faithfulness verdict and AOPCs; the run log carries one
`attribution_logged` event per method with `faithfulness_check` set accordingly.
`--artifacts` must be a strict descendant of the run log's directory. Publication
rejects observed symlink/hard-link aliases, file-syncs staged contents, installs
content-addressed artifacts without clobbering, and atomically replaces the run-log
name last. This ordering preserves any prior valid publication after an artifact-install
failure, but it is not a multi-file transaction or parent-directory-fsync guarantee.
External artifact loading remains an explicit converter opt-in and is never enabled by
bridge export; opted-in loading requires the recorded exact digest and shape to match.

## Status

Implemented and tested (`tests/python/test_attribution.py`): epsilon-LRP relevance
conservation, gradient agreement with torch autograd (when available), the
faithfulness check distinguishing a real attribution from an uninformative one, and
the emitted run log validating with the real Rust replay tool. Applying it to a real
VLA is gated on capture hardware / the LXT integration — the same honest gating as the
SAFE capture adapter; no contract change is expected.

**Preregister as an exploratory triangulation check** (`grandplan.md` §4 exploratory
questions; §3.8 kill rules): per matched case, Kendall τ between the PID-derived and
attribution-derived modality orderings; the statistic is the mean per-case τ across ≥ 20
cases with a family-blocked case-resampling bootstrap — supported only if the CI is
entirely > 0 AND the top-modality match rate is ≥ 70%; a CI entirely < 0 is affirmative
disconfirmation.
