# LeWM local qualification: pre-outcome decision

This record defines a bounded engineering run. It does not close M2, W1, W2, or W3.
The associated JSON plan freezes the inputs before model outcomes.

## Ten approaches

| Approach | Assumption and benefit | Failure mode | Decisive control |
| --- | --- | --- | --- |
| Existing affine fixture | Reuses complete local semantics | No pretrained visual model | Retain its existing gate |
| Repeat old synthetic probe | Confirms tensor compatibility | Misses real preprocessing | Real PushT renders |
| Unmodified CUDA evaluator | Preserves released execution | Host has no CUDA | Reject CUDA on this host |
| Current platform main | Could simplify integration | Breaks the pinned constructor | Retain exact 0.1.1 |
| Whole training environment | Provides every optional package | Broad unrelated dependencies | Minimal import closure |
| Repository JEPA port | Preserves canonical reviewed classes | Differs from model configuration | Freeze both source readings |
| Model-config LeWM port | Follows released checkpoint configuration | May differ from historical probe | Freeze both source readings |
| Reduced CEM first | Reduces run time | Changes the target before reproduction | Exact 30-by-300 first |
| Real-render fixed candidates | Tests pretrained action dependence | Standardized actions lack raw support | No command execution |
| Exact-budget search plus later scaled loop | Tests actual planner, then control | Unknown scaler invalidates commands | Bind scaling before a new execution plan |

The selected sequence combines both source readings, real renders, fixed candidates, and exact-budget CEM.
It preserves unsuccessful outcomes. A later plan must authorize any scaled closed-loop execution.

## Twenty review lenses

| Lens | Required boundary |
| --- | --- |
| Product | Test an actual pretrained action-conditioned model locally |
| Architecture | Use project model classes and the existing environment |
| Source identity | Bind exact source files and dependency versions |
| Weights | Verify bytes, hash, weights-only ingress, and strict key matching |
| Model configuration | Compare repository JEPA with checkpoint-configured wheel LeWM |
| Observation | Record actual PushT pixels and state |
| Appearance | Retain official normalization and resize order |
| Actions | Distinguish standardized blocks from raw two-dimensional commands |
| Dynamics | Pinned PushT owns physics; no surrogate simulator |
| Targets | Goal pixels remain separate from future outcomes |
| Numerical validity | Require finite outputs and frozen CPU/MPS tolerances |
| Missingness | Reject absent or malformed input; never fill scientific evidence |
| Selection | Record forecasts, costs, and deterministic candidate order |
| Search | Retain all rounds, elites, distribution updates, and final-mean score |
| Replay | Hash input arrays and every retained trace artifact |
| Resources | Bound threads, wall time, and trace bytes |
| Isolation | Use a private runtime; disable network and MPS fallback |
| Rights | Record code, package, checkpoint, and data rights separately |
| Baselines | No quality claim from action sensitivity or planner execution |
| Publication | Engineering receipts cannot promote scientific statuses |

## Source concordance

- `config/train/model/lewm.yaml` specifies repository `JEPA` and `ARPredictor`.
- The pinned model `config.json` specifies wheel `LeWM` and `Predictor`.
- `eval.py` uses `load_pretrained`, so its selected model configuration determines the implementation.
- Strict checkpoint key matching does not resolve this implementation difference.
- Both readings are frozen. Neither can be selected from favorable outcomes.
- Evaluation preprocessing scales uint8 pixels, applies ImageNet normalization, then resizes to 224 pixels.
- Training groups five dense actions and sets the action encoder width to ten.
- Evaluation fits `StandardScaler` on its selected dataset after removing NaN rows.
- Training normalization uses `torch.std`; evaluation uses `StandardScaler`.
- Their variance conventions differ. The first run makes no raw-action execution claim.
- The released solver samples unbounded standardized Gaussians. Its `Box` supplies dimensions only.
- The solver updates scale with sample standard deviation and returns the final mean without separately scoring it.
- The instrumentation records the exact solver callbacks and separately scores that final mean.
- The published configuration uses 30 rounds, 300 samples, 30 elites, five blocks, and five actions per block.
- `_set_state` advances physics once. Its seven-value state omits block velocity and solver memory.
- This state setter is not a general exact-checkpoint API.

The dataset revision is `655cd446b9929369d7d406001da85c15d1457850`.
Its compressed artifact has 13,136,247,974 bytes and SHA-256 `7cfbd6d90fa2f27876379a5ff169715a36ed82edbda64f9e5b5bfa34d212f318`.
No dataset download occurs in this runner. No scaler is invented.

Primary sources: [pinned LeWM source](https://github.com/Mengarr/lewm/tree/8a2c595813d0eee85b2dbffa6f58ff0842f9e673),
[pinned checkpoint](https://huggingface.co/quentinll/lewm-pusht/tree/22b330c28c27ead4bfd1888615af1340e3fe9052),
and [dataset revision](https://huggingface.co/datasets/quentinll/lewm-pusht/tree/655cd446b9929369d7d406001da85c15d1457850).
