# PID Experiment 0 Findings

## Executive Summary

Experiment 0 tests Partial Information Decomposition (PID) estimators on synthetic data with known ground truth. The results show systematic issues that need to be understood before proceeding to real VLA analysis.

**Status (corrected 2026-07-10): publish two verdicts, not one.**

- **MI/coherence gate: NO-GO** on the high-dimensional nuisance controls. pid-rs 0.4.0
  exposes three invariant-bound violations (for example `r̄≈28.6`, `v̄≈−26.6` on
  `unique_s1_pca`, d=64), so those MI terms cannot support atoms or Shannon invariants.
- **Continuous `I^sx_∩` gate: NOT VALIDATED.** The default Exp0 aggregate tests
  `independent_additive` redundancy against zero even though the implementation and committed
  Gaussian oracle say genuine shared-exclusions redundancy is positive (about 0.2 nats in
  the low-dimensional reference). The strict band avoids that target but gates MI terms and
  only reports atoms. Therefore the binary's printed aggregate **NO-GO** must not be presented
  as a clean `I^sx_∩` verdict. Upstream Exp0 needs separate `MI_GATE`/`ISX_GATE` outputs and a
  measure-specific oracle at each tested dimension.

The operational conclusion is unchanged and stronger: do not interpret continuous atoms on
real embeddings, but state the reason precisely.

**Docset-wide final solution:** `grandplan.md` §A.8 is the decision record. These findings justify the first step of the 10-step plan: keep Exp0/geometry gates strict, then build run-log/replay/Rerun diagnostics before any Tauri/SparkJS product shell or VLA claim.

---

## Observed Results

### NaN Values for Red(disj)

| Scenario | Dimension | Red(disj) | Explanation |
|----------|-----------|-----------|-------------|
| independent_additive | d=10 | NaN | `DisjunctionFromLocalMi` is a heuristic; it can become numerically undefined when pointwise `i(S1,S2;T)` dominates `i(S1;T), i(S2;T)` (log argument ≤ 0). Treat as method failure, not a result. |
| xor_like | d=10 | NaN | Expected method failure: pointwise `i(S1;T)≈i(S2;T)≈0` but `i(S1,S2;T)>0` makes the disjunction log argument ≤ 0. |
| xor_like | d=64,256 | 0.000 | Estimation collapsed entirely |

**Root Cause**: `Red(disj)` here refers to `IsxMethod::DisjunctionFromLocalMi`, a **non-paper-faithful** experimental baseline that computes `log(exp(i1)+exp(i2)−exp(i12))` from KSG local MI terms. This expression is **undefined** when `exp(i12) > exp(i1)+exp(i2)` at any sample, which can occur in strongly synergistic systems (XOR-like) and also via finite-sample noise/bias in other regimes. The implementation returns an error which this report treats as `NaN`.

### Zero MI Estimates at High Dimensions

| Scenario | d=10 | d=64 | d=256 |
|----------|------|------|-------|
| independent_additive I1 | 0.199 | 0.018 | 0.005 |
| redundant_copy I1 | 0.488 | 0.076 | 0.019 |
| xor_like I12 | 0.030 | 0.000 | 0.000 |

**Root Cause**: Curse of dimensionality for kNN-based estimation.

### Constant ID(t) ≈ 1.01 (was ≈ 1.14 under pid-rs 0.3.0)

The target T is always 1-dimensional (scalar). Under pid-rs 0.3.0 the estimate was ≈ 1.14 — the documented ~14% finite-sample bias of uncorrected Levina–Bickel. pid-rs 0.4.0's bias-corrected estimator (the k−2 correction) now reads ID(t) ≈ 1.01, confirming that interpretation: the bias was estimator-side, and correcting it recovers the true dimension almost exactly.

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
| ID(s1,s2) | ≈26–37 (pid-rs 0.4.0 bias-corrected; 28–42 under 0.3.0) | “low” (e.g., < 15) | Data fills high-dimensional space |
| DCcv | 0.12-0.16 | “not too small” (e.g., > 0.3) | Distance concentration occurring |
| d_rel | 0.07-0.09 | no validity threshold | Sampled-metric tree-likeness only; a Euclidean line has δ=0, so this does not invalidate kNN |

