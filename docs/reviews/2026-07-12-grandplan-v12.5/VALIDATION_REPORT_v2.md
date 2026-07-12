# Validation report — Prisoma grand plan v12.5

**Validation date:** 2026-07-12
**Canonical file:** `grandplan_v12_5.md`
**Canonical SHA-256:** `7906c6814d26824d802e93aed17e9da79d74a7a898c00232ff78140d25ce5356`
**Source snapshot:** `sepahead/prisoma@64bd881248463e7142d022bb95a5850bcf8fced2`

## Overall result

**PASS for internal artifact integrity and the declared review checks.**

This is not an empirical-validation verdict. It means the canonical document, evidence ledgers, patches, and archive satisfy the checks below. It does not mean that PID is valid in the proposed high-dimensional regime, that the hypotheses are true, or that the plan is scientifically “perfect.”

## Check 1 — structural, citation, and markup integrity

The canonical candidate was parsed independently for text, Markdown structure, references, URLs, tables, and math delimiters.

| Check | Result |
|---|---:|
| UTF-8 decoding | Pass |
| Final newline | Pass |
| NUL bytes | 0 |
| Trailing-whitespace lines | 0 |
| Tab-containing lines | 0 |
| Lines | 2,536 |
| Bytes | 198,502 |
| Approximate word tokens | 25,078 |
| Headings | 222 |
| Duplicate headings | 0 |
| Heading-level jumps | 0 |
| Code-fence events | 4, balanced |
| Reference definitions | 112 |
| Reference-ID sequence | exactly R1–R112 |
| Unique cited reference IDs | 112 |
| Total citation mentions | 276 |
| Undefined citations | 0 |
| Unused reference definitions | 0 |
| Duplicate reference IDs | 0 |
| URLs | 114 |
| Unique URLs | 114 |
| Malformed URLs found by the static parser | 0 |
| Markdown table blocks | 11 |
| Inconsistent table-column counts | 0 |
| Odd unescaped dollar-delimiter lines | 0 |

The v12.5 reference ledger contains exactly 112 rows, all marked `defined_and_cited`. The previously duplicated ForesightSafety-VLA entry was removed; `R113` does not occur and the relevant URL occurs once.

## Check 2 — scientific and semantic safeguards

Thirty-two machine-checkable assertions were tested against the canonical text; all passed. The assertions require, among other safeguards:

- an explicit Protocol A/Protocol B split for H1;
- a prohibition on interpreting paired software response as a physical individual treatment effect;
- truly pre-treatment primary moderators and diagnostic-instrumentation noninterference;
- effect-specific, held-out validation for Protocol B, with factual outcome fit secondary;
- separation of finite-benchmark, superpopulation, and transported targets;
- censoring, competing risks, frozen alarm policy, lead-time bias controls, and process-level safety outcomes for H2;
- mandatory treatment of SAFE, Hide-and-Seek, ActProbe, Rewind-IL/TIDE, VLAConf, perturbation disagreement, activation-warning probes, Foresight-style latents, temporal-difference calibration, and Tri-Info as comparator families where information access permits;
- separate PID population, measure, estimator, and application gates;
- explicit distinction between Prisoma’s pinned `pid-rs` revision and later upstream main;
- NCP’s status as optional and read-only;
- an evidence-bounded ecosystem statement in which only `pid-rs` and NCP are direct verified edges at the reviewed snapshot;
- preservation of a publishable PID-free thesis path.

These assertions detect omitted safeguards and wording regressions. They cannot validate assumptions that require data, physical experiments, external reproduction, or mathematical proof.

## Check 3 — patch reproducibility and reversibility

Three independent patch paths were tested in temporary Git repositories.

| Patch | Apply check | Applied target byte-identical | Reverse check | Restored base byte-identical |
|---|---:|---:|---:|---:|
| original reviewed `grandplan.md` → v12.5 | Pass | Pass | Pass | Pass |
| v11 → v12.5 | Pass | Pass | Pass | Pass |
| v12.4 → v12.5 | Pass | Pass | Pass | Pass |

Important hashes:

| Artifact | SHA-256 |
|---|---|
| Original reviewed plan | `1a768ef2cd6bbf512b369bf8f0b70865093e31de49a26be06428b336822bf500` |
| v11 | `dad36ccee5c612341db2e6233a32a3f9607ca67dd8950a6ddd3ec0f67b69f7c7` |
| v12.4 | `ad2c69a2bb1cf54f9f6e8836d8f544fd1660783c05805f9a52b018df65d59919` |
| v12.5 | `7906c6814d26824d802e93aed17e9da79d74a7a898c00232ff78140d25ce5356` |

## Supporting ledgers

The final validation also checked the required schemas and nonempty required cells for:

- 112-row v12.5 reference audit;
- 21-row ecosystem evidence audit;
- 6,117-row original sentence-coverage ledger;
- 574-row original section-disposition ledger;
- 138-row original citation-disposition ledger.

The ecosystem audit screened the public metadata of all 174 repositories displayed on the `sepahead` GitHub profile, followed by deeper inspection of plausibly relevant projects. Positive integration language is restricted by an E0–E5 evidence ladder. Negative findings are bounded to inspected public surfaces.

## Archive integrity check

The final ZIP was built twice from the same payload and produced the same archive hash. It uses sorted entries and fixed timestamps, and passed all of the following:

1. the archive passes `unzip -t`;
2. it contains no duplicate names, absolute paths, or `..` traversal components;
3. its entry set exactly matches the declared payload;
4. every extracted payload file other than the manifest itself matches the byte count and SHA-256 in `prisoma_review_manifest_v2.json`;
5. the canonical plan inside the archive matches the canonical hash above;
6. the external `.sha256` file matches the final ZIP bytes.

The machine-readable result is in `validation_checks_v2.json`; the complete payload inventory is in `PACKAGE_FILELIST.txt`.

## Known limits

A live HTTP request was not made from the artifact container to every one of the 114 referenced URLs. Load-bearing current sources were manually inspected through the web research system, while the complete set received static URL, uniqueness, and citation checks.

No static audit can establish construct validity, treatment fidelity, noninterference in a physical system, estimator consistency in the intended regime, external validity, robustness to unanticipated shifts, or the truth of a hypothesis. Those require the staged empirical gates in the plan and independent specialist scrutiny.

The public-repository audit cannot observe private repositories, unpublished branches, local files, or stale documentation. Consequently, “not verified as integrated” is deliberately weaker than “does not exist.”
