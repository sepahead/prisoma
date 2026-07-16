# PID Experiment 0 Findings

## Executive Summary

Experiment 0 tests Partial Information Decomposition (PID) estimators on synthetic data with known
analytic MI/coherence targets and selected low-dimensional atom/reference fixtures. It does not
supply matching ground truth for every reported PID atom. The results show systematic issues that
need to be understood before proceeding to real VLA analysis.

**Status: publish separate gate verdicts, not one.** These map onto the four PID validity
gates in `grandplan.md` §7.1 — population, measure, estimator, and **application**.

- **MI/coherence (estimator) gate: NO-GO** for the exact high-dimensional nuisance-control
  regimes tested by the exact `pid-rs` 0.9.0 post-tag review-source pin at `796c11e`. The default
  binary run covers 12 scenario–dimension cells over three deterministic seeds (36 case results)
  and has nine MI monotonicity violations and three normalized-invariant bound violations
  (for example, `redundant_copy`, d=10 estimates `I(S1,S2;T)=0.272` below
  `I(S1;T)=0.488`, with `r̄≈3.59`, `v̄≈−1.59`). Those estimated MI terms cannot support atoms or
  Shannon invariants in those regimes. This does not assign the population or measure gate for
  every other estimand/configuration tuple.
- **Continuous `I^sx_∩` application gate: NOT APPLICATION-VALIDATED (blocked).** The default
  Experiment 0 sweep never compares shared-exclusions redundancy with a zero target. It reports
  the atom-measure verdict as `not_adjudicated` because no matching shared-exclusions oracle ran,
  and the atom-estimator verdict as `blocked` because measure and application validation remain
  unresolved. The strict path gates only the curated low-dimensional analytic-MI band—three cases,
  zero recovery failures, **GO**—and reports atoms separately. Therefore neither the default
  MI/coherence **NO-GO** nor the strict analytic-MI **GO** is an `I^sx_∩` atom-validity verdict.

The operational conclusion is unchanged and stronger: do not interpret continuous atoms on
real embeddings, but state the reason precisely.

**Docset-wide final solution:** `grandplan.md` §16 is the decision record and §5.1 is the S0–S7
gate sequence. These findings justify a fail-closed Experiment 0 estimator gate (S1). Geometry
statistics remain descriptive warnings unless a versioned support envelope has calibrated them
against held-out recovery; they are not independent proof of validity or failure. Build the EC1
provenance-complete run-log/replay/Rerun substrate before any Tauri/SparkJS product shell or
high-dimensional VLA atom claim.

---

## Update (2026-07-16): `pid-rs` 0.9 review source and the binary-`L` support mismatch

The submodule now pins the exact `pid-rs` **0.9.0 post-tag review source** (`796c11e`). This review
surface makes continuous support **declared, never inferred**, and fails closed when a tuple falls
outside it; it makes no 1.x compatibility or published-wheel promise. Running the migrated harness
surfaced a defect that pid-rs 0.4 had been hiding:

**`crates/pid-sim/fixtures/offline_vlda_fixture.json` has a binary `L`** — exactly two values,
`{-1.0, +1.0}`, eight samples each. Every continuous `(V,L)→A` and `(L,D)→A` screen, and `MI(L;A)`,
had therefore been running an absolutely-continuous KSG/`I^sx_∩` estimator over a two-valued
variable. pid-rs 0.4 returned numbers for it; the current review source refuses
(`ambiguous k-th-neighbor shell`).

This is a **corrected scientific status, not a regression in estimator capability**. Those estimates
were outside the estimator's valid support contract all along. They are now reported as structured
abstentions with stable reason codes, and the harness produces no numeric placeholder for them.

Two further constraints surfaced the same way, and are reported honestly rather than worked around:

- The pinned two-source continuous `IsxMethod::EhrlichKsg` implementation requires **equal
  ambient source column counts** as a necessary common-radius scaling guard. Therefore `V` (2-d)
  paired with `D` (1-d) is ineligible for that estimator on this fixture
  (`estimator_requires_equal_source_dimensions`). This is estimator-specific, not a theorem that
  all continuous PID measures require equal-dimensional sources, and equality would not by itself
  validate the tuple.