---

## Three Hypotheses

Terminology note: the hypotheses in this section are local Experiment 0 diagnostic hypotheses about estimator behavior. They are not the canonical project hypothesis registry (H1–H9), which lives in `grandplan.md` §14.1 and is summarized in `README.md`.

### Hypothesis 1: Estimators Working Correctly
> The near-zero values reflect true signal loss due to noise dimensions swamping kNN distances.

**Evidence For**:
- Gaussian channel sweep shows accurate estimation in 1D
- Decay pattern (d=10 → d=64 → d=256) is smooth, not erratic

**Evidence Against**:
- The TRUE mutual information is non-zero (signal exists in dimension 0)
- Estimation should be biased, not collapsed to zero

**Verdict**: PARTIALLY TRUE - estimators are behaving as expected given the geometry, but "working correctly" is misleading since they're not recovering the true MI.

### Hypothesis 2: Fundamental kNN Limitations in High-D
> Continuous kNN-based PID has fundamental limitations in high dimensions.

**Evidence For**:
- Back-of-envelope uniform coverage in high-d is astronomically large (scales exponentially in `d`; do not treat any single number as exact)
- Distance concentration is mathematically inevitable
- All kNN-based MI estimators share this limitation

**Evidence Against**:
- Real high-D data often has low intrinsic dimension
- Manifold-aware methods might help

**Verdict**: TRUE for data that genuinely fills high-D space. The question is whether VLA embeddings do.

### Hypothesis 3: Projection Should Recover Signal
> Hash/PCA projection to d=64 should preserve enough signal for estimation.

**Evidence For**:
- Johnson-Lindenstrauss guarantees distance preservation
- PCA should capture variance

**Evidence Against**:
- Signal is 1D out of 256D (0.4% of dimensions)
- Signal variance = noise variance (no way to distinguish)
- Projection without supervision is information-destroying

**Verdict**: FALSE as implemented. Would need supervised projection (e.g., project onto directions predictive of T).

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

3. **The "Flow-as-Bridge" strategy is sound**:
   - Object-level flow summaries can be kept low-dimensional by construction (e.g., centroid trajectories / principal flow statistics)
   - Robot proprioception is ~7D (joint angles)
   - These sidestep the high-D problem entirely

---

## Paper-Informed Analysis

### From Gutknecht et al. 2025 (Shannon Invariants)

This paper (arXiv:2504.15779) changes the strategic response to the geometry warnings.

**The "NaN" Root Cause Re-evaluated:** The failures in Exp0 (NaNs, unstable atoms) are partly due to the brittleness of the `I^sx_∩` estimator's geometric requirements (intersection of exclusion balls) in sparse/degenerate data.

**The Solution:** The paper introduces **Average Degree of Redundancy ($\bar{r}$)** and **Vulnerability ($\bar{v}$)**.
*   These are **Shannon Invariants**: they depend *only* on Mutual Information terms ($I(S;T)$), not on specific PID atom definitions.
*   **Implication:** Shannon invariants avoid choosing a redundancy functional, but they do
    not avoid estimator validation. Every constituent marginal and joint MI term—including
    its product-space metric—must pass a gate before `r̄`/`v̄` is meaningful.

## Experiment 0 Update: Shannon Invariants Results

We implemented $\bar{r}$ and $\bar{v}$ in Exp0 and observed:

1.  **Stability vs atom estimators:** Unlike `Red(disj)` (a non-paper-faithful baseline) which can be numerically undefined, $\bar{r}$ and $\bar{v}$ are defined whenever the estimated joint MI is nonzero. In practice they are often a more stable screening signal — but treat `NaN` (e.g., when joint MI collapses to ~0) as a failure mode, not as “success”.
2.  **Diagnostic Value (Negative Vulnerability):** In the `redundant_copy` case (d=10), we observed $\bar{v} = -1.59$.
    *   Mathematically, $\bar{v} < 0$ implies that the sum of conditional mutual information terms is negative. Since conditional MI must be non-negative, this flags a fundamental estimator inconsistency.
    *   **The Specific Violation:** We observed $I(S_1; T) \approx 0.49$ but $I(S_1, S_2; T) \approx 0.27$.
    *   **Monotonicity Violation:** The estimator claims that **adding a second informative source reduces the total information**. This violates the monotonicity axiom $I(S_1, S_2; T) \ge I(S_1; T)$.
    *   **Root Cause:** The KSG estimator bias scales with dimension. The bias at $d_{joint}=20$ is significantly more negative than at $d_{marginal}=10$, causing the estimated joint MI to collapse below the marginals.
    *   **Action:** Use $\bar{v} < 0$ as a hard "NO-GO" gate. It detects when the "curse of dimensionality" has destroyed the coherence between marginal and joint estimates.

