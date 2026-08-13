# docs/ index

This directory holds generated views, dated records, and immutable archives.
For the full docset, see the documentation map in the root
[README](../README.md).

## Contents

| Entry | What it is |
|---|---|
| [`CAPABILITY_MATRIX.md`](CAPABILITY_MATRIX.md) | Generated capability/evidence inventory. Do not hand-edit. Regenerate with `python scripts/generate_capability_matrix.py --write`. |
| [`AUDIT-2026-07-09.md`](AUDIT-2026-07-09.md) | Dated historical audit record. |
| [`audits/2026-08-12-first-principles/`](audits/2026-08-12-first-principles/) | Current first-principles hypothesis, source, and estimator-adoption audit. |
| [`audits/2026-08-12-first-principles/WORLD_ACTION_MODEL_FRONTIER.md`](audits/2026-08-12-first-principles/WORLD_ACTION_MODEL_FRONTIER.md) | Deployed-graph WAM taxonomy, causal admission gate, and M4 Max decision. |
| [`archive/`](archive/) | Superseded grandplan v10.7. Immutable. |
| [`power-gate/`](power-gate/) | Retired dated artifact. Immutable. |
| [`reviews/`](reviews/) | Hash-bound review bundles. Immutable. |

## Rules

- Do not hand-edit generated files. Use the checked generator.
- Do not edit immutable archives, retired artifacts, or review bundles.
- Keep new documentation consistent with the canonical `grandplan.md`.
