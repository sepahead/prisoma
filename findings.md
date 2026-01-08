# PID Experiment 0 Findings

## Executive Summary

Experiment 0 tests Partial Information Decomposition (PID) estimators on synthetic data with known ground truth. The results show systematic issues that need to be understood before proceeding to real VLA analysis.

**Status**: PIVOT (2/3 zero-redundancy checks passed, 3/3 geometry warnings)

---

## Observed Results

### NaN Values for Red(disj)

| Scenario | Dimension | Red(disj) | Explanation |
|----------|-----------|-----------|-------------|
| independent_additive | d=10 | NaN | Expected - marginal MIs near zero |
| xor_like | d=10 | NaN | Expected - pure synergy, no redundancy to measure |
| xor_like | d=64,256 | 0.000 | Estimation collapsed entirely |

**Root Cause**: The `DisjunctionFromLocalMi` estimator requires non-zero local MI structure. When I(S1;T) ≈ 0 or I(S2;T) ≈ 0, the estimator returns an error (converted to NaN by design).

### Zero MI Estimates at High Dimensions

| Scenario | d=10 | d=64 | d=256 |
|----------|------|------|-------|
| independent_additive I1 | 0.199 | 0.018 | 0.005 |
| redundant_copy I1 | 0.488 | 0.076 | 0.019 |
| xor_like I12 | 0.030 | 0.000 | 0.000 |

**Root Cause**: Curse of dimensionality for kNN-based estimation.

### Constant ID(t) ≈ 1.14

The target T is always 1-dimensional (scalar). The ~14% overestimate is typical finite-sample bias in Levina-Bickel ID estimation.

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
- Chebyshev metric (max over dimensions) is especially vulnerable

### Why Projection Baselines Failed

| Method | Why It Failed |
|--------|---------------|
| Hash projection (256→64) | Random projection preserves distances but doesn't isolate signal |
| PCA projection (256→64) | Unsupervised; signal variance ≈ noise variance per dimension |

**Key insight**: Both projections are unsupervised. The signal dimension has variance σ²=1, identical to each noise dimension. PCA cannot distinguish signal from noise without label information.

### Geometry Diagnostics Interpretation

| Metric | Observed | Healthy Range | Interpretation |
|--------|----------|---------------|----------------|
| ID(s1,s2) | 28-42 | < 15 | Data fills high-dimensional space |
| DCcv | 0.12-0.16 | > 0.3 | Distance concentration occurring |
| d_rel | 0.07-0.09 | > 0.15 | Tree-like/concentrated geometry |

---

## Three Hypotheses

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
- With n=500, d=256: would need ~10^77 samples for uniform coverage
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

The experiment tests the wrong thing. It conflates:
1. **kNN estimation fidelity** (can we estimate MI given good geometry?)
2. **Signal discovery** (can we find signal hidden in noise?)

These require different solutions.

### Recommended Path Forward

1. **For estimator validation**: Generate S1, S2 with full-rank signal (all dimensions informative)

2. **For real VLA application**:
   - Use low-dimensional physical targets (3D flow, 6D pose)
   - Or use supervised projection before PID estimation
   - Or use representation learning to find informative subspaces first

3. **The "Flow-as-Bridge" strategy is sound**:
   - Object flow is 3D (well within kNN estimator capabilities)
   - Robot proprioception is ~7D (joint angles)
   - These sidestep the high-D problem entirely

---

## Paper-Informed Analysis

### From Ehrlich et al. 2024 (Continuous I^sx_∩)

The paper explicitly states:
> "We require variables on a comparable scale"

This preprocessing requirement is critical. The exp0 experiment violates this implicitly - the signal dimension has deterministic structure while noise dimensions are pure iid Gaussian. The kNN search doesn't know which dimension matters.

**Key validation from paper**: Table I shows the estimator works well for:
- Redundant gate: I_∩ = 0.35 (expected 0.35)
- Copy gate: I_∩ = 0.69 (expected 0.69)
- Unique gate: I_∩ = 0.00 (expected 0.00)

But these are all **low-dimensional** (d ≤ 3). The paper does not demonstrate high-d performance.

### From Kraskov et al. 2004 (KSG Estimator)

Critical quote on estimation properties:
> "Systematic errors [...] scale roughly as k/N"
> "Statistical errors scale as 1/√N"

With n=500 samples and k≈6-10 neighbors, systematic bias is ~1-2%. This explains why low-d estimates are accurate.

**The distance concentration problem is fundamental**: In high dimensions, kNN distances become concentrated around the mean, destroying discriminative power. This is mathematical reality, not an implementation bug.

### From grandplan.md (Project Strategy)

The project anticipated this issue:
> "H8: Geometry gate metrics predict a valid estimator regime"

The geometry diagnostics (ID, DCcv, d_rel) are designed to **detect** when the estimator will fail - not to fix it. The exp0 "PIVOT" status is the geometry gate working as designed.

**The escape hatch is H7 ("Flow-as-Bridge")**:
> "3D Object Flow as Embodiment-Agnostic Integration Diagnostic"

By using 3D flow targets instead of high-d embeddings as the target T, the project sidesteps the curse of dimensionality entirely.

---

## Final Verdict on Hypotheses

### Hypothesis 1: Estimators Working Correctly
**VERDICT: TRUE (with caveat)**

The estimators are mathematically correct and behave exactly as the theory predicts. The near-zero estimates at high-d are not bugs - they reflect the fundamental inability of kNN methods to find signal in high-dimensional noise. The estimators are "working correctly" in the sense that they correctly return "I cannot reliably estimate this."

**Caveat**: "Working correctly" doesn't mean "returning the true MI." The true MI is non-zero, but the estimators cannot recover it given the geometry.

### Hypothesis 2: Fundamental kNN Limitations in High-D
**VERDICT: TRUE (unambiguously)**

Both papers confirm this. Kraskov 2004 acknowledges the curse of dimensionality. Ehrlich 2024 only validates on low-d examples. The grandplan explicitly states the estimator was "validated on ~100 dimensions" but VLAs use 4096+.

This is not a bug to fix - it's a fundamental limitation of the approach. The response is to:
1. Use geometry gates to detect failure modes
2. Use low-d targets (3D flow) when possible
3. Use supervised dimensionality reduction when high-d sources are unavoidable

### Hypothesis 3: Projection Should Recover Signal
**VERDICT: FALSE (as implemented)**

Random projection and PCA are **unsupervised** methods. They preserve geometric properties (distances, variance) but cannot identify which dimensions carry task-relevant information.

**What would work**: Supervised projection methods that use label information to find informative subspaces:
- Linear discriminant analysis (LDA)
- Partial least squares (PLS)
- Projection onto directions maximizing I(projected;T)

The current hash/PCA baselines are the wrong tool for this job.

---

## Recommended Actions

1. **DO NOT** modify the estimator - it is working correctly
2. **DO** proceed with low-d targets (H7 Flow-as-Bridge)
3. **DO** implement supervised projection if high-d sources are required
4. **DO** treat geometry gate warnings as authoritative stop signals
5. **CONSIDER** validating on real VLA data to measure actual intrinsic dimension

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
In high dimensions, for random points:
```
max_distance / min_distance → 1 as d → ∞
```
This destroys the discriminative power of nearest-neighbor methods.

---

*Document created: 2025-01-08*
*Based on analysis of exp0.rs and experimental output*
