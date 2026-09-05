# A reproducible LeWM engineering adapter

This milestone makes a bounded pretrained-model experiment reproducible from verified local assets.
It does not qualify M2, W1, W2, W3, raw commands, or a CREBAIN adapter.

## Ten packaging choices

| Choice | Benefit | Failure mode | Decisive check |
| --- | --- | --- | --- |
| Install the complete upstream stack | Ordinary imports | Unrelated training and dataset dependencies | Inspect its full dependency closure |
| Follow current upstream main | Newer package surface | Breaks the pinned constructor contract | Reject a changed source identity |
| Modify installed package initializers | Small import closure | Invalid installed-file records | Reject modified installed files |
| Inject namespaces at runtime | Retains original modules | Brittle module ownership | Reject preloaded foreign namespaces |
| Extract constructor AST at runtime | Preserves one function | Hidden dynamic loading contract | Require ordinary source imports |
| Rewrite the entire model | Full local ownership | Architecture and numerical drift | Differential inference on both arms |
| Vendor upstream code into Git | Simple offline import | Additional update and redistribution surface | Exact source and notice review |
| Convert weights to another framework | Potential device benefit | Changes model arithmetic | Separate future qualification |
| Stage a deterministic ordinary package | Small, auditable import closure | Projection drift or missing notices | Exact module roster and byte checks |
| Keep only the custody runner | No new implementation | No maintained product workflow | Require a documented repository CLI |

The selected design stages an ordinary package from verified local source and wheels.
It copies exercised upstream modules without changing their bytes.
It replaces only eager package initializers with explicit, recorded minimal initializers.
A small Prisoma-owned ViT constructor states the exact reviewed configuration directly.
No runtime AST extraction or namespace injection is used.

## Five decision lenses

| Lens | Decision |
| --- | --- |
| Mathematical fidelity | Preserve actual model classes, CEM, preprocessing, and sample-standard-deviation updates |
| Scientific meaning | Separate standardized forecasts, raw support, reference outcomes, and physical claims |
| Authority and provenance | Verify bytes before imports and loading; grant no execution or estimator authority |
| Maintainability and distribution | Keep upstream code and weights outside Git; stage a closed ordinary package |
| Operator understanding | Provide one offline command, bounded artifacts, independent verification, and readable mathematics |

## Owned footprint

- `experiments/lewm/`: artifact admission, deterministic source staging, model construction, candidate contracts, qualification, and verification.
- `tests/python/test_lewm_*.py`: default checks without Torch or model downloads.
- `docs/lewm/`: run instructions, source concordance, mathematics, PID handoff, SVG, and a derived PDF.
- Active documentation entrypoints: small updates after the actual repository command passes.

The root Python environment remains small. Model dependencies use a separate, exact optional runtime.
The estimator submodule, original checkouts, NCP capture implementation, and historical scientific evidence remain outside this change.

## Required controls

The default gate checks valid and invalid source manifests, content hashes, source projection, bounded array admission, and candidate ordering.
It also checks unsupported command rejection, forecast commitments, trace arithmetic, and candidate-target injection rejection.

The opt-in gate uses the actual pretrained checkpoint and real PushT renders.
It runs both frozen source arms, CPU/MPS comparisons, and all 30-by-300 CEM rounds.
It compares the maintained package against both retained construction-run arms.
One matching input does not establish general implementation equivalence.

## Rights boundary

The reviewed planning wheel declares MIT, but supplies no license file.
The corresponding old source tree has the same notice discrepancy.
The staged package retains the exact metadata and every notice actually supplied by its inputs.
Current upstream license text does not silently resolve historical notice provenance.
This engineering adapter does not claim complete model, dataset, or transitive-rights clearance.

## Evidence boundary

The experiment commits observed inputs and candidate tensors before computing forecasts.
It commits each forecast and the separately scored final recommendation.
It executes no action and obtains no outcome label.

The independent verifier reconstructs captured CEM arithmetic and objective joins.
It does not independently prove image encoding, forecast quality, or simulator behavior.
The exact-fork Agent Bridge reference remains the separate owner of executable experiment-order semantics.