- The binary `L` axis is degenerate for the geometry diagnostics too (duplicate rows give a zero
  nearest-neighbour distance), so its intrinsic-dimension and distance-concentration diagnostics now
  fail closed and record a reason instead of emitting a number.

Note the direction of the inference: exact ties and low observed cardinality reject a **sample** for
the continuous estimator. They do **not** prove the population law is discrete. Support is declared
by the capture adapter; it is never read off the data.

The binary-default `exp0` summary is scoped explicitly: 36 case results from 12
scenario–dimension cells over three deterministic seeds, nine geometry warnings, zero geometry
abstentions, nine monotonicity violations, three normalized-invariant bound violations, and
`MI/Coherence Verdict: NO-GO`. The `just exp0-runlog` recipe deliberately passes one seed, so its
corresponding counts are 12, three, zero, three, and one. Both separately report atom-measure
validation as `not_adjudicated` and atom-estimator validation as `blocked`. No unavailable
estimate carries a numeric placeholder or metric event.

---

## Observed Results

### Failed `Red(disj)` computations (no numeric estimate)

| Scenario | Dimension | Typed outcome | Explanation |
|----------|-----------|-----------|-------------|
| independent_additive | d=10 | `abstained` (raw diagnostic marker: `NaN`) | `DisjunctionFromLocalMi` is a heuristic; it can become numerically undefined when pointwise `i(S1,S2;T)` dominates `i(S1;T), i(S2;T)` (log argument ≤ 0). This is method failure, not a result. |
| xor_like | d=10 | `abstained` (raw diagnostic marker: `NaN`) | Expected method failure: pointwise `i(S1;T)≈i(S2;T)≈0` but `i(S1,S2;T)>0` makes the disjunction log argument ≤ 0. |
| xor_like | d=64,256 | produced value rounding to `0.000` | The finite-sample estimate collapsed; this is not a zero-population-signal finding. |

**Root Cause**: `Red(disj)` here refers to `IsxMethod::DisjunctionFromLocalMi`, a
**non-paper-faithful** experimental baseline that computes
`log(exp(i1)+exp(i2)−exp(i12))` from KSG local MI terms. This expression is undefined when
`exp(i12) > exp(i1)+exp(i2)` at any sample, which can occur in strongly synergistic systems
(XOR-like) and also via finite-sample noise/bias in other regimes. The raw `exp0` diagnostic
currently maps that internal error to a `NaN` display sentinel. It is not an estimate: any typed
application-facing outcome must be `abstained` with a reason code and **no numeric field**.

### Near-zero MI estimates despite preserved population MI

| Scenario | d=10 | d=64 | d=256 |
|----------|------|------|-------|
| independent_additive I1 | 0.199 | 0.018 | 0.005 |
| redundant_copy I1 | 0.488 | 0.076 | 0.019 |
| xor_like I12 | 0.030 | 0.000 | 0.000 |

**Interpretation**: in these finite-sample, nuisance-rich Euclidean/Chebyshev regimes, the tested
KSG configuration fails to recover MI that the data-generating law preserves. Distance
concentration and dimension-dependent neighborhood bias are supported mechanisms; the table is
not evidence that the population signal disappeared, nor a universal claim about all kNN
estimators or all high-ambient-dimensional distributions.

### Current ID(t) estimate ≈ 1.01 (historically ≈ 1.14 under pid-rs 0.3.0)

The target T is scalar. Under the historical pid-rs 0.3.0 implementation the estimate was ≈1.14.
The bias correction introduced in pid-rs 0.4.0 (the k−2 correction) is retained by the current
0.9.0 review-source pin and yields ≈1.01 on this fixture. That is close estimated recovery of the known
one-dimensional construction and is consistent with the proposed finite-sample-bias explanation;
it is not a population-level proof about the diagnostic on other distributions.

---

## First-Principles Analysis

### The Data Generation Design

```
For all scenarios:
- S1, S2 ∈ ℝ^(n×d) where only column 0 carries signal
- Columns 1 through d-1 are iid N(0,1) noise
- T ∈ ℝ^n is a scalar function of S1[:,0] and/or S2[:,0]
```