## Strategic Guide: Where to Use Which Method

Based on Exp0 findings (negative vulnerability observed in `redundant_copy` at `n=500`, `d=10` per source; joint `d=20`) and the Shannon-invariants strategy, use the following selection logic (treat it as a decision aid, not a theorem):

### 1. The Method Selection Matrix

| Variables | Effective Dimension ($d$) | Geometry | Risk Status | Recommended Method |
| :--- | :--- | :--- | :--- | :--- |
| **V, L, D** (Raw) | ~4096 | Any | **High risk** (distance concentration; coherence-gate failures are common) | **Do not interpret atoms**; reduce/quantize or use MI-only screening |
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

*   **Scenario C: "Flow-as-Bridge" (Geometric Escape Hatch)**
    *   **Sources:** **Flow summaries** (e.g., object centroid trajectories or principal flow statistics; low‑d by construction), **Proprio** (~7D).
    *   **Method:** **Full Atomic PID ($I^{sx}_{\cap}$)**.
    *   **Why:** Lower *effective* dimension makes kNN estimation more plausible — but still require the Exp0 + coherence gates on this exact representation.

### 3. Manifold & Geometry Selection Guide

When standard Euclidean assumptions fail (distance concentration, hierarchy), select geometry based on data structure:

*   **Euclidean ($\mathbb{R}^n$):**
    *   **Use when:** Data is dense, locally flat, or pre-processed (PCA/Whitening).
    *   **Valid Estimators:** Standard kNN MI; continuous $I^{sx}_{\cap}$ only after Experiment 0 + coherence gates pass on the exact preprocessing pipeline (often only at low effective dimension).

*   **Spherical ($\mathbb{S}^n$):**
    *   **Use when:** Embeddings are cosine-similarity based (e.g., CLIP, SigLIP, normalized vectors).
    *   **Valid Estimators:** Geometry-aware MI estimation (e.g., geodesic-kNN-style approaches; not implemented in this repo — research-gated).
    *   **Avoid:** $I^{sx}_{\cap}$ (volume intersection is ill-defined).

*   **Hyperbolic / Poincaré ($\mathbb{H}^n$):**
    *   **Use when:** Data exhibits strong **hierarchical structure** (tree-like) or exponential volume expansion (e.g., language hierarchies, entailment cones).
    *   **Diagnostics:** Low sampled-mean δ can describe tree-likeness under a declared metric,
        but does not prove hyperbolic curvature or invalidate Euclidean kNN; a Euclidean line
        also has δ=0. Use matched controls and direct estimator recovery.
    *   **Valid Estimators:** Geometry-aware MI estimation (research-gated; not implemented here).
    *   **Avoid:** $I^{sx}_{\cap}$.

*   **Lorentzian ($\mathbb{L}^n$):**
    *   **Use when:** Numerical stability is required for Hyperbolic operations (Poincaré ball is unstable near boundary). Mathematically equivalent to Hyperbolic but better for optimization.

### From Ehrlich et al. 2024 (Continuous I^sx_∩)

High-level takeaway (verify details in the paper/official code): the continuous shared-exclusions estimator is a KSG-style kNN construction validated on low-dimensional synthetic systems. It is not evidence of robustness at VLA embedding scales, and it requires careful preprocessing/standardization choices (especially under L∞/Chebyshev geometry).

### From Kraskov et al. 2004 (KSG Estimator)

High-level takeaway (verify exact statements in the paper): KSG MI exhibits a bias/variance tradeoff as a function of `k` and sample size `N`, and can fail in strong-dependence or high-dimensional regimes.

