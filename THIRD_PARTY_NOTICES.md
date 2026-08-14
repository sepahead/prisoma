# Third-Party Notices

This file is the curated release-governance overview for prisoma. The machine-generated
direct-dependency bill of materials lives in `THIRD_PARTY_NOTICES.generated.md`
(produced by `scripts/generate_third_party_notices.py`; CI fails on drift). Neither
file is yet a complete *transitive* BOM — regenerate and review notices with dedicated
tooling before distributing binaries, wheels, Tauri apps, sidecars, datasets, model
weights, generated assets, or 3DGS captures.

## Generated dependency notices

- `uv run --no-sync python scripts/generate_third_party_notices.py --write` regenerates
  `THIRD_PARTY_NOTICES.generated.md` (direct Rust deps + licenses from
  `cargo metadata`; declared Python deps + versions from `uv.lock`).
- `--check` (run in CI) fails if the committed generated file is stale.

## Project License

prisoma project code is dual-licensed **MIT OR Apache-2.0** (see `LICENSE-MIT` and
`LICENSE-APACHE`). Local Rust crates declare `license = "MIT OR Apache-2.0"`.

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

## Reviewed external model candidates

These artifacts are not dependencies and are not distributed by Prisoma. Their licenses do not
become the Prisoma license.

| Candidate | Reviewed identity | Rights boundary |
|---|---|---|
| LeWorldModel PushT | `Mengarr/lewm@8a2c595813d0eee85b2dbffa6f58ff0842f9e673`; locked `stable-worldmodel==0.1.1` wheel SHA-256 `00eaabd9e046e6364b3d1db47e5b365a0f628aea3a9376d6a407f75cbbbd2ef5`, source tag `15a5538d492ae524c64cb18cc56a2d70611e877e`; locked `stable-pretraining==0.1.7` wheel SHA-256 `60fc8fc3c9490e9a059aa7e038ab62cbe0505841e78c4165c18a99d8f599ec65`; `quentinll/lewm-pusht@22b330c28c27ead4bfd1888615af1340e3fe9052` | LeWM source and model-card terms declare MIT. `stable-worldmodel` 0.1.1 wheel metadata declares MIT, but its wheel and reviewed source tag have no license file. `stable-pretraining` 0.1.7 includes an MIT license file. Resolve the former discrepancy and review data plus transitive rights before adoption. |
| JEPA-WM PushT | `facebookresearch/jepa-wms@13cf1d9c7e476f53c17714d2e0f1dc239a883ce0`; `facebook/jepa-wms@9b9c41ef249466630dbf1a20e78391865d07b3b9` | Code and model card declare `CC-BY-NC-4.0`. Do not distribute or use commercially without a separate rights decision. |
| SmolVLA | `huggingface/lerobot@a16f34c085c9597fcbdb9fde395a3334d78df716`; `lerobot/smolvla_base@c83c3163b8ca9b7e67c509fffd9121e66cb96205` | LeRobot code is Apache-2.0. The reviewed model card did not declare a weight license. |
| VLA-JEPA | same LeRobot code revision; `lerobot/VLA-JEPA-LIBERO@735d9f692981e286ade093b5046627eda876e5d0` | Reviewed code and model card declare Apache-2.0. Data, simulator, and upstream encoder rights remain separate. |

Recheck every source, model card, dependency, dataset, and intended use before download,
redistribution, publication, or deployment. A research paper's license does not license code or
weights.

## Release Checklist

1. Regenerate `THIRD_PARTY_NOTICES.generated.md` (`--write`) and confirm `--check` is clean.
2. Run `cargo deny --locked --all-features check` on the root graph. The `rapier` feature adds
   `rapier3d-f64` and its tree. Run
   `cargo deny --locked --manifest-path crates/ncp-observer/Cargo.toml check` on the separate NCP
   graph. Do not distribute that binary while the blockers in `SECURITY.md` remain open.
3. Resolve Python dependency licenses (`pip-licenses`) — `uv.lock` records versions but not licenses.
4. Run npm license tooling when a Tauri/Web frontend is added; include Rerun/Tauri sidecar notices if binaries are bundled.
5. Confirm that `meshmaker/README.md` is the only tracked `meshmaker/` file. Confirm that
   `api_keys.txt`, tooling, and generated outputs are absent (see `meshmaker/README.md`).
6. Record license/provenance for VLA checkpoints (e.g. the SAFE rollout datasets used by `experiments/safe_adapter`), video/world-model weights, datasets, generated meshes, prompts, 3DGS captures, and robot/sim assets separately from code.
7. Block release on unknown, copyleft-incompatible, non-commercial, or unclear artifact licenses unless the intended distribution allows them.