This creates an **adversarial setting** for kNN estimation:
- Signal lives in a 1-D subspace of ℝ^d
- kNN distances are dominated by the (d-1) noise dimensions
- Chebyshev distances are dominated by nuisance-coordinate maxima

Independent nuisance coordinates preserve the relevant MI terms, but they do **not**
generally preserve continuous shared-exclusions redundancy because its source-density
weights change. Atom drift across d therefore mixes a change in the mathematical functional
with estimator error unless a dimension-specific `I^sx_∩` oracle is supplied.

### Why Projection Baselines Failed

| Method | Why It Failed |
|--------|---------------|
| Hash projection (256→64) | Approximately preserves some pairwise geometry under its JL-style regime, but has no target signal with which to identify the distinguished coordinate |
| PCA projection (256→64) | Unsupervised; signal variance ≈ noise variance per dimension |

**Key insight**: Both projections are unsupervised. The signal dimension has variance σ²=1, identical to each noise dimension. PCA cannot distinguish signal from noise without label information.

### Geometry Diagnostics Interpretation

| Metric | Observed | Heuristic flag (rule-of-thumb) | Interpretation |
|--------|----------|---------------|----------------|
| ID(s1,s2) | ≈25–37 (current 0.9.0 review-source pin; bias correction introduced in 0.4.0), versus 28–42 under historical 0.3.0 | “low” (e.g., < 15) | Sample is consistent with high effective dimension under this diagnostic; not a population proof |
| DCcv | 0.12-0.16 | “not too small” (e.g., > 0.3) | Low empirical distance CV; descriptive warning, not a calibrated validity verdict |
| d_rel | 0.07-0.09 | no validity threshold | Sampled-metric tree-likeness only; a Euclidean line has δ=0, so this does not invalidate kNN |

---

## Three Hypotheses

Terminology note: the hypotheses in this section are local Experiment 0 diagnostic hypotheses about estimator behavior. They are not the canonical confirmatory claim registry (EC1, H1–H4), which lives in `grandplan.md` §4 and is summarized in `README.md`.

### Hypothesis 1: Recovery failure under nuisance dimensions
> The near-zero estimates are a finite-sample recovery failure caused by nuisance dimensions,
> despite non-zero population MI.

**Evidence For**:
- The low-dimensional Gaussian control recovers its analytic MI within the curated tolerance
- Estimated MI decays as nuisance dimension increases while the data-generating population MI is
  unchanged
- Marginal/joint estimates violate required information inequalities in the failed regimes

**Evidence Against**:
- The current sweep does not isolate one mechanism among distance concentration, neighborhood
  bias, and other configuration-specific finite-sample effects
- Smooth decay by itself would not establish the causal mechanism

**Verdict**: SUPPORTED AS A RECOVERY-FAILURE DESCRIPTION FOR THE TESTED REGIMES. It is not a
claim that the estimator is "working correctly," that population information was lost, or that
the same mechanism dominates every distribution.

### Hypothesis 2: Regime-limited kNN recovery at high effective dimension
> The tested continuous kNN MI/PID configuration has severe finite-sample recovery limitations
> when nuisance-rich data fills a high-dimensional neighborhood geometry.

**Evidence For**:
- Back-of-envelope uniform coverage in high-d is astronomically large (scales exponentially in `d`; do not treat any single number as exact)
- Distance concentration follows under the iid-like/isotropic conditions used by the synthetic
  nuisance controls
- The current KSG configuration shows direct analytic-recovery and information-coherence failures

**Evidence Against**:
- Real high-D data often has low intrinsic dimension
- Alternative metrics, estimators, and manifold-aware methods define separate regimes that have
  not been tested here
- Ambient dimension alone does not determine nearest-neighbor consistency or finite-sample error

**Verdict**: SUPPORTED FOR THE TESTED NUISANCE-RICH REGIMES; THE UNIVERSAL VERSION IS REJECTED.
Whether a VLA representation lies in a covered failure regime remains an application-gate
question.

### Hypothesis 3: Projection Should Recover Signal
> Hash/PCA projection to d=64 should preserve enough signal for estimation.

**Evidence For**:
- A Johnson–Lindenstrauss map can preserve pairwise distances within its stated random-projection
  conditions
- PCA preserves directions selected by marginal variance