**The distance concentration problem is fundamental** under the stated iid-like/isotropic
high-dimensional conditions, but not unconditional. Geometry diagnostics are correlates and
warnings; only held-out recovery controls establish whether an estimator regime works.

### From grandplan.md (Project Strategy)

The project anticipated this issue:
> "H8: Geometry gate metrics predict a valid estimator regime"

ID, concentration, ties, and dependence help explain risk; sampled-mean `d_rel` is
descriptive. None detects failure by itself. The observed high-d MI/coherence violations are
the direct NO-GO evidence; the atom gate remains unvalidated as explained above.

**The escape hatch is H7 ("Flow-as-Bridge")**:
> "3D Object Flow as Embodiment-Agnostic Integration Diagnostic"

By using low-dimensional **flow summaries** (and other low‑d physical targets) instead of high‑d embeddings as the target `T`, the project can often avoid the worst high‑d kNN pathologies — but still requires Exp0 + coherence gates on the exact representation.

---

## Final Verdict on Hypotheses

### Hypothesis 1: Estimators Working Correctly
**VERDICT: NOT A CORRECTNESS CLAIM; FAILURE MECHANISM SUPPORTED**

The observed behavior is consistent with known kNN-MI pathologies under high ambient/intrinsic dimension: near-zero or unstable estimates can occur even when the true MI is non-zero, because neighborhood geometry is dominated by nuisance dimensions.

The estimator fails its recovery objective in this regime. The collapse is explainable by
known neighborhood pathologies; that explanation does not turn the estimate into a valid one.

### Hypothesis 2: Fundamental kNN Limitations in High-D
**VERDICT: TRUE (for genuinely high-dimensional/noisy regimes)**

This failure mode is expected in regimes where data genuinely fills a high-dimensional space, and it is consistent with the qualitative limitations discussed in the kNN-MI literature and with the fact that continuous `I^sx_∩` validation is limited to low-dimensional synthetic systems (see `grandplan.md` for the project’s citation-policy boundaries).

This is a regime limitation, not proof that every high-ambient-dimensional representation
fails. The response is to:
1. Use direct recovery/coherence gates, with geometry diagnostics as supporting evidence
2. Use low-d targets (flow summaries) when possible
3. Use supervised dimensionality reduction when high-d sources are unavoidable

### Hypothesis 3: Projection Should Recover Signal
**VERDICT: FALSE (as implemented)**

In this isotropic equal-variance construction, the tested random projection and PCA baselines
have no target signal with which to identify the distinguished coordinate. This result is not
a theorem about every unsupervised method or structured distribution.

**What would work**: Supervised projection methods that use label information to find informative subspaces:
- Linear discriminant analysis (LDA)
- Partial least squares (PLS) — **now implemented** in `pid-rs/crates/pid-core/src/pls.rs` (NIPALS-PLS2 algorithm; test confirms signal recovery in signal-in-noise setting)
- Projection onto directions maximizing I(projected;T)

PLS is an implemented candidate, not a free fix: fit it on disjoint training data, freeze the
transform, test on held-out data, and include shuffled-target selection controls.

---

## Recommended Actions

1. **DO NOT** interpret continuous kNN PID atoms outside a validated regime (Exp0 + coherence gates).
2. **DO** prefer low‑d targets (H7 Flow‑as‑Bridge via flow summaries / physical state) when possible.
3. **CONSIDER** supervised projection if high-d sources are required, as a new frozen and
   leakage-controlled regime. Discrete `I_min` modes are also wired, but the emitted
   saturation warning is currently advisory rather than a strict fail-closed gate, so they
   cannot be the active scientific regime until that gap is closed.
4. **DO** treat geometry/coherence warnings as stop signals for atom-level conclusions.
5. **CONSIDER** validating on real VLA embeddings to measure intrinsic dimension and distance concentration before committing to a pipeline.

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
This destroys the discriminative power of nearest-neighbor methods.

---

*Last updated: 2026-07-10 (docset v10.7 corrective audit — MI/coherence NO-GO separated
from `I^sx` NOT VALIDATED; nuisance-dimension atom invariance and δ validity-gate claims
withdrawn)*
*Based on analysis of exp0.rs, experimental output, and implementation of PLS + discrete PID (now wired into the offline harness with saturation diagnostics)*
