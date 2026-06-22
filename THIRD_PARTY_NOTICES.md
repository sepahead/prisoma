# Third-Party Notices

This file is the curated release-governance overview for prisoma. The machine-generated
direct-dependency bill of materials lives in `THIRD_PARTY_NOTICES.generated.md`
(produced by `scripts/generate_third_party_notices.py`; CI fails on drift). Neither
file is yet a complete *transitive* BOM — regenerate and review notices with dedicated
tooling before distributing binaries, wheels, Tauri apps, sidecars, datasets, model
weights, generated assets, or 3DGS captures.

## Generated dependency notices

- `python scripts/generate_third_party_notices.py --write` regenerates
  `THIRD_PARTY_NOTICES.generated.md` (direct Rust deps + licenses from
  `cargo metadata`; declared Python deps + versions from `uv.lock`).
- `--check` (run in CI) fails if the committed generated file is stale.

## Project License

prisoma project code is MIT licensed. Local Rust crates declare `license = "MIT"`.

## Checked Core Dependencies

| Component | Current role | License metadata checked |
|---|---|---|
| Rerun Rust SDK/viewer crate | Phases 1-3 diagnostics | `MIT OR Apache-2.0` |
| `@rerun-io/web-viewer` | Future embedded viewer option | `MIT` |
| `@tauri-apps/api` | Future Phase 4 app shell | `Apache-2.0 OR MIT` |
| `@sparkjsdev/spark` | Future Phase 4 custom 3DGS renderer | `MIT` |
| Three.js | Future Phase 4 rendering dependency | `MIT` |
| Rust `numpy` crate | Python extension interop | `BSD-2-Clause` |
| `nalgebra` | Numeric geometry | `Apache-2.0` |
| `serde`, `serde_json`, `anyhow`, `pyo3`, `ndarray` | Rust/Python infrastructure | `MIT OR Apache-2.0` |

## Release Checklist

1. Regenerate `THIRD_PARTY_NOTICES.generated.md` (`--write`) and confirm `--check` is clean.
2. Run a full transitive Rust license audit (`cargo deny` or `cargo about`) on the locked graph; the optional `rapier` feature adds `rapier3d-f64` and its tree.
3. Resolve Python dependency licenses (`pip-licenses`) — `uv.lock` records versions but not licenses.
4. Run npm license tooling when a Tauri/Web frontend is added; include Rerun/Tauri sidecar notices if binaries are bundled.
5. Confirm `meshmaker/` is absent from the released tree and its `api_keys.txt` lives outside the repo (see `meshmaker/README.md`).
6. Record license/provenance for VLA checkpoints (e.g. the SAFE rollout datasets used by `experiments/safe_adapter`), video/world-model weights, datasets, generated meshes, prompts, 3DGS captures, and robot/sim assets separately from code.
7. Block release on unknown, copyleft-incompatible, non-commercial, or unclear artifact licenses unless the intended distribution allows them.
