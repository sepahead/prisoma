# Prisoma `grandplan.md` second-round review bundle

**Review date:** 2026-07-12
**Reviewed repository:** `sepahead/prisoma`
**Reviewed source commit:** `64bd881248463e7142d022bb95a5850bcf8fced2`
**Canonical revised plan:** `grandplan_v12_5.md`
**Canonical SHA-256:** `7906c6814d26824d802e93aed17e9da79d74a7a898c00232ff78140d25ce5356`

This archive contains the original reviewed plan, the first-round v11 replacement and audits, the complete second-round revision history through v12.5, ecosystem evidence, reversible patches, machine-readable validation results, and integrity manifests.

## Start here

1. Read `grandplan_v12_5.md`. It is the canonical candidate, not any earlier version.
2. Read `grandplan_deep_review_v2_2026-07-12.md` for the scientific reasoning behind the second round.
3. Read `prisoma_ecosystem_integration_audit_2026-07-12.md` for the evidence-bounded review of related public `sepahead` projects.
4. Read `VALIDATION_REPORT_v2.md` for the triple-check results and limitations.
5. Use `prisoma_review_manifest_v2.json` and the outer archive `.sha256` file for integrity verification.

## Applying the canonical replacement

The full replacement patch expects `grandplan.md` from the reviewed source commit:

```bash
git checkout 64bd881248463e7142d022bb95a5850bcf8fced2
git apply --check grandplan_v12_5_replacement.patch
git apply grandplan_v12_5_replacement.patch
sha256sum grandplan.md
```

The resulting hash should be:

```text
7906c6814d26824d802e93aed17e9da79d74a7a898c00232ff78140d25ce5356
```

Two incremental patches are also supplied:

- `grandplan_v11_to_v12_5.patch` expects the supplied `grandplan_v11.md` as `grandplan.md`.
- `grandplan_v12_4_to_v12_5.patch` expects the supplied `grandplan_v12_4.md` as `grandplan.md`.

All three patches were checked, applied, reversed, and compared byte-for-byte against their expected base and target files.

## What changed in v12.5

The second round adds four major corrections.

First, H1 is split into a paired frozen-snapshot algorithmic-response protocol and a genuinely randomized closed-loop protocol. The former is no longer allowed to masquerade as a physical individual treatment effect. The latter requires pre-treatment moderators, diagnostic noninterference, held-out heterogeneous-treatment-effect scoring, causal calibration, and policy-value or regret analysis.

Second, H2 is rebuilt as a prospective monitoring claim with explicit censoring, competing events, frozen alarm policies, lead-time safeguards, process-level safety outcomes, and a materially stronger mandatory comparator frontier. The May 29, 2026 Hide-and-Seek runtime-monitoring paper is included among the required comparators.

Third, PID remains conditional. Population validity, measure validity, estimator validity, and application eligibility are separate gates. The plan distinguishes the exact `pid-rs` revision pinned by Prisoma from later upstream evidence and requires an explicit scientific migration before any post-pin improvement can be inherited.

Fourth, the public `sepahead` ecosystem is represented through an E0–E5 evidence ladder. At the reviewed snapshot, only `pid-rs` and the optional read-only NCP observer have verified direct repository-level edges. Other repositories are treated as candidate producers, fixtures, comparators, transport settings, or scientific lineage unless executable evidence supports promotion.

## File guide

### Canonical and historical plans

- `grandplan_original.md` — source `grandplan.md` at the reviewed commit.
- `grandplan_v11.md` — first-round replacement.
- `grandplan_v12.md` through `grandplan_v12_4.md` — preserved second-round intermediates.
- `grandplan_v12_5.md` — canonical second-round candidate.
- `grandplan_v11_to_v12_5_change_log.md` — substantive delta summary.

### Review and evidence

- `grandplan_deep_review_2026-07-12.md` — first-round review.
- `grandplan_deep_review_v2_2026-07-12.md` — second-round adversarial review.
- `prisoma_ecosystem_integration_audit_2026-07-12.md` — detailed ecosystem audit.
- `prisoma_ecosystem_evidence_2026-07-12.csv` — structured evidence table for 21 relevant projects or project classes.
- `grandplan_v12_5_reference_audit.csv` — all 112 v12.5 references, definition lines, URLs, and citation counts.

### Original-plan coverage ledgers

- `grandplan_sentence_audit_2026-07-12.csv` — 6,117 machine-assisted sentence-like units from the original plan.
- `grandplan_section_disposition_2026-07-12.csv` — all 574 original headings.
- `grandplan_citation_disposition_2026-07-12.csv` — 138 original citation identifiers or URLs.

These ledgers establish visible coverage; they are not substitutes for independent expert judgment.

### Patches

- `grandplan_v12_5_replacement.patch` — original reviewed source to v12.5.
- `grandplan_v11_to_v12_5.patch` — v11 to v12.5.
- `grandplan_v12_4_to_v12_5.patch` — v12.4 to v12.5.
- `grandplan_v11_replacement.patch` — original reviewed source to v11, retained for provenance.

### Validation and integrity

- `VALIDATION_REPORT_v2.md` — human-readable triple-check report.
- `validation_checks_v2.json` — machine-readable checks.
- `prisoma_review_manifest_v2.json` — file sizes, hashes, and roles for the final payload; the manifest does not self-hash.
- `PACKAGE_FILELIST.txt` — compact sorted payload inventory.
- `grandplan_v12_5.md.sha256` — canonical-file checksum.
- `prisoma_grandplan_review_v12_5_2026-07-12.zip.sha256` — outer archive checksum, distributed beside the ZIP rather than inside it.

The first-round ZIP and its checksum are nested unchanged for provenance.

## Interpretation limits

The checks establish internal consistency, traceability, and reproducible patching. They do not prove the hypotheses, validate high-dimensional continuous PID for VLA representations, establish causal transport to unseen robots or environments, or guarantee acceptance by any committee or venue. Those claims require preregistration, data, experiments, independent replication, and specialist review.

The ecosystem audit covers public GitHub surfaces available on July 12, 2026. Absence of a verified public edge is not proof that no private, local, or unpublished integration exists.
