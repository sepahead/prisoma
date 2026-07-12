# Second-round grandplan review bundle (2026-07-12)

This directory preserves, for provenance, the independent second-round adversarial review
that produced **docset v12.5** — the version now adopted as the canonical `grandplan.md`.

- **Reviewed source commit:** `sepahead/prisoma@64bd881248463e7142d022bb95a5850bcf8fced2`
- **Canonical result:** `grandplan.md` (docset v12.5), SHA-256 `7906c6814d26824d802e93aed17e9da79d74a7a898c00232ff78140d25ce5356`
- **Outgoing plan archived at:** `../../archive/grandplan-v10.7.md` (SHA-256 `1a768ef2cd6bbf512b369bf8f0b70865093e31de49a26be06428b336822bf500`)

## Contents

| File | Role |
|---|---|
| `PRISOMA_REVIEW_README_v2.md` | Bundle entry point and file guide |
| `grandplan_deep_review_v2_2026-07-12.md` | Second-round adversarial scientific review (20-lens, findings) |
| `prisoma_ecosystem_integration_audit_2026-07-12.md` | Public-repository ecosystem audit (E0–E5 evidence ladder) |
| `grandplan_v11_to_v12_5_change_log.md` | Substantive delta v11 → v12.5 |
| `VALIDATION_REPORT_v2.md` / `validation_checks_v2.json` | Triple-check integrity/semantic/patch validation |
| `prisoma_review_manifest_v2.json` | File sizes, hashes, and roles for the payload |
| `grandplan_v12_5_reference_audit.csv` | All 112 v12.5 references (R1–R112), URLs, citation counts |
| `prisoma_ecosystem_evidence_2026-07-12.csv` | Structured evidence table for the ecosystem edges |

The full 914 KB reversible replacement patch (`grandplan_v12_5_replacement.patch`, SHA-256
`cbfaa2a3eb444ac903f63add5a27eb63dddc32ec13d4cfde97a8959de19bf337`) is not vendored — the
transformation is fully captured by git history plus the archived v10.7 and the current
`grandplan.md`.

## Reconciliation applied on adoption

The reviewed snapshot pinned NCP `v0.7.1`/wire 0.7. The repository has since migrated to
NCP **`v0.8.0`/wire 0.8**; active NCP-pin references in `grandplan.md` and the companion docs
were reconciled to the current pin on adoption. Everything else in v12.5 was adopted verbatim.
