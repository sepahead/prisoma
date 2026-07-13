# SAFE → (V, L, D, A) adapter (S2/EC1 reference capture adapter)

Adapts the released **SAFE** VLA rollout datasets into this project's `(V, L, D, A)` +
labels contract, so a real VLA/task capture can be run through `pid-offline-harness`
without building capture from scratch. SAFE (`vla-safe/SAFE`; NeurIPS 2025 per the repo — verify venue/license) released OpenVLA-on-WidowX/LIBERO and π0-FAST-on-Franka rollouts with success/failure outcomes. This is
the reference capture adapter for the S2/EC1 gate recorded in `REVIEW_AND_TODO.md` and
`grandplan.md` §8.7 (adapter contract) / §8.9.4 (adapter promotion contract).

## What the released SAFE tensors actually give you (read this first)

This adapter is deliberately honest about provenance. Per rollout step, the
released SAFE data cleanly provides:

| Contract variable | Source in SAFE | Provenance tag |
|---|---|---|
| **A** (action) | `action_vectors` (e.g. 7-D `dx,dy,dz,droll,dpitch,dyaw,dgripper`) | `action_vector` |
| **D** (neutral candidate internal-state axis) | `hidden_states` — a declared policy-backbone site `D_hidden[k]`; world/plan semantics remain unestablished | `hidden_state_pool` or `token_slice:state` |
| success label | `episode_success` | `episode_success` |
| `episode_id` | `task{id}--ep{idx}` | — |
| train/held-out split | Prisoma's manifest-bound outer partition: declared seen task IDs → train, remaining task IDs → test (not SAFE's internal train/`val_seen`/`val_unseen` roles) | — |

It does **not** ship clean, separate pre-fusion vision `V` or text `L` embeddings.
The adapter never fabricates them. The supported non-fabricated sources are:

- **token slicing** (`token_slice`): if the *raw* per-token hidden states
  `(T, n_token, d)` and token-group ranges are exported, slice the vision /
  language / state token groups for declared sub-representations. The slice names
  are provenance, not semantic validation: architecture/token-mask ancestry and
  held-out probes are still required before calling them vision, language, or
  dynamics variables. This is the default mapping mechanics exercised by the
  synthetic fixture.
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
> coverage, and rights. At the 2026-07-13 check, public `vla-safe/SAFE` main was
> `b6036abe07b2b2bb9996afb2c07f13d6a9f507c0`, had no repository license file,
> linked rollout downloads from its README, and loaded their `.pkl` metadata with
> unrestricted `pickle.load`. Re-check rather than treating that dated receipt as
> permission or as a permanent repository fact.

## Secure ingress contract

Downloaded pickle is executable code, not a data format that becomes trusted merely
because it is on local disk. The adapter therefore fails closed on raw `.pkl` by
default. Its normal input is a canonical per-episode bundle:

- the upstream action CSV;
- an NPZ array container loaded with `allow_pickle=False` after ZIP/NPY header and
  tensor-size checks; and
- a strict JSON metadata object containing identifiers, outcome, description, and
  optional token groups.

Every directory must also contain `safe_bundle_manifest.json`. The manifest binds
the exact byte length and SHA-256 of every admitted payload, the source name and
immutable revision, the complete task universe, fixed seen→train/other→test rule,
split origin, whether the split was frozen before outcomes, a contamination-review
receipt, a model/checkpoint/hook/tensor-contract receipt and semantic-validation
status, and an operator-declared rights-status/reference attestation. Extra,
missing, mixed-format, hash-mismatched, oversized, non-finite,
ragged, or filename/metadata-conflicting inputs are rejected before conversion.
Conversion also refuses missing lineage or pooling across different manifest,
source revision, rights, split, model/checkpoint/hook, tensor-contract, or semantic
receipts: one emitted dataset is one bound capture regime.
The manifest and raw episode hashes are carried into every emitted sample's lineage
metadata. The converter records the manifest locator as
`external_not_archived_by_converter`: it does not copy the bundle or tensor-contract
artifact into a repository-wide store. A real EC1 evidence package must therefore
archive those original artifacts separately and verify their recorded hashes before
the staging directory is removed. The `just safe-adapter` fixture intentionally
deletes its temporary synthetic bundle and is not promotion evidence. These are
integrity/provenance checks, not a license determination or a scientific gate pass.

