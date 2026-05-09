# Third-Party Notices

This file is release-governance groundwork for PID-VLA. It is not a complete generated bill of materials; regenerate and review notices before distributing binaries, wheels, Tauri apps, sidecars, datasets, model weights, generated assets, or 3DGS captures.

## Project License

PID-VLA project code is MIT licensed. Local Rust crates declare `license = "MIT"`.

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

1. Run a Rust license audit (`cargo deny` or `cargo about`) on the locked dependency graph.
2. Run npm license tooling when a Tauri/Web frontend is added.
3. Include Rerun/Tauri sidecar notices if binaries are bundled.
4. Record license/provenance for VLA checkpoints, video/world-model weights, datasets, generated meshes, prompts, 3DGS captures, and robot/sim assets separately from code.
5. Block release on unknown, copyleft-incompatible, non-commercial, or unclear artifact licenses unless the intended distribution allows them.