**Evidence Against**:
- Signal is 1D out of 256D (0.4% of dimensions)
- Signal variance equals each nuisance-coordinate variance, so the population PCA criterion does
  not identify the distinguished coordinate
- Neither tested transform optimizes target-conditioned recovery; compression can dilute or omit
  the predictive direction

**Verdict**: FALSE for the implemented transforms and fixture. Johnson–Lindenstrauss distance
preservation is not a guarantee of MI-estimator recovery, and this result is not a theorem about
all unsupervised projections. A supervised, leakage-controlled projection is a candidate separate
regime, not a guaranteed repair.

---

## Implications for VLA Analysis

### The Core Problem

The current aggregate conflates:
1. **kNN estimation fidelity** (can we estimate MI given good geometry?)
2. **Signal discovery** (can we find signal hidden in noise?)
3. **Functional sensitivity** (continuous `I^sx_∩` itself can change after adding source noise)

These require different solutions.

### Recommended Path Forward

1. **For estimator validation**: keep analytic MI controls, add the committed low-d Gaussian
   `I^sx_∩` oracle plus a pinned independent `csxpid` fixture, and use a dimension-specific
   atom reference rather than assuming invariance

2. **For real VLA application**:
   - Use low-dimensional physical targets (3D flow, 6D pose)
   - Or use supervised projection before PID estimation
   - Or use representation learning to find informative subspaces first

3. **"Flow-as-Bridge" is a target-side candidate, not a validated geometric escape**:
   - Object-level flow summaries can be kept low-dimensional by construction (e.g., centroid trajectories / principal flow statistics)
   - Robot proprioception is ~7D (joint angles)
   - A low-dimensional flow target reduces target-side burden but does not remove high-dimensional
     `V`/`D` source neighborhoods or their joint source–target product geometry; the complete tuple
     still needs all four gates

---

## Paper-Informed Analysis

### From Gutknecht et al. 2025 (Shannon Invariants)

This paper (arXiv:2504.15779) changes the strategic response to the geometry warnings.

**Failed-computation cause re-evaluated:** Some Exp0 computations that the raw diagnostic marks
with `NaN`, plus some unstable produced atoms, are consistent with the restricted estimator's
neighborhood/intersection requirements failing in sparse or degenerate samples. A raw `NaN`
sentinel is never a scientific result; a typed report must abstain with no numeric placeholder.

**An alternative diagnostic:** The paper introduces **Average Degree of Redundancy
($\bar{r}$)** and **Vulnerability ($\bar{v}$)**.
*   These are **Shannon Invariants**: they depend *only* on Mutual Information terms ($I(S;T)$), not on specific PID atom definitions.
*   **Implication:** Shannon invariants avoid choosing a redundancy functional, but they do
    not avoid estimator validation. Every constituent marginal and joint MI term—including
    its product-space metric—must pass a gate before `r̄`/`v̄` is meaningful.

## Experiment 0 Update: Shannon Invariants Results

We implemented $\bar{r}$ and $\bar{v}$ in Exp0 and observed:

1.  **Stability vs atom estimators:** Unlike `Red(disj)` (a non-paper-faithful baseline), which
    can be numerically undefined, $\bar{r}$ and $\bar{v}$ have a value only when their estimated
    joint-MI denominator is resolved and nonzero. If that prerequisite fails, record a typed
    abstention/unresolved outcome and no numeric value; do not promote an internal `NaN` sentinel
    to a result. In covered regimes they may be useful screening diagnostics, but that is not an
    estimator-validation claim.
2.  **Diagnostic Value (Negative Vulnerability):** In the `redundant_copy` case (d=10), we observed $\bar{v} = -1.59$.
    *   Given the positive estimated joint-MI denominator in this case, $\bar{v} < 0$ implies
        that the corresponding sum of estimated conditional-MI terms is negative. Population
        conditional MI is non-negative, so this is an information-coherence violation for the
        estimate tuple.
    *   **The Specific Violation:** We observed $I(S_1; T) \approx 0.49$ but $I(S_1, S_2; T) \approx 0.27$.
    *   **Monotonicity Violation:** The estimator claims that **adding a second informative source reduces the total information**. This violates the monotonicity axiom $I(S_1, S_2; T) \ge I(S_1; T)$.
    *   **Supported mechanism, not uniquely identified cause:** the joint estimate uses a
        higher-dimensional product space than either marginal, and its finite-sample bias is more
        negative in this case. That pattern is consistent with dimension-dependent KSG bias, but
        this sweep does not isolate it from every other configuration-specific effect.
    *   **Action:** Use $\bar{v} < 0$ as a hard NO-GO for the exact MI estimator/configuration
        tuple that produced it: the required information coherence has failed. This diagnoses the
        estimate tuple, not the population law and not every kNN regime.

