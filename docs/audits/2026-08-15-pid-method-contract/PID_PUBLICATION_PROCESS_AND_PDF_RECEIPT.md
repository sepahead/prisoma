# PID publication-process and PDF receipt

- Status: reviewed process artifact
- Review date: 2026-08-16
- Scientific authority: [`PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md`](../../../PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md)
- Derived publication view: [`PID_Method_Selection_and_Publication_Contract.pdf`](../../../output/pdf/PID_Method_Selection_and_Publication_Contract.pdf)
- Renderer: [`render_pid_method_contract_pdf.py`](../../../scripts/render_pid_method_contract_pdf.py)

## 1. Scope and claim boundary

This receipt records how the PID method-selection contract was reviewed and rendered. It is process
evidence only. It does **not** promote a PID functional, evaluator, estimator, implementation,
validation result, application gate, thesis result, or novelty claim.

The Markdown file is the canonical scientific source. The PDF is a deterministic publication view
of those exact source bytes. The PDF must not be edited independently, and hash equality establishes
byte identity only. It does not establish mathematical correctness, readable layout, scientific
validity, or application validity.

## 2. Bound artifacts

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| `PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md` | 60,022 | `9dfebb8197a96c68a018477389ee21aa8d096901bbc8402dc5178ce12e9167d1` |
| `scripts/render_pid_method_contract_pdf.py` | 38,881 | `84c31ccf6042c1930665a8f7ce2c3c451efb6e87bbaf1c9bde806b119aded3b4` |
| `output/pdf/PID_Method_Selection_and_Publication_Contract.pdf` | 261,174 | `6a014ccf480c0f26a2cb444bbc4cd932e8fac0c6a626f7288ffbf0657c0d024b` |

The rendered document contains 18 A4 pages, 33 headings, 8 tables, and 9 displayed equations.

## 3. Renderer environment

| Component | Identity |
|---|---|
| Python | `3.11.15` |
| ReportLab | `4.4.7` |
| Matplotlib | `3.10.8` |
| Ghostscript | `10.03.0` |
| Poppler `pdftoppm` | `26.06.0` |
| Poppler `pdftotext` | `26.06.0` |

The renderer uses the following exact font files:

| Font file | Bytes | SHA-256 |
|---|---:|---|
| `/System/Library/Fonts/Supplemental/Arial.ttf` | 773,236 | `525979822591a3447cfc49d943d6f7683508e25543407871c0ed8fed05fd2bd9` |
| `/System/Library/Fonts/Supplemental/Arial Bold.ttf` | 750,984 | `d72db21f9242aedd6b917d8549ad5921766b24d5f8d0becfda2ff4c620b3c2e0` |
| `/System/Library/Fonts/Supplemental/Arial Italic.ttf` | 553,284 | `ce1d2f1ab89db45f9796100eee960f5702a40e84c225c2b48c3ec3e81d153f98` |
| `/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf` | 558,672 | `374b0190a9844343110d8f8ed1818117a4591803d022bbb2bd189d63a681e731` |
| `/System/Library/Fonts/Supplemental/Andale Mono.ttf` | 109,700 | `ca436a8f07f6699107542ebe19dcc9478f12aa666927699e9fa10115e7d2ee95` |

These absolute font identities make this receipt a record of the reviewed macOS render. A render on
another platform is a new render event and must receive its own receipt unless it reproduces the
bound PDF bytes exactly.

## 4. Exact render and replay commands

The PDF was created with:

```text
uv run --no-sync --group report --group analysis python scripts/render_pid_method_contract_pdf.py
```

Deterministic replay was checked with:

```text
uv run --no-sync --group report --group analysis python scripts/render_pid_method_contract_pdf.py --check
```

The replay produced the same PDF SHA-256 and byte size and reported:

```text
{"equation_count":9,"heading_count":33,"pdf_bytes":261174,"pdf_sha256":"6a014ccf480c0f26a2cb444bbc4cd932e8fac0c6a626f7288ffbf0657c0d024b","renderer_bytes":38881,"renderer_sha256":"84c31ccf6042c1930665a8f7ce2c3c451efb6e87bbaf1c9bde806b119aded3b4","source_bytes":60022,"source_sha256":"9dfebb8197a96c68a018477389ee21aa8d096901bbc8402dc5178ce12e9167d1","table_count":8}
```

## 5. Structural and extracted-text checks

The following checks passed:

1. Ghostscript parsed all pages using `-dNOPAUSE -dBATCH -sDEVICE=nullpage` with exit status zero.
2. `pdfinfo` reported 18 unrotated A4 pages, PDF 1.4, no encryption, no form, and no JavaScript.
3. `pdftotext -layout` extracted 75,183 characters with no Unicode replacement character.
4. The extracted text contained the generic-PID prohibition, the scientific-object registry, the
   signed order-K distinction, the complete `PID-M0 preserve` through `PID-M8 publish` process,
   the publication-process heading, and the primary-sources heading.
5. The extracted text contained exactly nine `Canonical source expression:` captions, one for each
   displayed equation. The captions are vector text and therefore independently searchable even
   though the equations are rendered at high resolution.
6. The renderer's `--check` path reconstructed byte-identical output without replacing the reviewed
   PDF.

## 6. Visual review

Every page was rendered to PNG with Poppler and inspected at original rendered detail. Pages 1–18
were checked for clipping, overlap, missing glyphs, broken equations, split tables, unreadable type,
unexpected blank pages, stale headers, page-number drift, and malformed links.

