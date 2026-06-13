# SAFE → (V, L, D, A) adapter (milestone M5 capture shortcut)

Adapts the released **SAFE** VLA rollout datasets (`vla-safe/SAFE`, NeurIPS 2025 —
OpenVLA on WidowX/LIBERO, π0-FAST on Franka, with success/failure outcomes) into
this project's `(V, L, D, A)` + labels contract, so a real VLA/task capture can be
run through `pid-offline-harness` without building capture from scratch. This is
the critical-path shortcut recorded in `REVIEW_AND_TODO.md` and `grandplan.md`
§10.10.13 / §12.5.

## What the released SAFE tensors actually give you (read this first)

This adapter is deliberately honest about provenance. Per rollout step, the
released SAFE data cleanly provides:

| Contract variable | Source in SAFE | Provenance tag |
|---|---|---|
| **A** (action) | `action_vectors` (e.g. 7-D `dx,dy,dz,droll,dpitch,dyaw,dgripper`) | `action_vector` |
| **D** (world/plan) | `hidden_states` — the policy backbone state `D_hidden[k]` | `hidden_state_pool` or `token_slice:state` |
| success label | `episode_success` | `episode_success` |
| `episode_id` | `task{id}--ep{idx}` | — |
| train/held-out split | SAFE seen/unseen task split | — |

It does **not** ship clean, separate pre-fusion vision `V` or text `L` embeddings.
The adapter never fabricates them. The supported non-fabricated sources are:

- **token slicing** (`token_slice`): if the *raw* per-token hidden states
  `(T, n_token, d)` and token-group ranges are exported, slice the vision /
  language / state token groups for genuine `V` / `L` / `D` sub-representations.
  This is the default mapping and the one the synthetic fixture exercises.
- **explicit features** (`explicit`): supply a separately extracted `(T, d_v)`
  vision-feature array (e.g. from running a vision encoder over the rollout frames)
  or `(T, d_l)` language features from a sentence encoder.
- **text hashing proxy** (`text_hash`): a deterministic, transparent featurization
  of the instruction text for `L`. Clearly labelled `text_hash_proxy` in metadata —
  it is reproducible but is **not** a learned semantic embedding; prefer real
  `language_features`.

If only **pooled** hidden states are available and you select `hidden_pool` for
`V`/`L`/`D`, the decomposition is degenerate (`V == L == D`); prefer `--text-proxy-l`
for `L` and treat that run as a `D`+`A` analysis with a text-proxy `L`. Every sample
records the per-variable provenance in `metadata.*_provenance` so no proxy is ever
silent.

> Before using a checkpoint, verify the exact released tensor shapes, per-step
> coverage, and licenses (the SAFE repo is unlicensed at time of writing; the
> rollout downloads are linked from its README).

## Choosing the `D` hook layer (grandplan §7.6.3)

When you export multiple candidate hidden layers, use
`layerwise_physics_probe(...)` to run the Physics-Emergence-Zone procedure before
geometry gating: it linearly probes each layer for physical quantities (object
speed/direction/contact) on a train split, scores them on held-out, and reports the
**peak layer** — the recommended `D_hidden[k]` hook. It warns when the peak is in
the near-output layers (likely action formatting, not a world model) and when the
peak is intermediate (the emergence zone to prefer).

## End-to-end usage

```bash
# 1. (testing) synthesize a SAFE-format rollout directory with learnable structure
python -m experiments.safe_adapter synth --out /tmp/safe_synth

# 2. convert to the (V,L,D,A) contract; seen tasks 0,1 -> train, rest -> held-out
python -m experiments.safe_adapter convert \
    --rollouts /tmp/safe_synth --out outputs/safe_vlda.json --seen-tasks 0,1

# 3. pre-flight verify (class coverage + episode disjointness, fail-closed)
python -m experiments.safe_adapter verify --input outputs/safe_vlda.json

# 4. run the real harness with the leakage gates the contract was built to pass
cargo run -p pid-sim --bin pid-offline-harness -- \
    --input outputs/safe_vlda.json \
    --summary-json outputs/safe_vlda_summary.json \
    --runlog outputs/safe_vlda_runlog.jsonl \
    --require-heldout-split --require-heldout-class-coverage \
    --require-heldout-episode-disjoint
```

For real SAFE downloads, point `--rollouts` at the unpacked rollout directory
(`task*.csv` + `task*.pkl`) and pass the seen-task ids SAFE used for that benchmark.

## Status

Implemented and tested end-to-end on a synthetic, schema-faithful SAFE fixture
(`tests/python/test_safe_adapter.py`): synth → load → convert → verify → real Rust
harness (all three strict leakage gates pass; PID screens + non-PID baselines incl.
the SAFE-class logistic-regression detector run on the result). Running it on the
real multi-GB SAFE downloads is a data-pull step (verify tensors/licenses first);
no code change should be needed for the default `token_slice` / `pooled` paths.