## Strategic Guide: Where to Use Which Method

Based on Exp0 findings (negative vulnerability observed in `redundant_copy` at `n=500`, `d=10` per source; joint `d=20`) and the Shannon-invariants strategy, use the following selection logic (treat it as a decision aid, not a theorem):

### 1. The Method Selection Matrix

| Variables | Dimension status | Geometry | Risk Status | Recommended Method |
| :--- | :--- | :--- | :--- | :--- |
| **V, L, D** (Raw) | architecture-dependent ambient width; effective dimension unmeasured | unknown until profiled | **High risk** (current synthetic nuisance controls show concentration/coherence failures; real-representation frequency is unassessed) | **Do not interpret atoms**; profile first, then define and validate any reduced/quantized or MI-only regime separately |
| **V, L, D** (Reduced) | measured, not assumed | candidate Euclidean chart | Bias risk | MI/Shannon invariants only if every constituent MI passes `MI_GATE` |
| **A, Flow summaries, Proprio** | often single-digit to low-tens | validate | Lower, not zero, risk | Atomic PID only after both `MI_GATE` and `ISX_GATE` pass on the exact pipeline |
| **Possible manifolds** | measured | unknown until calibrated | Geometry/model risk | No default; compare separately validated MI pipelines, and make no atom claim without a measure-specific derivation/oracle |

### 2. Applied V-L-A-D Scenarios

*   **Scenario A: V-L-A (Vision-Language Alignment)**
    *   **Sources:** $V_{red}$ (PCA/SAE $\to$ 20d), $L_{red}$ (PCA/SAE $\to$ 20d).
    *   **Method candidate:** Shannon invariants, only after all MI terms validate.
    *   **Goal:** screen additive/redundancy–synergy balance. High `r̄` does not imply good
        grounding without external targets and interventions.

*   **Scenario B: V-D-A (World Model Consistency)**
    *   **Sources:** $V_{red}$ (20d), $D_{red}$ (20d).
    *   **Method:** **Shannon Invariants ($\bar{r}$)**.
    *   **Goal:** $\bar{r} \approx 1$ means *additive* MI, which is **consistent with** the policy ignoring the Dream state (or V) — but additivity can also arise from Red ≈ Syn cancellation, so confirm with interventions before concluding "ignored".

*   **Scenario C: "Flow-as-Bridge" (target-side reduction candidate)**
    *   **Sources:** **Flow summaries** (e.g., object centroid trajectories or principal flow statistics; low‑d by construction), **Proprio** (~7D).
    *   **Method candidate:** Full atomic PID ($I^{sx}_{\cap}$), only if the exact population,
        measure, estimator, and application gates pass.
    *   **Why:** Lower effective source and target dimensions can make kNN estimation more
        plausible. A low-dimensional target alone is insufficient when another source or the
        product space remains high-dimensional.

### 3. Manifold & Geometry Selection Guide

When standard Euclidean assumptions fail (distance concentration, hierarchy), select geometry based on data structure:

*   **Euclidean ($\mathbb{R}^n$):**
    *   **Use when:** Data is dense, locally flat, or pre-processed (PCA/Whitening).
    *   **Valid Estimators:** Standard kNN MI; continuous $I^{sx}_{\cap}$ only after Experiment 0 + coherence gates pass on the exact preprocessing pipeline (often only at low effective dimension).

*   **Spherical ($\mathbb{S}^n$):**
    *   **Use when:** Embeddings are cosine-similarity based (e.g., CLIP, SigLIP, normalized vectors).
    *   **Valid Estimators:** Geometry-aware MI estimation (e.g., geodesic-kNN-style approaches; not implemented in this repo — research-gated).
    *   **Shared-exclusions status:** the current Euclidean/Chebyshev $I^{sx}_{\cap}$ derivation
        and implementation are ineligible. This repository has no spherical measure/estimator
        derivation or oracle; that absence is not a universal impossibility theorem.