Disposition: **pass**. No visual defect remains. Tables remain legible, continuations are
unambiguous, equations have matching textual captions, page numbers are consistent, and all primary
sources fit legibly on page 18 without an isolated bibliography page. Each equation image and its
searchable canonical-source caption are kept together as one card. Inline code, mathematics, and
links in dark table headers are forced to white for sufficient contrast. Decorative running headers
were deliberately omitted so continuation pages have one consistent layout.

## 7. Twenty-lens scientific review

This review was applied to the Markdown authority, not inferred from the PDF layout.

| Lens | Reviewed question | Disposition |
|---:|---|---|
| 1 | Scientific-object identity | Functionals, coordinates, estimators, evaluators, transforms, certifiers, objectives, evidence, and interpretations are different typed nodes. |
| 2 | Defining mathematics | Each admitted route requires its equations, quantity, component, aggregation law, and theorem or derivation locator. |
| 3 | Probability-law domain | Finite categorical, empirical categorical, continuous, Gaussian, mixed, singular, atomic, quantized, and unknown laws are not interchanged. |
| 4 | Source count and lattice | Two-source, three-source, higher-source, and hierarchy outputs are separate mathematical regimes. |
| 5 | Functional versus route | A functional is not its sample estimator; a declared-law evaluator has no sampling claim. |
| 6 | Coordinate and atom meaning | A lattice coordinate, hierarchy component, averaged atom, and objective coefficient are not interchangeable. |
| 7 | Gauge and representation | Continuous gauge, metric, scale, preprocessing, regularization, and tuple-law assumptions are identity-bearing. |
| 8 | Support | Population support is declared; sample ties or uniqueness do not infer it. |
| 9 | Transformation | Quantization, projection, added noise, and feature learning create or condition a route and may change the estimand. |
| 10 | Sampling and dependence | Row identity, episode identity, dependence, schedule, and resampling admissibility are explicit. |
| 11 | Uncertainty | Nominal permutation p-values and surrogate-tail scores remain distinct; FDR accepts only valid p-values. |
| 12 | Numerical assurance | Floating evaluation, rational certification, interval bounds, exact-count certification, conditioning, and regularization are separate statuses. |
| 13 | Resources and determinism | Memory and work components have typed checked bounds; deterministic ordering and failure behavior are required. |
| 14 | Provenance | Paper-defined, paper-derived, project-defined, external-reference, and no-implementation statuses stay orthogonal to maturity. |
| 15 | Software identity | Exact source, feature, build, data, split, and environment identities are bound without being treated as scientific validity. |
| 16 | Evidence ladder | Definition, derivation, implementation, oracle, route-regime, application, and intervention/replication levels are not collapsed. |
| 17 | Application gate | Population, measure, estimator/evaluator, and application gates are independent and fail closed. |
| 18 | Negative controls | Counterexamples, known failures, abstentions, disagreements, and blocked routes are preserved as results rather than erased. |
| 19 | Independence and human ownership | Shared code, fixtures, prompts, reviewers, model families, and assumptions are recorded as common ancestry; AI/council agreement is not independent replication, and the human researcher must reproduce and own every load-bearing result under the applicable disclosure policy. |
| 20 | Ecosystem handshake, publication, and preservation | Prisoma and Galadriel exchange the full result-identity tuple; valid work remains a named research candidate, and promotion requires an exact claim boundary and complete reproducible packet. |

## 8. Material scientific dispositions

The review retained, rather than discarded, valid research directions including higher-source
categorical MGW, exact and interval finite-law assurance, specified rational and binary64 law
evaluation, dependence-aware schedules, continuous tuple/gauge contracts, bivariate BROJA-related
and restricted-Gaussian functionals, deficiency-based quantities, conditional-independence Gaussian
hierarchies, I-PID, infomorphic objectives, and future mixed or conditional definitions.

They remain distinct research programs until a definition, domain, mapping theorem where needed,
route, oracle, and evidence level justify promotion. In particular:

- categorical MGW, Schick-Poland's general construction, Ehrlich's continuous construction and kNN
  estimator, Williams–Beer `I_min`, BROJA, Gaussian-restricted BROJA-related quantities, deficiency
  routes, Lyu–Clark–Raviv hierarchy quantities, and infomorphic objectives are not one PID;
- existing `pid-rs` method IDs remain canonical implementation routes, not one-to-many aliases for
  newly separated functional identities;
- an n=3 inconsistency counterexample rules out a universal family claim but is not an impossibility
  theorem for every fixed source count or for the continuous-only case;
- the raw small-`lambda` deficiency relaxation has a feasible-copy upper bound that tends to zero
  when the relevant conditional mutual information is finite, so a BROJA endpoint requires an
  explicit normalization or limiting theorem rather than a name-based assertion;
- Lyu–Clark–Raviv's N-source construction is a signed information hierarchy and deliberately does
  not supply an N>=3 redundancy atom or a complete nonnegative PID; and
- group bootstrap with replacement duplicates continuous coordinates, so occurrence IDs alone do
  not repair the exact-tie incompatibility of the current continuous route.

These dispositions are method-selection constraints and research opportunities. They are not
claims that Prisoma currently implements every preserved route.

## 9. Reproduction and amendment rule

Any change to the Markdown authority, renderer, font bytes, equation rendering, or PDF requires a
new render, deterministic replay, complete extracted-text checks, all-page image inspection, and a
new receipt. Do not overwrite this receipt to hide an earlier disposition. Preserve it and add a
new dated amendment that identifies what changed and why.

The complete method process remains `PID-M0 preserve` through `PID-M8 publish`. A stopped route is
still a preserved research object, but it cannot enter an application or publication claim above
its achieved evidence level.