To prepare an already safe-exported real bundle, generate the manifest without
deserializing its payloads:

```bash
python -m experiments.safe_adapter manifest \
    --rollouts /path/to/safe-export \
    --source-name vla-safe/SAFE \
    --source-revision <exact-commit-or-release> \
    --rights-status verified \
    --rights-reference <review-or-license-receipt> \
    --model-id <policy-id> \
    --checkpoint-revision <immutable-checkpoint-id> \
    --hook-id <module-and-capture-site> \
    --tensor-contract-sha256 <sha256> \
    --semantic-validation-status unvalidated \
    --seen-tasks 0,1 \
    --split-origin <versioned-split-rule-or-artifact> \
    --split-frozen-before-outcomes \
    --contamination-review <lineage-review-receipt>
```

These source, rights, split, contamination, and semantic fields are bounded
operator attestations; the manifest makes them immutable and auditable but does not
verify a remote revision, grant rights, or supply architecture evidence by itself.
`unverified` rights status is recorded but rejected unless the operator explicitly
passes `--allow-unverified-rights`; that override grants no rights. A
manifest-hashed legacy NumPy pickle may be attempted with
`--allow-legacy-pickle`, which uses a restricted NumPy-only unpickler and rejects
arbitrary globals. It is still a trust boundary, not a sandbox. The currently
documented upstream Torch-tensor pickles are intentionally outside that allowlist:
re-export them to NPZ/JSON in a disposable, resource-constrained environment whose
inputs and outputs are separately reviewed, then run the normal manifest path.
Omitting `--split-frozen-before-outcomes` records `false`; `not_assessed`
contamination or an unfrozen split is rejected by conversion unless the explicit
`--allow-unfrozen-split` audit-only override is present. That override cannot support
a held-out scientific claim: emitted samples retain a machine-readable blocked
eligibility verdict and the adapter verifier fails it. A downstream flag does not
become leakage-safe merely because the JSON says `test`, and the converter will not
invert the manifest's fixed seen→train mapping; the Rust harness's strict held-out
modes also reject the explicit blocked verdict.

## Choosing the `D` hook layer (grandplan §9.1 / §8.4)

When you export multiple candidate hidden layers, use
`layerwise_physics_probe(...)` to run the Physics-Emergence-Zone procedure before
geometry gating: it linearly probes each layer for physical quantities (object
speed/direction/contact) on a train split, scores them on held-out, and reports the
**peak layer** — the recommended `D_hidden[k]` hook. It warns when the peak is in
the near-output layers (likely action formatting, not a world model) and when the
peak is intermediate (the emergence zone to prefer).

## End-to-end usage

```bash
# 1. (testing) synthesize a bounded canonical bundle + hashed manifest
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
    --require-heldout-episode-disjoint --require-axis-provenance-honest
```

`--require-axis-provenance-honest` is an opt-in gate (mirroring
`--require-geometry-pass`) that fails the run on degraded or absent axis
provenance markers; the offline VLDA harness surfaces the per-axis
`{v,l,d,a}_provenance` it checks (`token_slice:*` / `hidden_state_pool` /
`action_vector` are honest, `text_hash_proxy` is degraded). The `just
safe-adapter` recipe passes this flag plus the three held-out gates. Here
“honest” means the declared bytes/slice are present and not silently fabricated;
it is a mechanical provenance gate, not architecture/semantic validation. The
synthetic bundle therefore records `semantic_validation_status=unvalidated` even
though its token-slice provenance gate passes.

## Status

Implemented and tested end-to-end on a synthetic, schema-faithful canonical SAFE fixture
(`tests/python/test_safe_adapter.py`): synth → load → convert → verify → real Rust
harness (all three strict leakage gates pass; PID screens + non-PID baselines incl.
the SAFE-class logistic-regression detector run on the result). Real capture remains
blocked on obtaining/re-exporting the data, freezing the exact source and split,
verifying tensors and rights, and satisfying the scientific gates. Secure ingress
readiness is not S2/EC1 completion and establishes no H1/H2 evidence.