*   **Hyperbolic / Poincaré ($\mathbb{H}^n$):**
    *   **Use when:** Data exhibits strong **hierarchical structure** (tree-like) or exponential volume expansion (e.g., language hierarchies, entailment cones).
    *   **Diagnostics:** Low sampled-mean δ can describe tree-likeness under a declared metric,
        but does not prove hyperbolic curvature or invalidate Euclidean kNN; a Euclidean line
        also has δ=0. Use matched controls and direct estimator recovery.
    *   **Valid Estimators:** A default-off experimental hyperbolic MI path exists in `pid-rs`,
        but it is a separate research regime and has not cleared the application gate here.
    *   **Shared-exclusions status:** the current Euclidean/Chebyshev $I^{sx}_{\cap}$ derivation
        and implementation are ineligible. A hyperbolic analogue would be a new measure/estimator
        requiring derivation and independent validation; none exists in this repository.

*   **Lorentzian ($\mathbb{L}^n$):**
    *   **Use when:** A separately justified hyperbolic model uses the Lorentz representation.
        It can have different numerical conditioning from a Poincaré-ball implementation, but
        changing representation or metric does not license the current shared-exclusions
        estimator.

### From Ehrlich et al. 2024 (Continuous I^sx_∩)

High-level takeaway (verify details in the paper/official code): the continuous shared-exclusions estimator is a KSG-style kNN construction validated on low-dimensional synthetic systems. It is not evidence of robustness at VLA embedding scales, and it requires careful preprocessing/standardization choices (especially under L∞/Chebyshev geometry).

### From Kraskov et al. 2004 (KSG Estimator)

High-level takeaway (verify exact statements in the paper): KSG MI exhibits a bias/variance tradeoff as a function of `k` and sample size `N`, and can fail in strong-dependence or high-dimensional regimes.

**Distance concentration follows under the stated iid-like/isotropic high-dimensional
conditions**, but the conclusion is not unconditional. Geometry diagnostics are correlates and
warnings; only held-out recovery controls establish whether an estimator regime works.

### From grandplan.md (Project Strategy)

The plan anticipated this (v12.5 §7.9, "Geometry diagnostics are diagnostics, not proofs"):
geometry metrics may flag risk but do not prove estimator validity, and may enter a hard gate
only after they predict oracle-defined estimator validity on held-out synthetic families.

ID, concentration, ties, and dependence help explain risk; sampled-mean `d_rel` is
descriptive. None detects failure by itself. The observed high-d MI/coherence violations are
the direct NO-GO evidence; the continuous application gate remains blocked as explained above.

**Flow-as-a-bridge is a target-side candidate** (v12.5 §9.6, an exploratory
low-dimensional, potentially embodiment-portable target — object/contact flow):

Using low-dimensional **flow summaries** (and other low-dimensional physical targets) can reduce
target-side estimation burden. It does not bypass high-dimensional source neighborhoods or the
joint source–target product geometry, and portability additionally requires standardized frames,
correspondence, visibility, and contact semantics. The exact representation tuple still requires
all four gates.

---

## Final Verdict on Hypotheses

### Hypothesis 1: Estimated recovery failure despite preserved population signal
**VERDICT: SUPPORTED FOR THE TESTED REGIMES; NOT A CORRECTNESS CLAIM**

The data-generating law preserves non-zero MI while the finite-sample estimate approaches zero and
the marginal/joint estimates lose information coherence. This establishes recovery failure for the
tested estimator/configuration tuples. Nuisance-dominated neighborhood geometry is a supported
explanation, not a uniquely identified population mechanism.

The collapse does not mean that the population signal disappeared, and explaining a failed estimate
does not make it valid.

### Hypothesis 2: Regime-limited kNN recovery at high effective dimension
**VERDICT: SUPPORTED IN THE TESTED NUISANCE-RICH REGIMES; UNIVERSAL CLAIM REJECTED**

The observed failure is consistent with qualitative kNN-MI limitations under the synthetic
controls' iid-like/isotropic, high-effective-dimensional conditions and with continuous
`I^sx_∩` validation being limited to named low-dimensional synthetic systems (see
`grandplan.md` for citation-policy boundaries).

It is not proof that every kNN estimator or high-ambient-dimensional representation fails. The
response is to:
1. Use direct recovery/coherence gates, with geometry diagnostics as supporting evidence
2. Use low-d targets (flow summaries) when possible
3. Use supervised dimensionality reduction when high-d sources are unavoidable

### Hypothesis 3: Projection Should Recover Signal
**VERDICT: FALSE (as implemented)**

In this isotropic equal-variance construction, the tested random projection and PCA baselines
have no target signal with which to identify the distinguished coordinate. This result is not
a theorem about every unsupervised method or structured distribution.

**Candidate approaches**: supervised projection methods that use training-only target information
to seek informative subspaces:
- Linear discriminant analysis (LDA)
- Partial least squares (PLS) — **now implemented** in `pid-rs/crates/pid-core/src/pls.rs`
  (NIPALS-PLS2; a unit test demonstrates recovery on one signal-in-noise fixture, not general
  application validity)
- Projection onto directions maximizing I(projected;T)

PLS is an implemented candidate, not a free fix: fit it on disjoint training data, freeze the
transform, test on held-out data, and include shuffled-target selection controls.

---

## Recommended Actions

1. **DO NOT** interpret continuous kNN PID atoms outside a validated regime (Exp0 + coherence gates).
2. **DO** consider low-dimensional targets (flow-as-bridge via flow summaries / physical state;
   exploratory, `grandplan.md` §9.6) as target-side burden reduction, while still validating
   high-dimensional source and product-space geometry.
3. **CONSIDER** supervised projection if high-d sources are required, as a new frozen and
   leakage-controlled regime. Discrete `I_min` modes are also wired, but the emitted
   saturation warning is currently advisory rather than a strict fail-closed gate, so they
   cannot be the active scientific regime until that gap is closed.
4. **DO** treat direct recovery or information-coherence failures as stop signals for the exact
   estimator tuple. Treat geometry diagnostics as warnings/descriptive evidence unless a validated
   support envelope has calibrated an abstention rule; the application gate remains blocked either
   way.
5. **CONSIDER** profiling real VLA embeddings to measure intrinsic dimension and distance
   concentration before committing to a pipeline. Those descriptive measurements do not themselves
   validate an estimator or application regime.

---

## Open Questions

1. What is the intrinsic dimension of real VLA embeddings (e.g., DINO, SigLIP)?
2. Do VLA action spaces have concentrated or dispersed geometry?
3. Can we validate PID estimates against known robotics ground truth?

---

## Appendix: Key Equations

### KSG Mutual Information Estimator
```
I(X;Y) = ψ(k) - ⟨ψ(nx+1) + ψ(ny+1)⟩ + ψ(N)
```
where nx, ny are neighbor counts in marginal spaces at the kth-neighbor distance in joint space.

### Gaussian Channel Ground Truth
```
X ~ N(0,1), Y = X + σZ, Z ~ N(0,1)
I(X;Y) = 0.5 * ln(1 + 1/σ²)
```

### Distance Concentration
In high dimensions, for random points with iid-like/isotropic coordinates (Beyer et al. 1999 conditions — not unconditional; low intrinsic dimension escapes it):
```
max_distance / min_distance → 1 as d → ∞
```
Under those conditions this erodes finite-sample distance discrimination; it is not an
unconditional theorem of failure for every distribution or nearest-neighbor estimator.

---

*Last updated: 2026-07-16 (docset v12.5 — gate verdicts mapped onto the four PID gates
of `grandplan.md` §7.1; MI/coherence estimator gate NO-GO separated from the continuous
`I^sx_∩` application gate BLOCKED / NOT APPLICATION-VALIDATED; nuisance-dimension atom
invariance and δ validity-gate claims withdrawn; current results attributed to the exact 0.9.0
post-tag review-source pin and
historical 0.3/0.4 behavior labelled explicitly)*
*Based on analysis of the current `exp0.rs`, experimental output, and implementation of PLS +
discrete PID (now wired into the offline harness with saturation diagnostics)*
