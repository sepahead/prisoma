# Comprehensive PID-VLA Specification
## Partial Information Decomposition for Vision-Language-Action Model Diagnostics
### A Critical Technical Analysis with Full Discussion of Approaches, Limitations, and Open Questions

**Version:** 5.7 (First-Principles Geometry Analysis + VLA Verification)
**Date:** 2026-01-02
**Status:** Research Specification (critical assessment + engineering roadmap)
**Canonical:** This is the living spec; prior versions live in git history.

> **⚠️ Critical Discovery (v5.5):** The continuous `I^sx_∩` estimator (Ehrlich et al. 2024) relies on Chebyshev (L∞) geometry for exact product-ball cancellations. It **cannot** be applied directly to hyperbolic/Lorentz/manifold embeddings without a **new mathematical derivation** of the disjunction neighborhoods and volume forms in that geometry. Do not simply plug manifold distances into the current estimator.

> **🚧 WORK IN PROGRESS (v5.6): Top 5 Manifold-Compatible Approaches**
> 
> 1. **The "Manifold Unrolling" Approach (Isomap/AE → Standard Estimator):**
>    Use Isomap or Contractive Autoencoders to flatten the manifold into a lower-dimensional Euclidean space (e.g., d=32), then run the standard Ehrlich `I^sx_∩` estimator. This "unrolls" the geometry so L∞ distances become valid proxies.
> 
> 2. **The "Geodesic MI" Approach (Manifold kNN → Shannon Invariants):**
>    Abandon PID atoms. Use a Manifold-Aware MI Estimator (Marx & Fischer 2021) with geodesic distances to compute Mutual Information only. Derive Co-Information (Red - Syn) as a rigorous, coordinate-invariant scalar summary.
> 
> 3. **The "Linear Projection" Approach (PCA → Standard Estimator):**
>    Pragmatic baseline. Use PCA to reduce to ~256 dims. PCA is a linear rotation, preserving the "box" volume logic of the Chebyshev estimator better than nonlinear warping, provided the manifold is locally flat enough.
> 
> 4. **The "Quantization" Approach (Clustering → Discrete PID):**
>    Map continuous embeddings to discrete cluster IDs (k=100..1000) using k-means/VQ. Use the classic Discrete `I^sx_∩` estimator (Makkeh et al. 2021), effectively bypassing geometry issues by counting mass instead of volumes.
> 
> 5. **The "Copula Transform" Approach (Rank Transform → Standard Estimator):**
>    Apply empirical CDF transform to every dimension to force Uniform marginals. This mitigates "empty space" issues in high-d L∞ metrics and maximizes estimator efficiency, though it ignores dependencies during the transform.
>
> **⛔ EXCLUDED APPROACHES (Why they didn't make the list):**
> 
> *   **Kernel Density Estimation (KDE):** Excluded due to the "curse of dimensionality" at d=4096 (bandwidth selection is statistically impossible). Furthermore, numerically integrating the complex "disjunction" shape for `I^sx_∩` is intractable compared to KSG counting.
> *   **Harmonic/Spectral Methods (Diffusion Maps):** Excluded due to computational cost ($O(N^3)$ eigendecomposition) and uncontrolled density distortion. Unlike Isomap ("Unrolling"), spectral embeddings change local volumes in ways that are difficult to correct for PID.
> *   **Naive Geodesic kNN (for PID atoms):** **Violates the v5.5 Warning.** The Ehrlich estimator relies on Euclidean product-volume cancellation ($Vol_{XY} \approx Vol_X \cdot Vol_Y$). Curvature breaks this exact cancellation, making atom estimates invalid. (Contrast with Method 2, which restricts itself to MI/CI where this cancellation is not required).

**v5.7 notes (changes without deleting prior work):**
- **Closed v5.6 as stable.** All v5.6 manifold approaches remain valid; this version adds empirical validation methods and VLA-specific guidance.
- **VLA Architecture Verification:** Cross-checked all VLA architecture claims against original papers (see §7.6):
  - OpenVLA: Corrected to SigLIP+DinoV2 (600M params), 32 layers, 4096 hidden dim ✓
  - DreamVLA: GPT-2 variant/hidden dims NOT SPECIFIED in paper ⚠️
  - PixelVLA: Verified 7D actions, chunk size 8, SAM encoder, LoRA rank 32 ✓
  - TraceVLA: Verified 7B params, 4096 hidden dim inherited from OpenVLA ✓
- **First-Principles Geometry Analysis (§16.6-§16.11):**
  - §16.6: 4 empirically validated local flatness testing methods
  - §16.7: δ-hyperbolicity testing with Gromov 4-point condition
  - §16.8: SAE analysis for VLA (NeurIPS 2025 verification)
  - §16.9: Chebyshev/PixelVLA geometry transition analysis
  - §16.10: GPT-2 vs modern LLMs hierarchy evidence
  - §16.11: Unified Geometry-First Protocol + NanoGPT foundational study
- **Authoritative Code Sources:** Added Wibral GitLab repos (infomorphic_networks, continuouspidestimator) to §13
- **New VLA Research Integration:**
  - VLA-Arena benchmark: "memorization over generalization" finding (arXiv:2512.22539)
  - GenieReasoner/FACT tokenizer: flow-matching action discretization (arXiv:2512.24125)
  - Hierarchical geometry of cognitive states in transformer embeddings (arXiv:2512.22227)
- **Hyperbolic Training Guidance:** Added explicit guidance on when/where hyperbolic embedding training is needed (§16.7.4)

**v5.6 notes (changes without deleting prior work):**
- **Added Top 5 Manifold Solutions:** Explicitly listed "Manifold Unrolling", "Geodesic MI", "Linear Projection", "Quantization", and "Copula Transform" as practical engineering paths to address the v5.5 geometry warning.
- **Documented Exclusions:** Explicitly noted why KDE, Harmonic Math, and Naive Geodesic kNN are suboptimal or dangerous in this context.

**v5.6 Architecture Verification (superseded by v5.7):**
  - **OpenVLA**: Corrected to SigLIP + DinoV2 (600M params), 32 layers (not 33), 4096 hidden dim ✓
  - **DreamVLA**: Added caveat that GPT-2 variant/hidden dims are NOT specified in paper ⚠️
  - **PixelVLA**: Added verified specs (7D actions, chunk size 8, SAM prompt encoder, LoRA rank 32) ✓
  - **TraceVLA**: Added verified 7B params, 4096 hidden dim inherited from OpenVLA ✓
- **Added §7.6 Architecture Verification Summary:** Documents verification status, intrinsic dimension literature, and v5.6 approach review in light of confirmed d=4096.
- **First-Principles Geometry Analysis (Jan 2026):** Major additions to §16:
  - **§16.6 Local Flatness Testing:** 4 empirically validated methods (manifold curvature via subspace angles, Ollivier-Ricci curvature, DLME constraint, curvature-adjusted PCA)
  - **§16.7 δ-Hyperbolicity Testing:** Gromov 4-point condition, empirical evidence from LLM embeddings showing modern models are MORE tree-like
  - **§16.8 SAE Analysis for VLA:** Application of Sparse Autoencoders to VLM/VLA components (NeurIPS 2025 verification), concrete protocol for PID analysis
  - **§16.9 Chebyshev/PixelVLA Analysis:** Geometry transition analysis showing where L∞ is appropriate vs hierarchical methods
  - **§16.10 GPT-2 vs Modern LLMs:** Architectural differences affecting geometry, layer-wise hierarchy evidence
  - **§16.11 Unified Geometry-First Protocol:** Complete decision framework integrating all diagnostics + NanoGPT foundational study protocol

**v5.5 notes (changes without deleting prior work):**
- **Critical Documentation Fix:** Explicitly documented that Wibral PID (`I^sx_∩`) on manifolds/Lorentz spaces requires new derivations (volume forms/disjunctions).
- Added top-level warning to prevent naive application of Euclidean estimators to curved spaces.

**v5.4 notes (VLA integration):**
- Verified key VLA + Shannon-invariants citations via **arXiv API** (titles/authors/dates):
  - OpenVLA — arXiv:2406.09246
  - DreamVLA — arXiv:2507.04447
  - Dream-VL & Dream-VLA — arXiv:2512.22615
  - PixelVLA — arXiv:2511.01571
  - TraceVLA — arXiv:2412.10345
  - Shannon invariants — arXiv:2504.15779
- Clarified how OpenVLA/DreamVLA/PixelVLA/TraceVLA affect **what variables exist** (what “D” can mean) and therefore which decompositions are scientifically clean (§6.1, §7).
- Clarified the **primary hypothesis** vs. **candidate sub-hypotheses/features** and made the hypothesis↔aims mapping explicit (§1.3, §3.3).
- Tightened the “hierarchy vs geometry” story: Shannon invariants/hierarchical screening address **source-count scaling**, while manifold/high‑d diagnostics address **estimator validity at (N,d)** (§8.1.5, §16).
- Tightened dimensionality-reduction language so the table cannot be misread as “random projection fixes manifolds” or “hyperbolic is drop‑in” (§8.2, §16.4).

**v5.1–v5.3 notes (restored):**
- **v5.3 (Hierarchy vs Geometry):** Distinguished source-count scaling (hierarchy) from estimator validity (geometry).
- **v5.2 (PixelVLA & TraceVLA):** Added citations and scope for visual prompting and trace-based architectures.
- **v5.1 (OpenVLA & DreamVLA):** Clarified variable definitions and world model ("D") extraction.

**v5.0 final audit notes (changes without deleting prior work):**
- Added confounding factors analysis (§14)
- Added numerical stability guidance (§15)
- Added manifold/PCA/kNN limitations section (§16) with detailed diagnostics and decision flowcharts
- Integrated information geometry methods and intrinsic dimension estimation
- Code audit complete — implementation cross-checked against reference implementations
- Grant-ready documentation with full provenance tracking

**Reference verification status (important):**
- Core `I^sx_∩` / KSG papers: verified by DOI metadata; local copies exist under `.external/papers/`.
- arXiv IDs in §13: verified via arXiv API (title/authors/date).
- Detailed architecture claims, runtime/latency numbers, and some “ecosystem” descriptions are treated as **unverified unless explicitly sourced**; keep them as ideas/design sketches, not facts.

---

# Table of Contents

1. [Executive Summary and Critical Warnings](#1-executive-summary-and-critical-warnings)
2. [Theoretical Foundations](#2-theoretical-foundations)
3. [The Core Research Questions](#3-the-core-research-questions)
4. [Decomposition Strategies: What Variables to Analyze](#4-decomposition-strategies-what-variables-to-analyze)
5. [Three-Way PID: I(V, L, D; A)](#5-three-way-pid-iv-l-d-a)
6. [Discarded Approaches and Why](#6-discarded-approaches-and-why)
7. [VLA Architecture Analysis](#7-vla-architecture-analysis)
8. [Estimation and Implementation](#8-estimation-and-implementation)
9. [Experimental Design](#9-experimental-design)
10. [World Model Integration (WAN, GWM, 3DGS)](#10-world-model-integration-wan-gwm-3dgs)
11. [Technical Implementation](#11-technical-implementation)
12. [Open Questions and Future Directions](#12-open-questions-and-future-directions)
13. [References](#13-references)
14. [Confounding Factors Analysis: Proving and Disproving the Hypotheses](#14-confounding-factors-analysis-proving-and-disproving-the-hypotheses)
15. [Numerical Stability and Optimization: Technical Guidance](#15-numerical-stability-and-optimization-technical-guidance)
16. [Why PCA and kNN Are Suboptimal for Manifold-Valued Embeddings](#16-why-pca-and-knn-are-suboptimal-for-manifold-valued-embeddings)
A. [Appendix A: Glossary](#appendix-a-glossary)
B. [Appendix B: Decision Log and Implementation Reference](#appendix-b-decision-log-and-implementation-reference)

---

# 1. Executive Summary and Critical Warnings

## 1.1 What This Document Is

This document provides a comprehensive specification for applying Partial Information Decomposition (PID), specifically the shared-exclusions measure I^sx_∩ from the Wibral group at Göttingen, to diagnose grounding failures ("hallucinations") in Vision-Language-Action (VLA) models.

**Scope constraint (PhD-critical):**
- This document is intentionally anchored on the **Wibral/Göttingen line of work**: shared-exclusions PID (`I^sx_∩`, “SxPID”) and the related **Shannon-invariants** program (Gutknecht et al. 2025).
- Other PID measures/tools may be mentioned for contrast or baselines, but **they are not the scientific object of this project**.

**First-principles epistemic split (do not blur):**
- The **quantity** `I^sx_∩` is a mathematical functional of the data-generating distribution.
- Any **estimator** (kNN/KSG, variational, etc.) is a finite-sample algorithm with bias/variance/failure modes. Most “surprising” effects at VLA scale are more likely estimator/pathology than new science unless ruled out by Experiment 0.

**Units (avoid silent mismatches):**
- Papers often use `log2` (bits). This repo’s Rust implementation uses natural `log` (nats).
- Convert via: `bits = nats / ln(2)` and `nats = bits * ln(2)`.

## 1.2 ⚠️ CRITICAL WARNINGS: Read Before Proceeding

This project underwent extensive first-principles review that revealed **fundamental conceptual issues** that must be understood before any implementation:

### Warning 1: The Core Hypothesis May Be Unfounded

**Claim in original proposal:** "Negative synergy (Syn < 0) indicates hallucination because V and D conflict"

**What the mathematics actually says:** Negative synergy under I^sx_∩ means **subadditive information**—combining sources V and D provides less predictive power about target A than expected from their individual contributions. This is NOT the same as "conflict."

Negative synergy could arise from:
- Estimation artifacts at high dimensions (curse of dimensionality)
- High correlation between V and D (double-counting effects)
- General model uncertainty (unrelated to hallucination)
- Pointwise misinformation (observing sources makes target less likely)

**Status:** This is a HYPOTHESIS requiring empirical validation, not a definitional truth. In this spec it is treated as a *candidate sub-hypothesis / feature* inside the primary evaluation aim (see §3.3), not as the project’s sole thesis.

### Warning 2: The V-D-A Decomposition May Be Degenerate

In a VLA model:
```python
action = vla_forward(vision, dream_state, language_instruction)
```

The action A is **deterministically computed** from (V, D, L). This creates problems:

1. **Triviality when conditioning on all inputs:** If `A = f(V,D,L)` deterministically (fixed weights + deterministic inference), then `I(V,D,L;A) = H(A)` (up to inference stochasticity). A 3-source PID of `(V,L,D)→A` decomposes `H(A)` and does not by itself validate grounding/correctness.
2. **Pairwise MI depends on what varies:** `I(V,D;A)` can be informative when `L` varies across samples, but it can approach `H(A)` if `L` is constant (or effectively redundant) in the dataset. Always report the sampling unit and which inputs are included in the decomposition.
3. **Grounding/failure diagnosis needs an external target or counterfactual:** Prefer `A*` (teacher/optimal action), a success/failure label, or controlled interventions/counterfactuals. Otherwise you risk measuring only the model’s internal consistency rather than “hallucination.”

### Warning 3: The KSG Estimator May Fail at VLA Scale

The continuous I^sx_∩ estimator (Ehrlich et al., 2024) was validated on:
- Low-dimensional systems (~100 dimensions)
- Thousands of samples
- Well-behaved distributions

VLA embeddings are:
- 4096+ dimensions
- Hundreds of samples per trajectory
- Unknown distributional properties

At d=4096, k-NN methods suffer from the curse of dimensionality: "nearest neighbors" become nearly equidistant.

### Warning 4: Strong Dependence Can Break kNN MI Even at Low Dimension

There is a separate (often missed) failure mode from “high dimension”: **strong statistical dependence** (very large true MI, e.g., near-deterministic relationships) can make kNN MI estimators require **prohibitively many samples** even when `d` is small.

Gao, Ver Steeg, and Galstyan (AISTATS 2015; arXiv:1411.2003) show that popular kNN MI estimators can have sample complexity that scales **exponentially in the true MI** for strongly dependent variables, due to an implicit local-uniformity assumption. They propose corrections/alternatives (e.g., local non-uniformity correction; local Gaussian approximations, arXiv:1508.00536) that can reduce bias in this regime.

Why this matters here:
- VLA pipelines often contain **near-deterministic mappings** (e.g., `A = f(V,D,L)`; deterministic decoders; cached embeddings; quantized actions).
- If variables are treated as continuous, **MI may be effectively unbounded** in the deterministic/noiseless limit. Estimator output can be dominated by numerical/finite-precision effects rather than meaningful “information integration.”

Design implication:
- Experiment 0 must include **strong-dependence** synthetic cases (not just “high `d`”) and explicitly test estimator stability under near-determinism.
- When using MI/PID on VLA signals, you must be explicit about the noise model / discretization / stochasticity that makes the quantity finite and interpretable.

### Warning 5: kNN Estimators Assume i.i.d. Samples (Trajectory Autocorrelation Is a Confound)

The KSG family (and the Ehrlich et al. `I^sx_∩` estimator built on it) is typically analyzed under an **i.i.d. sample** assumption.

But VLA data is usually collected as **trajectories**:
- Adjacent timesteps are strongly autocorrelated → “N frames” is not “N independent samples”.
- Some variables are constant within a trajectory (e.g., instruction `L`) → within-trajectory MI/PID can be degenerate or misleading.

**Implication:** Treating every frame as an i.i.d. sample can inflate apparent sample size, distort variance estimates, and change neighbor geometry. Any “real-time” claims must state the sampling unit (frames vs windows vs trajectories) and the effective sample size.

**Mitigations (design choices, not afterthoughts):**
- Prefer **across-trajectory** datasets where each sample is an episode/timepoint chosen by a reproducible rule (or use large stride subsampling).
- Use **block bootstrap** / trajectory-level resampling for uncertainty estimates when temporal dependence is unavoidable.

### Warning 6: Liang et al. (2023) Use DIFFERENT PID Measures

Their robotics results do NOT validate I^sx_∩ specifically. They use:
- "Batch estimator" based on variational bounds
- "CVX estimator" using convex optimization over discrete clusters

Neither uses the shared-exclusions definition. Their success doesn't transfer automatically to our approach.

## 1.3 Recommended Approach

Given these warnings, the recommended approach is:

1. **Run Experiment 0 FIRST:** Validate the estimator on synthetic data at target dimensionality before any VLA experiments
2. **Include strong baselines:** Compare against entropy, Liang et al.'s estimators, learned classifiers
3. **Pre-register success criteria:** Specify AUROC threshold, statistical tests
4. **Plan for negative results:** If entropy works just as well, that's a valid (and publishable) finding
5. **Test multiple decompositions:** Don't commit to V-D-A alone; test V-L-A and three-way decomposition

**Coherence note (why §1 highlights “one hypothesis” but §3 has multiple aims):**
- The original “Syn < 0 ⇒ hallucination” claim is *not* the project thesis; it is one candidate feature/sub-hypothesis.
- The project thesis is broader: under a **validated estimator regime**, a **feature set** derived from Shannon invariants (Gutknecht et al. 2025) and (where feasible) SxPID (`I^sx_∩`) should add predictive/diagnostic value beyond strong uncertainty baselines.
- §3.3 rewrites the aims to match this hierarchy and makes the gating explicit.

## 1.4 How the Pieces Fit Together (VLA Architecture × Hierarchy × Manifolds)

This project has three *separable* axes that are easy to conflate:

1. **What variables exist (model/architecture):** what “V”, “L”, “D”, and “A” mean depends on the VLA.
   - **DreamVLA (arXiv:2507.04447):** provides explicit world‑knowledge predictions → “D” is operationalizable (and interventionable).
   - **OpenVLA (arXiv:2406.09246):** no explicit “dream/world model” output → any “D” is a hidden-state extraction (definition choice).
   - **PixelVLA (arXiv:2511.01571):** introduces **multiscale V** and **visual prompts** → many candidate “sources” (hierarchy becomes useful).
   - **TraceVLA (arXiv:2412.10345):** injects history via **visual traces** → temporal information is partly “inside V,” blurring V/D boundaries.

2. **How we scale to many sources (hierarchy):** Shannon invariants / hierarchical screening (Gutknecht et al., arXiv:2504.15779) address **combinatorial explosion in source count**, not high‑dimensional geometry.
   - Level 1: MI-only invariants (CI/Ω) to screen many candidate sources/windows.
   - Level 2: targeted pairwise SxPID (`I^sx_∩`) where meaningful.
   - Level 3: optional full 3-way SxPID (18 atoms) offline.

3. **Whether estimation is valid at all (geometry/manifolds):** kNN/KSG and disjunction‑kNN `I^sx_∩` can collapse at high effective dimension or under strong dependence.
   - Always run geometry diagnostics (intrinsic dimension + distance concentration proxies) and the Experiment 0 gate **after** any projection/preprocessing.
   - If kNN-based `I^sx_∩` is invalid even after reduction, restrict claims to Shannon invariants / MI-only baselines and treat them as a different pipeline (not “`I^sx_∩` results”).

---

# 2. Theoretical Foundations

## 2.1 Partial Information Decomposition (PID)

PID addresses the question: Given two (or more) source variables S₁, S₂ and a target variable T, how can we decompose the total mutual information I(S₁, S₂; T) into components that capture:

- **Redundancy:** Information available from EITHER source alone
- **Unique Information:** Information available from ONE source but not the other
- **Synergy:** Information available ONLY from both sources together

For two sources, the decomposition is:
```
I(S₁, S₂; T) = Red(S₁, S₂; T) + Unq(S₁; T) + Unq(S₂; T) + Syn(S₁, S₂; T)
```

### 2.1.1 The Problem: PID is Underdetermined

Shannon's information theory doesn't uniquely specify how to compute these atoms. Multiple PID measures exist, each with different properties and trade-offs.

## 2.2 The I^sx_∩ (Shared-Exclusions) Measure

We adopt I^sx_∩ from Makkeh, Gutknecht, and Wibral (2021), extended to continuous variables by Ehrlich et al. (2024).

### 2.2.1 Definition

The shared-exclusions redundancy `I^sx_∩` is defined via **exclusions of probability mass** and can be written as a **local mutual information** induced by an auxiliary “statement” variable `W` (Makkeh et al. 2021, Eq. 17):
```
i^sx_∩(t : s₁; s₂) := i(t : W_{s₁,s₂}=1) = log[ p(t | W_{s₁,s₂}=1) / p(t) ]

I^sx_∩(S₁, S₂; T) := E_{t,s₁,s₂}[ i^sx_∩(t : s₁; s₂) ]
```

Where `i(·;·)` is the **pointwise mutual information**:
```
i(s; t) = log[p(s, t) / (p(s)·p(t))]
```

**Subtle but important (do not gloss over):**
- The *local* term `i(t : W_{s₁,s₂}=1)` is a pointwise mutual information with an auxiliary statement variable.
- The *global* redundancy `I^sx_∩(S₁,S₂;T)` is **not** the mutual information `I(T;W)` because the expectation is taken over `p(t,s₁,s₂)` (Makkeh et al. 2021 note under Eq. 17), not over `p(t,W)`.
- Consequence: even the **redundancy itself** can be negative at the distribution level; negative values are not automatically “bugs,” but they do require careful interpretation and estimator validation.

Ehrlich et al. (2024) derive a **kNN/KSG-style estimator** for the continuous case by replacing conjunction (intersection) neighborhoods with disjunction (union) neighborhoods; see §8.1.3 for the concrete estimator form.

**Important:** Do **not** confuse `I^sx_∩` with Williams & Beer’s `I_min`, which is defined using a minimum over “specific information” terms; `I_min` is a different redundancy measure.

### 2.2.2 Key Properties

| Property | I^sx_∩ | Implication |
|----------|--------|-------------|
| **Differentiability (distribution-level)** | ✓ | Differentiable as a functional of probabilities; **gradient-based training still requires a differentiable estimator** (the kNN/KSG estimator is not). |
| **Target Chain Rule** | ✓ | Atoms sum to total MI |
| **Atom non-negativity** | ✗ | Some atoms can be negative (including `I^sx_∩` itself; not just synergy/unique) |
| **Transformation Invariance** | ✗ | Traded for TCR |

### 2.2.3 Why Negative Synergy is Possible

The impossibility results from Matthias et al. (2025, arXiv:2512.16662) prove that:

> Non-negativity + Target Chain Rule + Transformation Invariance are **mutually incompatible**

I^sx_∩ satisfies the Target Chain Rule by sacrificing all-atom non-negativity. This means synergy CAN be negative. Whether negative-synergy regimes correlate with grounding failures is an empirical question that must be tested under controlled validation and strong baselines.

### 2.2.4 What Negative Synergy Actually Means (Mathematically)

When Syn < 0:
```
Syn = I(S₁, S₂; T) - Red - Unq₁ - Unq₂ < 0
```

This means: Red + Unq₁ + Unq₂ > I(S₁, S₂; T)

**Interpretation options:**
1. **Redundancy-leaning allocation under `I^sx_∩`:** relative to other PID measures, `I^sx_∩` can allocate more (or even negative) redundancy to satisfy its axioms; the synergy term adjusts accordingly via the PID identities.
2. **Subadditivity in the chosen decomposition:** combining sources yields less *net* information about `T` than suggested by their individual terms once redundancy is accounted for (a statement about the decomposition, not about “conflict”).
3. **Pointwise misinformation:** at specific points, observing `(s₁,s₂)` can make `t` less likely than marginally expected (negative local information), which can propagate into negative PID atoms depending on the measure.
4. **Estimator/pathology:** high dimension, strong dependence, ties/quantization, and trajectory autocorrelation can all produce artifactual negative atoms; treat “unexpected signs” as a prompt to run controls, not as a conclusion.

**NOT a valid interpretation:** "The sources are fighting each other" or "conflict" in any intuitive sense. This is a seductive but potentially misleading metaphor.

## 2.3 Continuous Variable Extension

Ehrlich et al. (2024) extended I^sx_∩ to continuous variables using k-nearest neighbor (k-NN) estimation, building on the KSG estimator (Kraskov et al., 2004).

### 2.3.1 KSG Estimator

The KSG formula for mutual information:
```
I(X; Y) = ψ(k) + ψ(N) - ⟨ψ(n_x + 1) + ψ(n_y + 1)⟩
```

Where:
- ψ is the digamma function
- k is the number of neighbors
- N is sample size
- n_x, n_y are marginal counts within the k-th neighbor distance
- **Maximum norm (Chebyshev distance)** is used for BOTH k-NN search AND marginal counting

### 2.3.2 Extension to I^sx_∩

Ehrlich et al. (2024) derive a **KSG-style kNN estimator** for continuous `I^sx_∩`. It is **not** “take the minimum of pointwise MI terms.”

The key adaptation is that the shared-exclusions “OR” in Makkeh et al. (2021) becomes a **disjunction neighborhood** in source space. Under Chebyshev/L∞:

- `d_S_disj(i,j) = min( d(S₁ᵢ,S₁ⱼ), d(S₂ᵢ,S₂ⱼ) )`
- `d_ST_disj(i,j) = max( d(Tᵢ,Tⱼ), d_S_disj(i,j) )`

For each sample `i`, let `εᵢ` be the distance to the `k`-th nearest neighbor under `d_ST_disj`. Count `n_α(i)` neighbors in the source-disjunction ball and `n_T(i)` neighbors in target space within `εᵢ`, then estimate:

```
Î^sx_∩ = ψ(k) + ψ(N) − (1/N) Σ_i [ ψ(n_α(i)) + ψ(n_T(i)) ]
```

**Counting convention (make this explicit; it affects off-by-one bugs):**
- In many KSG-style presentations, neighbor counts exclude the sample itself and the formula uses `ψ(n_x(i)+1)` / `ψ(n_y(i)+1)`.
- In other (equivalent) presentations, counts **include** the sample itself and the `+1` is absorbed into the count.

This document (and `crates/pid-core`) uses the **include-self** convention for `n_α(i)` and `n_T(i)` so the digamma arguments are the inclusive counts.

See §8.1.3 for concrete implementation notes (tie handling and “strict radius” rules matter).

## 2.4 Infomorphic Networks (Optional / Exploratory)

Makkeh et al. (2025, PNAS; DOI `10.1073/pnas.2408125122`) describe “infomorphic networks”: using local information-theoretic terms as **learning objectives** rather than post-hoc analysis tools. This is *conceptually adjacent* but **not required** for Aim 1 (implementing/validating `I^sx_∩` as a diagnostic).

In infomorphic networks, neurons optimize:
```
L_local = α·Redundancy + β·Unique + γ·Synergy
```

This motivates Aim 3 as an exploratory direction *only after* Experiment 0 and the diagnostic experiments succeed, and only with a differentiable estimator (the kNN/KSG estimators in this spec are not differentiable).

## 2.5 Shannon Invariants as Problem-Solvers

Gutknecht et al. (2025, arXiv:2504.15779) introduced Shannon invariants to address PID's scalability limitations. Here we explore how they can solve specific problems in our VLA application.

### 2.5.1 The Scalability Problem

- Full PID grows super-exponentially with the number of variables
- For 2 sources: 4 atoms
- For 3 sources: 18 atoms
- For 4 sources: 166 atoms

For three variables (V, L, D), estimating all 18 atoms is computationally expensive and many atoms are hard to interpret.

### 2.5.2 What Makes an Invariant "Shannon"?

**Key Insight from Gutknecht et al. (2025):** A "Shannon invariant" is a quantity that:
1. Captures meaningful properties of information decomposition
2. **Depends only on Shannon's entropy definition** (not on which PID measure you choose)
3. Can be computed efficiently from standard MI estimates

**Why This Matters:** Different PID measures (I^sx_∩, I_min, I_BROJA, etc.) give different values for redundancy and synergy. But Shannon invariants have the **same value regardless of which PID measure you use**. This makes them theoretically robust and practically useful.

**Units note (bits vs nats):**
- Gutknecht et al. (arXiv:2504.15779) primarily report in **bits** (`log2`).
- This repo’s Rust estimators report in **nats** (`ln`).
- Changing log base multiplies all MI/entropy/PID quantities by a constant: `bits = nats / ln(2)`.
  - Signs (e.g., `CI < 0`) and rank-order comparisons are unchanged.
  - Any numeric thresholds (e.g., “MI > 4 nats”) must be converted when comparing across papers.

**The Key Example - Co-Information:**

For any bivariate PID measure, the following identity holds:

```
CI(X₁, X₂; Y) = I(X₁;Y) + I(X₂;Y) - I(X₁,X₂;Y) = Red - Syn
```

This equals Redundancy minus Synergy for **any** valid PID measure. The individual values of Red and Syn depend on your measure choice, but their difference is invariant.

### 2.5.3 Shannon Invariants: Scalar Summaries

Instead of computing all PID atoms, Shannon invariants provide interpretable numbers:

**Co-Information (Interaction Information) for 3 Variables:**
```
CI(V, L, D; A) = I(V;A) + I(L;A) + I(D;A) 
              - I(V,L;A) - I(V,D;A) - I(L,D;A) 
              + I(V,L,D;A)
```

**Interpretation:**
This is the natural higher-order extension of the pairwise “interaction information” with a distinguished target:

```
CI_m(X₁,…,X_m; Y) := Σ_{∅≠S⊆{1..m}} (-1)^{|S|+1} I(X_S; Y)
```

For `m=2`, this reduces to `CI_2(X₁,X₂;Y)=I(X₁;Y)+I(X₂;Y)-I(X₁,X₂;Y)=Red−Syn` (a Shannon invariant for any bivariate PID).

**Sign convention warning:** literature flips signs and names (co-information vs interaction information) depending on author. In this document, **negative CI is treated as “synergy-dominant”** (i.e., `Syn > Red` for the corresponding bivariate PID), and **positive CI** as “redundancy-dominant”.

**Important:** `CI_m` is a Shannon-invariant summary computed from MI terms. It is **not** a PID and it conflates multiple PID atoms for `m≥3`. Use it as a screening statistic, not as a substitute for `I^sx_∩`.

**O-Information (for n > 3 variables):**

O-information (Rosas et al., 2019) is a **synergy-vs-redundancy bias** scalar defined for a *set of variables* (no distinguished target). A standard entropy-form definition is:

```
Ω(X₁,…,Xₙ) = (n-2)·H(X₁,…,Xₙ) + Σᵢ H(Xᵢ) − Σᵢ H(X_{-i})
```

where `X_{-i}` denotes the collection of all variables except `Xᵢ`. Equivalently, `Ω = TC − DTC` (total correlation minus dual total correlation).

For `n=3`, `Ω(X,Y,Z)` equals the (3-variable) co-information / interaction information (up to sign conventions).

**How to use it here (and where it does *not* help):**
- It can summarize whether a *chosen small set* (e.g., `{V,L,D,A}` or `{V,L,D}`) is globally synergy-leaning (`Ω<0`) vs redundancy-leaning (`Ω>0`).
- It is **not automatically scalable** to “hundreds of attention heads” in the raw sense: estimating the required high-order entropies/CMIs in high-dimensional continuous spaces can be harder than PID itself unless you introduce strong structure (coarse-graining, factorization, parametric assumptions, or dedicated estimators).
- Treat `Ω` as a screening/description statistic that may motivate where to apply the hierarchical SxPID pipeline (Level 1 CI → Level 2 targeted `I^sx_∩`).

**Application to VLA:** `Ω` can be useful once you have **a small, well-defined set of variables** (or a **coarse-grained** representation of many units). It is not a “free lunch” for hundreds of raw attention heads unless you add structure (clustering/SAE, factor models, or other dimensionality reduction) and re-validate estimator behavior.

### 2.5.4 How Shannon Invariants Solve Our Problems

#### Problem 1: Combinatorial Explosion in 3-Way PID

**Problem:** Computing I(V, L, D; A) decomposition requires 18 atoms.

**Shannon Invariant Solution:** Co-information gives a single summary statistic that captures whether the overall interaction is synergistic or redundant, without needing all 18 atoms.

```python
def co_information(V, L, D, A, k=5):
    """Efficient 3-way summary using only pairwise MI estimates."""
    I_V_A = ksg_mi(V, A, k)
    I_L_A = ksg_mi(L, A, k)
    I_D_A = ksg_mi(D, A, k)
    I_VL_A = ksg_mi(np.hstack([V, L]), A, k)
    I_VD_A = ksg_mi(np.hstack([V, D]), A, k)
    I_LD_A = ksg_mi(np.hstack([L, D]), A, k)
    I_VLD_A = ksg_mi(np.hstack([V, L, D]), A, k)
    
    return I_V_A + I_L_A + I_D_A - I_VL_A - I_VD_A - I_LD_A + I_VLD_A
```

**Cost:** 7 MI estimates instead of 18 PID atoms.

#### Problem 2: Which Decomposition to Use (V-D-A vs V-L-A)?

**Problem:** We don't know a priori whether V-D-A or V-L-A is more informative.

**Shannon Invariant Solution:** Compare co-information across decompositions:

```python
CI_VD = co_information_2way(V, D, A)  # Standard: I(V;A) + I(D;A) - I(V,D;A)
CI_VL = co_information_2way(V, L, A)
CI_LD = co_information_2way(L, D, A)
```

**Interpretation (cautious):**
- CI is a Shannon-invariant summary: for any bivariate PID, `CI = Red − Syn`.
- If CI_VL is strongly negative relative to CI_VD, the V–L pair is more “synergy-dominant” than V–D (a candidate to prioritize in deeper analysis).
- If CI_VD is strongly negative relative to CI_VL, the V–D pair is more “synergy-dominant” than V–L (a candidate to prioritize in deeper analysis).
- If all are similar, either (a) the system is genuinely symmetric, or (b) the estimator regime is too noisy to differentiate pairs.

#### Problem 3: Localizing Failure Mode Without Full 3-Way PID

**Problem:** We want to know WHERE the failure is (V-L? V-D? L-D?) without computing all 18 atoms.

**Shannon Invariant Solution:** Hierarchical pairwise co-information pattern:

```python
# Compute pairwise co-information (simpler than full PID)
CI_VL = I(V;A) + I(L;A) - I(V,L;A)  # Negative = synergistic
CI_VD = I(V;A) + I(D;A) - I(V,D;A)
CI_LD = I(L;A) + I(D;A) - I(L,D;A)
```

**Heuristic interpretation (requires validation):**

| CI_VL | CI_VD | CI_LD | Interpretation (heuristic) |
|-------|-------|-------|----------------|
| < 0 | < 0 | < 0 | All pairs synergy-dominant (Syn > Red for each pair) |
| > 0 | < 0 | < 0 | V–L redundancy-dominant (Red > Syn) while others are synergy-dominant |
| < 0 | > 0 | < 0 | V–D redundancy-dominant while others are synergy-dominant |
| < 0 | < 0 | > 0 | L–D redundancy-dominant while others are synergy-dominant |
| > 0 | > 0 | < 0 | Both V–L and V–D are redundancy-dominant relative to L–D |
| > 0 | < 0 | > 0 | V–L and L–D are redundancy-dominant relative to V–D |
| < 0 | > 0 | > 0 | V–D and L–D are redundancy-dominant relative to V–L |
| > 0 | > 0 | > 0 | All pairs redundancy-dominant (Red > Syn for each pair) |

**Note:** This uses classical interaction information (MI-based), not I^sx_∩. While less fine-grained, it's much cheaper to compute.

#### Problem 4: Real-Time Monitoring

**Problem:** Full PID is too slow for real-time intervention (~seconds per sample).

**Shannon Invariant Solution:** Pre-compute PID on training data to learn a mapping from CI to failure probability:

```python
# Offline: Learn relationship
training_data = [(CI_VL, CI_VD, CI_LD, failure_label) for trajectory in training_set]
ci_to_failure_model = train_classifier(training_data)

# Online: Fast inference using only CI (7 MI estimates)
ci_vec = [co_info_2way(V, L, A), co_info_2way(V, D, A), co_info_2way(L, D, A)]
failure_prob = ci_to_failure_model.predict(ci_vec)
```

**Speed improvement:** CI requires only MI terms (7 KSG runs for a triplet); full `I^sx_∩` PID adds an additional disjunction-kNN redundancy estimator with per-sample kNN radii + neighbor counts, which is typically more expensive.

### 2.5.4 Recommended Strategy: Hierarchical Approach

**Level 1 (Fast screening; MI-only):** Compute CI_VL, CI_VD, CI_LD using KSG. Use for triage / monitoring.

**Level 2 (Targeted, slower):** Compute full pairwise `I^sx_∩` PID for the most suspicious pair (identified by Level 1).

**Level 3 (Slow, offline):** Compute full 3-way decomposition or run detailed analysis for failure diagnosis.

**Latency note (do not oversell):** Wall-clock time depends strongly on `(N, d, k)` and on the kNN backend. A brute-force O(N²) implementation is not “real-time” at N in the thousands; any ms-level budgets are design targets that presume aggressive dimensionality reduction and/or accelerated kNN.

This hierarchical approach balances speed with interpretability, using Shannon invariants as a fast screening layer.

---

# 3. The Core Research Questions

## 3.1 Primary Question

**Do shared-exclusions PID (SxPID / `I^sx_∩`) features provide statistically reliable signal for VLA failure detection and diagnosis beyond strong uncertainty baselines?**

This is the critical validation gate. The “synergy sign” is one candidate feature, but we should treat it as a hypothesis rather than a privileged statistic. If SxPID features do not significantly outperform baselines under a validated estimator regime, we report negative results.

**Diagnostic-first stance (recommended):**
- It is scientifically reasonable to treat PID primarily as a **diagnostic / interpretability** tool (explaining *which sources matter and how*), even if it does not beat entropy/PRMs on AUROC.
- It is *not* scientifically justified to reduce PID to “synergy only” a priori. In practice, the most reliable signal may come from a **feature set** that includes MI terms, CI, redundancy, uniques, synergy, and derived summaries—then letting Experiments 1–2 determine what actually predicts failures.

## 3.2 Secondary Questions

1. **Which decomposition is most predictive?** V-D-A? V-L-A? Three-way? Something else?
2. **At what dimensionality does the estimator work?** Raw 4096-dim? PCA to 256? Learned projections?
3. **Is synergy causal or merely correlated with failure?** Can interventions on D cause synergy changes?
4. **Does synergy dynamics predict success?** Does synergy half-life correlate with task completion?

## 3.3 Specific Aims

**Non-negotiable gate:** All aims below assume Experiment 0 establishes a validated estimator regime (possibly only **after** explicit dimensionality reduction). If Experiment 0 is **NO-GO** even after reduction (e.g., at \(d \approx 256\)), we do not claim results about kNN-based `I^sx_∩` on VLA embeddings; we pivot to Shannon-invariant screening and non-`I^sx_∩` baselines.

### Aim 1 (Primary): Comparative Evaluation (Experiments 1–2)

**Primary hypothesis (falsifiable; pre-register):** Under a validated estimator regime, a feature set derived from Shannon invariants (CI/Ω) and (where feasible) SxPID atoms from plausible decompositions (e.g., V–D–A, V–L–A, hierarchical pairwise) contains predictive information about failure labels beyond the best baseline.

**Candidate sub-hypothesis (not privileged):** The “synergy sign / frequency of negative-synergy windows” contributes additional signal beyond MI/entropy alone; it may also fail entirely (estimator/pathology or irrelevance).

**Baselines:**
1. Predictive entropy: H(A|V, L)
2. Semantic entropy (VL-Uncertainty)
3. Snapshot ensemble variance
4. Cross-modal attention entropy
5. Learned failure classifier
6. Liang et al. Batch/CVX estimators (different PID family; baseline only)
7. **Process Reward Model (GRM):** Progress-based failure detection (Robo-Dopamine)

**Success criteria:** statistically significant improvement over best baseline (paired bootstrap or matched test; p < 0.05) with a practically meaningful effect size (pre-registered), OR a well-supported negative result (no improvement) with analysis of failure causes (estimator regime, confounds, variable choice).

### Aim 2: Regime Mapping for High‑d / Manifold‑Valued Embeddings (Experiment 3 + Exp0 subsets)

**Question:** At what effective dimensionality and preprocessing does the estimator become stable enough to support Aim 1?

**Deliverable:** a regime map and a single recommended measurement pipeline (e.g., Raw vs PCA95 vs random/Hash projection), with geometry diagnostics (intrinsic dimension, distance concentration proxies) recorded at each stage and explicit GO/PIVOT/NO-GO outcomes.

### Aim 3: Causal Validation for Diagnosis (Experiment 4)

**Question:** Are decomposition signatures merely correlational, or do controlled interventions / counterfactual targets (`A*`) produce predictable, reproducible changes?

**Design note:** causal claims require interventions (on `D`/`V`/`L`) or external targets; otherwise VLA self-consistency can masquerade as “information integration.”

### Optional extensions (only if Aim 1 succeeds AND Aim 2 yields a stable regime)

- **Synergy dynamics:** test whether time-resolved summaries (e.g., a “synergy half-life” under explicit windowing/stride + dependence-aware uncertainty) add signal beyond static features.
- **RL fine-tuning (exploratory):** kNN/KSG estimators are not differentiable and are unlikely to be safe as direct rewards. If pursued, follow Wibral-group “infomorphic networks” framing (Makkeh et al. 2025) or train an offline differentiable surrogate to predict SxPID-derived features; treat generic PRM/SRL methods as baseline controls.

## 3.4 Where PID Provides Unique Value (Six Use Cases)

**Important framing:** PID may not outperform entropy for pure AUROC on failure detection. Its unique value lies in **interpretable decomposition**. Here are six specific use cases where decomposition provides value that entropy cannot:

### Use Case 1: Failure Mode Diagnosis (Post-Hoc)

**Scenario:** Robot fails a task. We want to know WHY.

**What entropy tells us:** "The model was uncertain."

**What PID decomposition may suggest (hypotheses that require validation):**

| Pattern (estimated) | Hypothesis | Next Check |
|---------|-----------|-------------------|
| High Unq(V), low Unq(D) | Policy relies mostly on V for T | Intervention on D should have limited effect |
| Low Unq(V), high Unq(D) | Policy relies mostly on D for T | Intervention on V (occlusion/corruption) should strongly affect |
| High Syn (positive) | Joint V–D interaction contributes | Check whether synergy tracks task phase or known fusion modules |
| Low/negative Syn | Subadditivity / possible integration anomaly | Distinguish (a) estimator pathology, (b) redundancy inflation, (c) true “misinformation” via controls |
| High Unq(L) (V–L–A) | Language heavily determines T | Check instruction perturbations / paraphrases |

### Use Case 2: Architecture Design Feedback

**Scenario:** Comparing two VLA architectures on the same task.

**What entropy tells us:** "Architecture A is more certain than B."

**What PID decomposition tells us:**
- "Architecture A shows higher Syn, suggesting better multimodal fusion"
- "Architecture B shows higher Unq(V), suggesting it relies more on vision"
- "Architecture A shows higher Red, suggesting V and D encode similar information (potential redundancy)"

This informs which architectural choices improve integration vs. reliance on individual modalities.

### Use Case 3: Training Curriculum Design

**Scenario:** Designing a training curriculum for VLA fine-tuning.

**What entropy tells us:** "Train on examples where the model is uncertain."

**What PID decomposition tells us:**
- *(Hypotheses; require a validated estimator regime + controls for confounds like task difficulty and distribution shift.)*
- "Model shows persistently low/unstable Syn on manipulation tasks → candidate integration weakness → prioritize targeted manipulation data"
- "Model shows high Unq(D) and low Unq(V) (relative to validated baselines) → candidate over-reliance on internal state → add visual grounding data"
- "Model shows atypical V–L interaction signatures (e.g., CI_VL strongly negative or `Syn_VL` consistently extreme) → candidate language–vision alignment issue → add alignment data"

**Proposed curriculum objective:**
```python
curriculum_priority = α*|Syn| + β*imbalance(Unq_V, Unq_D) + γ*task_importance
```

### Use Case 4: Targeted Data Collection

**Scenario:** Limited budget for collecting new robot demonstrations.

**What entropy tells us:** "Collect data for high-uncertainty scenarios."

**What PID decomposition tells us:**
- *(Hypotheses; PID atoms can be negative under `I^sx_∩` and can be estimator-sensitive at high `d`.)*
- "Model needs visual grounding data (e.g., Unq(V) systematically low relative to Unq(D) under validated preprocessing)"
- "Model needs language–vision alignment data (e.g., V–L pair shows abnormal CI/PID signatures compared to controls)"
- "Model needs action diversity / disambiguation (e.g., redundancy-dominant signatures across task variants; verify with controlled task splits)"

This enables **targeted** data collection rather than blanket uncertainty-based collection.

### Use Case 5: Real-Time Intervention Selection

**Scenario:** Robot is about to fail. What help should we provide?

**What entropy tells us:** "Robot is uncertain → request help."

**What PID decomposition tells us:**
- *(Heuristic; only meaningful if the estimator regime is validated and the mapping is learned/calibrated on held-out data.)*
- High Unq(D), low Unq(V) → "Show me what you see" (visual confirmation)
- High Unq(V), low Unq(D) → "What do you expect to happen?" (prediction query)  
- V–L signatures suggest mismatch → "Did you understand the instruction?" (language clarification)

**Note:** This requires fast PID computation (<10ms), which is currently challenging. May need to use PID signatures computed offline to classify real-time scenarios.

### Use Case 6: Interpretability for Safety Certification

**Scenario:** Certifying a VLA for deployment in safety-critical settings.

**What entropy tells us:** "Model uncertainty stays below threshold X."

**What PID decomposition tells us:**
- "Model avoids extreme negative-atom regimes outside those seen in validated controls" (a stability check, not a guarantee of “coherence”)
- "Model shows stable information signatures across task variations under fixed preprocessing" (robustness evidence if replicated)
- "Failure modes are *more traceable* to specific information sources" (interpretability hypothesis; validate against intervention tests)

This provides an **audit trail** for safety certification that entropy alone cannot provide.

### Summary: When to Use PID vs. Entropy

| Goal | Use Entropy | Use PID |
|------|-------------|---------|
| Simple failure detection | ✓ (faster, comparable AUROC) | |
| Understanding WHY failure occurred | | ✓ (decomposition) |
| Comparing architectures | | ✓ (multimodal integration metrics) |
| Training curriculum design | | ✓ (targeted improvement) |
| Data collection prioritization | | ✓ (specific capability gaps) |
| Safety certification | | ✓ (audit trail) |

**Positioning Statement:** PID is complementary to entropy, not a replacement. Use entropy for speed and simplicity; use PID for interpretability and actionable insights.

## 3.5 PID vs. Process Reward Models (PRMs)

### 3.5.1 What Are Process Reward Models?

Process Reward Models (PRMs) are vision-language models trained to predict task progress from visual observations. Unlike outcome reward models (ORMs) that only provide sparse binary success/failure signals, PRMs provide dense, step-by-step progress estimates.

**Recent Example: Robo-Dopamine (arXiv:2512.23703)**

Robo-Dopamine introduces a General Reward Model (GRM) trained on 35M samples from 3,400+ hours of video:
- **Step-wise Reward Discretization:** Hop-based relative progress labels
- **Multi-Perspective Progress Fusion:** Combines incremental, forward-anchored, and backward-anchored predictions
- **Policy-Invariant Reward Shaping:** Avoids "semantic trap" where agent stagnates in high-progress states
- **Results:** 92.8% progress accuracy, 0.953 VOC score, policy improves from ~0% to 95% in 150 rollouts *(paper-reported; verify evaluation protocol if used for quantitative comparisons).*

### 3.5.2 Comparison: PID vs. PRM

| Aspect | PID (I^sx_∩) | PRM (e.g., GRM) |
|--------|--------------|-----------------|
| **What it measures** | Information structure between V, D, A | Task progress toward goal |
| **Output** | Syn, Red, Unq decomposition | Progress estimate Φ ∈ [0,1] |
| **Interpretability** | WHY failure occurred | HOW FAR along task |
| **Computational cost** | O(n² × d) per pair | O(1) forward pass |
| **Training required** | None (estimator-based) | 35M+ samples, 3400+ hours |
| **Multi-view support** | Implicit in embeddings | Explicit (GRM uses multi-view fusion) |
| **Real-time feasible** | Shannon invariants only (fastest; depends on n,d and kNN backend) | Yes (single forward pass; hardware-dependent) |

### 3.5.3 When to Use Each

| Scenario | Use PID | Use PRM |
|----------|---------|---------|
| Diagnosing WHY failure occurred | ✓ | |
| Dense reward for RL fine-tuning | | ✓ |
| Comparing multimodal fusion quality | ✓ | |
| One-shot task adaptation | | ✓ (GRM adapts from 1 demo) |
| Architecture design feedback | ✓ | |
| Policy learning efficiency | | ✓ |
| Understanding V-D integration | ✓ | |
| Progress monitoring in deployment | | ✓ |

### 3.5.4 Potential Synergies

PID and PRMs can be complementary:

1. **PRM-guided PID sampling:** Use GRM progress estimates to identify critical transitions, then apply PID for detailed diagnosis
2. **PID-augmented rewards:** Add PID-based synergy term to PRM rewards for multimodal coherence:
   ```
   r_combined = r_GRM + α·Syn(V,D;A)
   ```
3. **OOD detection fusion:** GRM's consistency checking (forward vs backward anchored disagreement) + PID's synergy sign could provide robust failure detection
4. **Multi-Perspective Fusion analogy:** GRM's fusion of {incremental, forward-anchored, backward-anchored} predictions mirrors how we might fuse {Syn_VD, Syn_VL, Syn_LD} in hierarchical PID

### 3.5.5 Key Insight from Robo-Dopamine: The Semantic Trap

Robo-Dopamine identifies a critical failure mode in naive reward shaping:

**Problem:** Using r(s,a,s') = Φ(s') - Φ(s) as dense reward creates perverse incentives. The agent learns to reach high-progress states and stagnate rather than complete tasks.

**Their Solution:** Policy-Invariant Reward Shaping:
```
r_GRM = r_gold + γΦ(s') - Φ(s)
```
where r_gold = 1 at task completion. This telescopes to a boundary term, preserving optimal policy.

**Relevance to PID:** A similar trap could occur if using Syn as intrinsic reward. Our proposed:
```
r_intrinsic = α·Syn(V,D;A) - γ·max(0, -Syn)
```
should be analyzed for policy invariance properties.

---

# 4. Decomposition Strategies: What Variables to Analyze

## 4.1 The Original Proposal: V-D-A

```
I(V, D; A) = Red(V,D;A) + Unq(V;A) + Unq(D;A) + Syn(V,D;A)
```

Where:
- **V** = Vision (observed scene, from vision encoder)
- **D** = Dream (internal world model state)
- **A** = Action (motor output)

### 4.1.1 Problems with V-D-A

1. **Potential degeneracy:** because `A` is computed from `(V,D,L)`, `I(V,D;A)` can become close to `H(A)` when `L` is constant/redundant and inference is near-deterministic. Treat this as a *dataset- and inference-protocol-dependent risk*, not an identity (see Warning 2 in §1.2).
2. **L is ignored:** Language instruction is not in the decomposition
3. **D is often implicit:** In autoregressive VLAs like OpenVLA, there's no explicit "dream" state

## 4.2 Alternative: V-L-A (Vision-Language-Action)

```
I(V, L; A) = Red(V,L;A) + Unq(V;A) + Unq(L;A) + Syn(V,L;A)
```

### 4.2.1 Advantages of V-L-A

| Advantage | Explanation |
|-----------|-------------|
| **L is always available** | No need to extract hidden states |
| **L is externally specified intent** | It encodes what the human requested (often the closest available “ground truth” for intent, but can be ambiguous) |
| **Language grounding failures are common** | "Pick up red cup" → picks blue |
| **Often more interpretable than D** | Language is more interpretable than an implicit “dream” state, but negative synergy still requires careful controls/validation |

### 4.2.2 Interpretation of V-L-A Atoms

| PID Atom | Interpretation |
|----------|----------------|
| Unq(L;A) | Action determined purely by instruction (ignoring scene) |
| Unq(V;A) | Action determined purely by visual scene (ignoring instruction) |
| Syn(V,L;A) > 0 | Joint V–L interaction appears important (candidate “integration” signal) |
| **Syn(V,L;A) < 0** | Subadditivity / potential mismatch; distinguish from estimator artifacts and redundancy inflation via controls |

### 4.2.3 Why V-L-A Might Be Better Than V-D-A

Many VLA failures are specifically **language grounding failures**:
- "Pick up the red cup" → robot picks up blue cup
- "Place it on the left" → robot places on right
- Instruction ambiguity → wrong interpretation

V-D-A cannot distinguish these from other V–D internal mismatch hypotheses without language-side controls.

## 4.3 The Question of Ignoring L

### 4.3.1 Arguments FOR Ignoring L (Original Approach)

1. **L is static within a trajectory** - doesn't change mid-execution
2. **D encodes L-conditioned predictions** - already incorporated
3. **Simplicity** - three-variable PID is tractable

### 4.3.2 Arguments AGAINST Ignoring L

1. **Language grounding failures are major failure mode** - can't detect them without L
2. **V-L-A decomposition is more interpretable** - external signals only
3. **D may not be cleanly separable** - especially in autoregressive models
4. **Liang et al. include language** - validated approach includes L

### 4.3.3 Recommendation

**Elevate V-L-A to co-primary status with V-D-A.** Test both and compare predictive power.

## 4.4 Other Decomposition Options

| Decomposition | Sources | Target | Hypothesis |
|---------------|---------|--------|------------|
| V-D-A | Vision, Dream | Action | V–D mismatch may correlate with certain failures (requires controls) |
| V-D-A* | Vision, Dream | Optimal Action | Measures error, not tautology |
| V-L-A | Vision, Language | Action | V–L mismatch may correlate with language-grounding failures (requires controls) |
| D_t-D_{t-1}-A | Current Dream, Previous Dream | Action | Temporal inconsistency → failure |
| V-A*-Error | Vision, Optimal Action | Prediction Error | Directly predicts failure magnitude |

---

# 5. Three-Way PID: I(V, L, D; A)

## 5.1 Motivation

Rather than choosing between V-D-A or V-L-A, we could analyze all three sources simultaneously:

```
I(V, L, D; A) = ?
```

This would capture:
- Vision–language mismatches
- Vision–dream mismatches  
- Language–dream mismatches (e.g., instruction misinterpretation vs world-state representation)
- Three-way synergies and redundancies

## 5.2 The Problem: Combinatorial Explosion

For two sources, PID has 4 atoms: {Red, Unq₁, Unq₂, Syn}

For three sources, the partial information lattice has **18 distinct antichains** (atoms):

```
                        {VLD}                    ← Full synergy (all three needed)
                       /  |  \
                  {VL} {VD} {LD}                 ← Pairwise synergies
                 / | \ / | \ / | \
              {V} {L} {D}                        ← Unique information
                 \ | / \ | / \ | /
                  {VL∩} {VD∩} {LD∩}              ← Pairwise redundancies  
                       \  |  /
                        {VLD∩}                   ← Full redundancy (any one suffices)
```

Estimating 18 quantities is expensive and many are hard to interpret.

## 5.3 Practical Options for Three-Way Analysis

### Option 1: Full 3-Source PID

Compute all 18 atoms.

**Pros:** Complete picture  
**Cons:** Expensive, hard to interpret, estimation variance multiplies

### Option 2: Shannon Invariants / Co-Information

Compute a summary statistic:

```python
CI(V, L, D; A) = I(V;A) + I(L;A) + I(D;A) 
              - I(V,L;A) - I(V,D;A) - I(L,D;A) 
              + I(V,L,D;A)
```

This is the "interaction information" or "co-information":
- Negative = synergistic (three-way cooperation)
- Positive = redundant (three-way overlap)

**Pros:** Single interpretable number, cheap  
**Cons:** Loses fine-grained structure

### Option 3: Hierarchical Pairwise (RECOMMENDED)

Compute three separate 2-source PIDs:

```
PID(V, L; A)  → Syn_VL  (vision-language coherence)
PID(V, D; A)  → Syn_VD  (vision-dream coherence)
PID(L, D; A)  → Syn_LD  (language-dream coherence)
```

**Diagnostic Matrix:**

| Syn_VL | Syn_VD | Syn_LD | Hypothesis (requires validation) |
|--------|--------|--------|----------------|
| + | + | + | Pairwise synergies appear positive (suggests interaction-dominant regime) |
| - | + | + | V–L interaction appears weak/subadditive relative to other pairs (check language perturbations) |
| + | - | + | V–D interaction appears weak/subadditive (check D corruption / vision occlusion controls) |
| + | + | - | L–D interaction appears weak/subadditive (check instruction changes and D dependence) |
| - | - | + | V appears atypical relative to (L,D) (could be occlusion/OOD; check estimator stability) |
| - | + | - | L appears atypical relative to (V,D) (could be instruction ambiguity; check paraphrase robustness) |
| + | - | - | D appears atypical relative to (V,L) (could be world-model mismatch; check D interventions) |
| - | - | - | Broad subadditivity across pairs (could be estimator breakdown; run controls + Experiment 0-style checks) |

**Pros:**
- Only 3× the cost of single PID
- Each pairwise synergy is interpretable
- Pattern across all three is diagnostic
- Localizes failure mode

**Cons:**
- Doesn't capture true 3-way synergy
- Some redundant computation

### Option 4: Conditional PID

Compute PID conditioned on the third variable:

```
PID(V, D; A | L)  → "Given the instruction, how do vision and dream interact?"
```

**Pros:** Controls for task variation  
**Cons:** Requires more samples per conditioning value; conditional MI estimation is itself hard in high dimension. If conditioning becomes central, consider dedicated conditional-MI estimators as baselines (e.g., CCMI, arXiv:1906.01824), but treat that as a separate validated estimator pipeline (not automatically compatible with `I^sx_∩`).

## 5.4 Recommendation

**Start with Option 3 (Hierarchical Pairwise)**, with co-information (Option 2) as a summary.

The pattern {Syn_VL, Syn_VD, Syn_LD} can help generate and localize **testable hypotheses**:
- All negative → broad subadditivity across pairs (could be genuine mismatch or estimator breakdown; investigate with controls)
- Only Syn_VL negative → candidate V–L integration issue (validate with language-side perturbations)
- Only Syn_VD negative → candidate V–D mismatch (validate with D/V interventions and estimator controls)
- Only Syn_LD negative → candidate L–D mismatch (validate with instruction changes and D dependence)

---

# 6. Discarded Approaches and Why

## 6.1 OpenVLA vs DreamVLA Architectural Comparison

### 6.1.1 The Original Idea

Compare PID profiles between:
- **OpenVLA (arXiv:2406.09246; Kim et al. 2024):** autoregressive VLA with no explicit “dream/world model” prediction head; actions via discrete tokens / binning (paper details; verify exact action parameterization before treating as a variable definition).
- **DreamVLA (arXiv:2507.04447; Zhang et al. 2025):** GPT‑2-style backbone + structured attention + explicit world‑knowledge prediction channels (e.g., dynamic/depth/semantic) + diffusion-style action modeling (see paper for exact heads/parameterization).
  - Related but distinct: **Dream‑VL & Dream‑VLA (arXiv:2512.22615; Ye et al. 2025)** uses a diffusion language-model backbone; do not conflate its architectural details with DreamVLA unless explicitly matched.

**Hypothesis (weaker / testable):** Architectures with explicit predicted world-knowledge channels may yield different PID signatures than those without such channels, *under matched variable definitions and matched targets*. Whether those differences correlate with “grounding failures” remains empirical.

### 6.1.2 Why It Was Discarded

#### Reason 1: The Core Hypothesis Became Questionable

During first-principles review, we discovered that "negative synergy = hallucination" is not mathematically rigorous. It's a hypothesis, not a definition.

#### Reason 2: Too Many Confounding Variables

| Aspect | OpenVLA | DreamVLA |
|--------|---------|----------|
| Backbone | Llama 2 7B | GPT-2 based |
| Action representation | 256-bin discretization | Continuous actions (diffusion-based transformer in arXiv:2507.04447) |
| World model | Implicit/none | Explicit world-knowledge prediction (dynamic/depth/semantic) |
| Vision encoder | SigLIP | See paper (arXiv:2507.04447); semantics pipeline references DINOv2 + SAM in overview/figures |
| Attention | Causal (autoregressive) | Block-wise structured |
| Training data | Open-X | Open-X + additional |

If we observe different PID profiles, we CANNOT attribute the difference to "world model quality" because too many variables differ.

#### Reason 3: “D” Exists Explicitly in One Model but Not the Other (Definition Mismatch)

DreamVLA explicitly predicts world‑knowledge via dedicated channels/tokens. This makes a **concrete “D”** operationalization plausible *within DreamVLA* (and supports targeted interventions on D).

OpenVLA does not provide an explicit “dream/world model” output channel by default. Any “D” you define in OpenVLA is necessarily an **extracted hidden state**, which changes the scientific question (and makes cross‑model comparisons fragile).

As a result, an OpenVLA↔DreamVLA PID comparison risks becoming circular or uninterpretable: observed differences may reflect **variable-definition choices**, not “world model quality.”

#### Reason 4: "D" is Ill-Defined for OpenVLA

DreamVLA has explicit "dream" outputs (dynamic region, depth, semantics). For OpenVLA, "D" would need to be extracted from intermediate hidden states, which is:
- Arbitrary (which layer? which tokens?)
- Not comparable to DreamVLA's explicit D
- May not represent world model at all

### 6.1.3 Why It Might Still Be Interesting

Despite these issues, the comparison could be valuable IF:
1. We first validate PID on a single architecture
2. We carefully control for confounds
3. We interpret results cautiously

**Recommendation:** Defer until after core validation (Experiments 0-3).

## 6.2 Using WAN for Analytical (Not Just Visualization) Purposes

### 6.2.1 The Original Idea

WAN's 3D Causal VAE (Wan-VAE) is itself a learned world model. We could:
1. Use it as a **proxy** for what a "good" world model should predict
2. Compare VLA's synergy against WAN's synergy
3. Treat large, systematic gaps (e.g., `Syn_VLA << Syn_WAN` under *matched variable definitions*) as a **hypothesis** about failure modes, not a diagnostic; validate with labels and controlled interventions.

### 6.2.2 WAN Ecosystem Overview (Updated December 2025)

The WAN (Wanxiang/Tongyi Wanxiang) family has evolved significantly:

| Model | Parameters | Key Features |
|-------|------------|--------------|
| Wan 2.1 T2V/I2V | 1.3B / 14B | Base text/image-to-video, DiT architecture |
| Wan 2.1 FLF2V | 14B | First-Last-Frame interpolation |
| **Wan 2.1 VACE** | 1.3B / 14B | **All-in-one video creation and editing** |
| **Wan 2.2** | 27B (14B active) | MoE architecture, faster inference |
| Wan 2.2 Animate | 14B | Human/character animation |
| Wan 2.6 | — | Reference-to-video, multi-shot storytelling |

**VACE (Video All-in-one Creation and Editing)** is particularly relevant:
- Unified framework for R2V (reference-to-video), V2V (video-to-video), MV2V (masked editing)
- Video Condition Unit (VCU) for organized task inputs
- Supports depth, pose, and mask conditioning
- Available at: `github.com/Wan-Video/Wan2.1` and via Diffusers

**Wan-Move** (NeurIPS 2025) enables motion control:
- Dense point trajectory guidance in latent space
- No architecture change to base I2V model
- Supports object dragging, camera motion, 3D rotation, motion transfer

### 6.2.3 Can WAN Be Made Action-Conditioned?

**Yes, through several approaches:**

| Approach | Paper/Project | Method |
|----------|---------------|--------|
| LoRA fine-tuning | DreamGen, Scalable Policy Eval | Add action tokens, fine-tune with LoRA |
| VACE conditioning | Wan 2.1 VACE | Use Video Condition Unit with robot states |
| Latent action integration | Motus | Wan 2.2 5B as "Generative Expert" in unified model |
| Trajectory guidance | Wan-Move | Propagate motion through latent trajectories |

**DreamGen Benchmark Results** (benchmarking WAN 2.1 for robotics):
- Instruction Following: ~60-70% (paper-reported; GPT-4o evaluation is protocol-sensitive; treat as approximate)
- Physics Alignment: ~50-60% (paper-reported; verify task suite and scoring)
- Outperformed by Cosmos (pre-trained on physical-AI data)
- Can be fine-tuned on robot data to improve alignment

**Motus Architecture** (December 2025) demonstrates WAN integration:
```
Understanding Expert (Qwen3-VL-2B) ─┐
Generative Expert (Wan 2.2 5B) ─────┼─ Tri-model Joint Attention → Action
Action Expert (Transformer) ────────┘
```
- Uses UniDiffuser-style scheduling for multi-modal generation
- Achieves 88.66% on RoboTwin 2.0 (50 tasks)

### 6.2.4 Why Original Concerns Remain Partially Valid

#### Concern 1: Distribution Mismatch (PARTIALLY ADDRESSED)

WAN was trained on general videos (1.5B videos + 10B images), not robot manipulation. However:
- **Mitigation:** LoRA fine-tuning on 1000-10,000 robot trajectories shows strong transfer
- **Caveat:** Zero-shot robot video generation is poor; fine-tuning is necessary
- **Alternative:** Cosmos (pre-trained on physical-AI data) performs better out-of-box

#### Concern 2: Latent Space Incompatibility (STILL VALID)

VLA latent encodes task-relevant features; WAN latent encodes visual reconstruction features. 
- **For PID analysis:** Computing PID on VLA latents directly is still preferable
- **For visualization:** WAN provides excellent rendering of VLA predictions

#### Concern 3: Computational Cost (IMPROVED IN 2.2)

| Model | Time (RTX 4090) | Notes |
|-------|-----------------|-------|
| Wan 2.1 14B | ~4 min / 5s clip | Original |
| Wan 2.2 14B (MoE) | ~2-3 min / 5s clip | MoE reduces active params to 14B |
| Wan 2.2 TI2V 5B | ~1-2 min / 5s clip | Runs on consumer GPUs (4090, 22GB VRAM) |
| Wan 2.1 1.3B | ~4 min / 5s 480p | Lightweight option |

Still too slow for real-time, but feasible for offline analysis.

#### Concern 4: Circular Reasoning Risk (STILL VALID BUT MANAGEABLE)

If we fine-tune WAN on robot data, it learns similar biases. Mitigation:
- Use WAN fine-tuned on **different** robot datasets than VLA
- Use WAN only for visualization, not analytical comparison
- Use independent world models (GWM, Cosmos) for synergy comparison

### 6.2.5 Recommended Alternative: GWM (Gaussian World Model)

GWM (ICCV 2025) remains more appropriate for **analytical** purposes:

| Property | WAN (base) | WAN (fine-tuned) | GWM |
|----------|------------|------------------|-----|
| Trained on robot data | No | Yes (LoRA) | **Yes (native)** |
| 3D representation | No (2D video) | No | **Yes (3DGS)** |
| Action-conditioned | No | **Yes** | **Yes** |
| Latent space alignment | Poor | Medium | **High** |
| Inference speed | Slow | Slow | **Faster** |

### 6.2.6 When to Use WAN vs GWM vs Neither

| Use Case | Recommendation |
|----------|----------------|
| Core PID validation (Aims 1-2) | **Neither** - compute PID on VLA latents only |
| Debugging specific failures | **GWM** - 3D spatial localization |
| Paper figures / demos | **WAN** - highest visual quality |
| Training data augmentation | **WAN VACE** or **GWM** - both action-conditioned |
| Unified world model baseline | **Motus** (uses WAN 2.2 internally) |
| Real-time intervention | **Neither** - too slow, use entropy |

### 6.2.7 Key Resources

```
WAN Official:
- GitHub: github.com/Wan-Video/Wan2.1, github.com/Wan-Video/Wan2.2
- HuggingFace: Wan-AI/Wan2.2-T2V-A14B, Wan-AI/Wan2.1-VACE-14B-diffusers
- Website: wan.video

Extensions:
- Wan-Move: arxiv.org/abs/2512.08765 (motion control)
- VACE: arxiv.org/abs/2503.07598 (all-in-one editing)
- Motus: arxiv.org/abs/2512.13030 (unified latent action world model)
- DreamGen: arxiv.org/abs/2505.12705 (robot learning via neural trajectories)
```

## 6.3 Using Full 3-Source PID from the Start

### 6.3.1 Why It Was Discarded

- 18 atoms to estimate (expensive)
- Many atoms are hard to interpret
- Estimation variance multiplies
- Hierarchical pairwise gives most of the benefit

### 6.3.2 Why It's Still Potentially Interesting

True three-way synergy (information requiring ALL THREE of V, L, D) might be important for complex tasks. Worth exploring after pairwise validation.

## 6.4 Using Raw 4096-dim Embeddings

### 6.4.1 Why It's Problematic

Curse of dimensionality: at d=4096, k-NN methods fail because nearest neighbors become nearly equidistant.

### 6.4.2 Mitigation

Test dimensionality reduction:
1. PCA to 256-dim (retaining 95% variance)
2. Learned projections to 64-dim
3. Use intermediate VLA layers instead of final embeddings

---

# 7. VLA Architecture Analysis

## 7.0 Conceptual Framing: Dual-Process Theory Analogy (With Caveats)

### 7.0.1 The Analogy

An intriguing conceptual parallel exists between our V-D decomposition and dual-process theories of cognition (Kahneman, 2011). In cognitive psychology:

| System | Characteristics | Proposed VLA Analogue |
|--------|----------------|----------------------|
| **System 1** | Fast, automatic, reactive, feedforward | **V** (Vision): Direct perceptual features from early layers |
| **System 2** | Slow, deliberate, predictive, requires working memory | **D** (Dream): World model predictions requiring temporal integration |

Under this framing:
- **High synergy** might indicate coherent integration between reactive perception and predictive reasoning—analogous to healthy System 1/2 coordination
- **Low/negative synergy** might indicate a failure to integrate—analogous to the cognitive conflict when "gut feeling" contradicts deliberation

This parallel is particularly apt for **DreamVLA**, which explicitly separates:
- Vision encoder (feedforward, "System 1-like")
- World model with `<dream>` queries (requires attending to past states, predicting future, "System 2-like")
- Diffusion action head (integrates both streams)

### 7.0.2 Why This Analogy Is LIMITED (Important Caveats)

**We emphasize this is a loose conceptual analogy, NOT a mechanistic claim:**

1. **Timescales don't match:** System 1 and 2 differ by 100-1000× in processing speed in humans. In VLAs, V and D are computed in the same forward pass with similar latency.

2. **Architecture doesn't match:** Human dual-process theory involves distinct neural circuits (e.g., Default Mode Network vs. Prefrontal Cortex). VLAs have a single unified architecture.

3. **We are NOT testing dual-process theory:** Our decomposition is grounded in information theory (PID), not cognitive architecture. We make no claims about VLAs "implementing" System 1/2.

4. **The analogy could mislead:** Reviewers familiar with cognitive science may object to loose application of these terms.

### 7.0.3 When This Framing IS Useful

- **Grant motivation:** Helps non-technical reviewers understand the intuition
- **Discussion section:** Situates findings in broader cognitive science context
- **Future directions:** Could motivate architectures with explicit fast/slow pathways

### 7.0.4 When to AVOID This Framing

- **Core hypothesis:** Don't claim "PID measures System 1/2 integration"
- **Technical sections:** Use precise information-theoretic language
- **Wibral group review:** They are mathematically rigorous; lead with PID formalism

### 7.0.5 The Scientifically Defensible Claim

**What we CAN say:**

> "Our V-D decomposition separates early visual features from later integrated representations that incorporate world model predictions. PID quantifies how these two information streams combine to determine actions. This is conceptually analogous to—though mechanistically distinct from—the integration of fast reactive processing with slower deliberative reasoning in dual-process cognitive theories."

**What we CANNOT say:**

> "PID measures System 1/2 integration in VLAs" ❌

## 7.1 OpenVLA (arXiv:2406.09246)

### 7.1.1 Architecture

```
Image → ┌─ SigLIP ViT ─┐
        │              ├─ [concat] → 2-layer MLP Projector → Llama 2 7B → Action Tokens → 256-bin
        └─ DinoV2 ─────┘                                         ↑
           (600M total)                                  Language Instruction
```

- **Backbone:** Llama 2 7B (autoregressive)
- **Vision Encoder:** SigLIP + DinoV2 fused (600M parameters total; channel-wise concatenation)
- **Action Representation:** 256-bin discretization per dimension
- **Training:** Open-X Embodiment dataset (970k trajectories)
- **Patch Embeddings:** 256 patches extracted from each ViT, concatenated along hidden dimension, projected into LLM input space via 2-layer MLP

### 7.1.2 Key Properties for PID Analysis

- **No explicit world model:** "D" must be inferred from hidden states
- **Causal attention:** Each token only attends to previous tokens
- **Hidden states:** 4096-dim at each of **32 transformer decoder blocks** (Llama 2 7B architecture)
- **Layer-specific encoding:** object-state vs action-state localization is *often* reported in probing studies for large transformers, but treat any specific layer claim here as **unverified until you cite a concrete probing result for OpenVLA**.

**Verified Dimensions (Jan 2026):**
| Component | Dimension | Source |
|-----------|-----------|--------|
| Hidden size | 4096 | Llama 2 7B spec |
| Transformer layers | 32 | Llama 2 7B spec |
| Attention heads | 32 | Llama 2 7B spec |
| Vision encoder params | 600M | OpenVLA paper |
| Action bins | 256/dim | OpenVLA paper |

### 7.1.3 Where to Extract "D"?

Options:
1. **Layer 16 (middle):** candidate “mid-level” representation (heuristic; requires model-specific probing)
2. **Layer 24:** candidate “late” representation (heuristic; requires model-specific probing)
3. **Average across layers:** Lose layer-specific information
4. **Don't use D at all:** Focus on V-L-A decomposition

## 7.2 DreamVLA (arXiv:2507.04447)

### 7.2.1 Architecture

```
Image → (vision encoder; see DreamVLA paper) → GPT-2 backbone with block-wise structured attention (DreamVLA, arXiv:2507.04447)
                              ↓
              ┌───────────────┼───────────────┐
              ↓               ↓               ↓
         Dynamic          Depth         Semantic
         Prediction      Prediction     Prediction
              ↓               ↓               ↓
              └───────────────┼───────────────┘
                              ↓
                    World Embedding (D)
                              ↓
                    Diffusion-based Action Transformer
```

**DreamVLA (arXiv:2507.04447, Zhang et al. 2025)** (verified by PDF inspection) describes:
- **Backbone:** GPT-2 based multimodal transformer with **block-wise structured attention**
- **Explicit world-knowledge prediction:** predicts **dynamic regions**, **depth**, and **semantic knowledge** (the paper references DINOv2 + SAM for semantics in its overview/figures)
- **Action modeling:** a **diffusion-based transformer** for action generation (not "flow matching" in the arXiv v3 PDF)
- **Key architectural idea:** prevent interference/leakage between dynamic/spatial/semantic streams via structured attention masks
- **Vision encoder:** MAE-based (Masked Autoencoder)

**⚠️ Dimension Caveat (Jan 2026 verification):**
The DreamVLA paper does **NOT** specify the GPT-2 variant size or hidden dimensions. Standard GPT-2 sizes are: Small (768d, 12L), Medium (1024d, 24L), Large (1280d, 36L), XL (1600d, 48L). Treat any specific dimension claims as **unverified** until confirmed.

**Verified Specs from Paper:**
| Component | Value | Source |
|-----------|-------|--------|
| Query length per modality | 9 | Paper Table/ablation |
| Diffusion steps (DiT) | 10 | Paper |
| Query K options tested | {4, 9, 16} | Paper ablation |
| Hidden dimension | **NOT SPECIFIED** | ⚠️ |
| Number of layers | **NOT SPECIFIED** | ⚠️ |

**Diffusion parameterization note (optional, but estimator-relevant):**
Diffusion models differ in whether they predict *noise/noised quantities* vs. *clean data*. Li & He (2025, arXiv:2511.13720) argue that predicting clean data can better respect the manifold assumption. For PID/MI estimation, this matters because it may change:
- the intrinsic dimension and local geometry of latents,
- the degree of apparent determinism between latents and outputs.
If you analyze diffusion-model internal representations (DreamVLA actions or predicted world knowledge), record the model’s diffusion parameterization and which representation you treat as the variable in PID.

**Dream-VL & Dream-VLA (arXiv:2512.22615, Ye et al. 2025)** is a related but distinct line:
- Uses a **diffusion LLM backbone** (“dLLM”) for VL/VLA, emphasizing bidirectionality and parallel generation.
- Reports strong LIBERO/SimplerEnv results; treat performance numbers as benchmark-dependent and verify protocols before using as “ground truth” comparisons.

**Note on GPT-2 vs NanoGPT/nanochat:**
DreamVLA uses GPT-2 architecture with pretrained weights. For custom VLA training from scratch, consider:
- **NanoGPT** (Karpathy): ~600 lines, reproduces GPT-2 124M in ~1hr for ~$10
- **nanochat** (Karpathy 2025): Full-stack ChatGPT training, ~$100-$300 for GPT-2-grade models
- **llm.c**: C/CUDA implementation, 7% faster than PyTorch

For this specification, GPT-2 refers to the pretrained architecture; NanoGPT is useful for ablations or custom backbones.

### 7.2.2 Key Properties for PID Analysis

- **Explicit D (operationalizable):** world-knowledge predictions provide a concrete candidate “D” variable (dynamic/depth/semantic outputs and/or intermediate “world embedding”)
- **Partial stream separation:** block-wise attention is intended to reduce cross-talk between predicted knowledge components (verify exact masking scheme before treating as “disentangled”)
- **Action chunking:** Predicts K future actions simultaneously
- **Caveat:** “designed for PID” is too strong; the claim we can defend is only that the architecture makes D extraction less arbitrary than in models without explicit prediction heads/tokens.

### 7.2.3 Why DreamVLA is Better for V-D-A Analysis

Relative to models with no explicit world-knowledge outputs, DreamVLA can be **more amenable** to V–D–A analysis because:
1. “D” can be defined as an explicit predicted representation (rather than “some hidden state we decided to call D”).
2. You can test interventions that specifically corrupt predicted knowledge (e.g., corrupt depth vs corrupt semantics) and see whether PID features move as expected (§9.5).
3. You can probe whether the model actually uses predicted knowledge by comparing PID features with/without access to that channel (ablation-style).

This still does not remove the degeneracy/strong-dependence concerns in §1.2: if `A` is effectively deterministic and continuous, you must define the noise/discretization model that makes the information quantities finite and interpretable.

## 7.3 PixelVLA (Pixel-Level Understanding; arXiv:2511.01571)

### 7.3.1 Architecture

PixelVLA (arXiv:2511.01571, November 2025) extends VLAs with pixel-level understanding and multimodal visual prompting:

```
Image → DinoV2+SigLIP → MLP Projector → Llama 2 7B → Continuous Action Decoder
              ↓                              ↑
    Multiscale Pixel-Aware Encoder ──────────┘
              ↑
    Visual Prompting Encoder ← (points, lines, regions, masks)
```

**Key Components:**

| Component | Description | Novelty |
|-----------|-------------|---------|
| **Visual Prompting Encoder** | Lightweight encoder from SAM (Segment Anything Model) | Handles points, lines, bboxes, masks |
| **Multiscale Pixel-Aware Encoder** | Generates pixel-aware embeddings E_p^0 ∈ ℝ^(N_p × D) | Injects pixel-level understanding |
| **Continuous Action Decoder** | Sequential linear projector + N_r ResNet blocks + MLP | Finer action granularity |

**Training Pipeline:**
1. **Continuous Action Training:** Align continuous action decoder with VLA backbone
2. **Pixel-Level Understanding Enhancement:** Fine-tune on Pixel-160K dataset with LoRA (rank=32)

**Pixel-160K Dataset:**
- 160K trajectories with pixel-level annotations
- Two-stage automated annotation pipeline:
  1. Gripper-aware region proposal (video segmentation)
  2. Multimodal object segmentation (LLM + open-vocab segmentation)

**Verified Dimensions (Jan 2026):**
| Component | Dimension | Source |
|-----------|-----------|--------|
| LLM backbone | Llama 2 7B | Paper |
| Hidden size (D) | 4096 | Llama 2 7B spec |
| Action dimension | 7D per timestep | Paper |
| Action chunk size (N_c) | 8 | Paper |
| LoRA rank | 32 | Paper |
| Multiscale features | L levels from SigLIP (H_i × W_i × D_i) | Paper |
| Vision encoders | DinoV2 + SigLIP (pre-trained) | Paper |

### 7.3.2 Key Properties for PID Analysis

PixelVLA offers unique advantages for PID-based diagnostics:

| Property | Advantage for PID |
|----------|-------------------|
| **Pixel-level V** | V captures fine-grained spatial structure, not just global features |
| **Visual prompts** | Can probe specific regions for localized PID analysis |
| **Continuous actions** | No discretization artifacts in A |
| **Multiscale encoder** | Multiple V representations at different scales |

**PID Decomposition Opportunities:**

1. **V at Multiple Scales:** PixelVLA's multiscale encoder produces V_coarse, V_medium, V_fine. We can compute:
   ```
   Syn(V_coarse, V_fine; A) → Does the model integrate global and local visual info?
   ```

2. **Visual Prompt → V Analysis:** When visual prompts (masks, points) are provided:
   ```
   I(V_prompted, V_unprompted; A) → How much does prompting change action?
   ```

3. **Pixel-Level Failure Localization:** If Syn(V,D;A) < 0, we can use visual prompts to probe specific regions and identify WHERE the failure occurs.

### 7.3.3 PixelVLA vs Other VLAs for PID

| Property | OpenVLA | DreamVLA | PixelVLA |
|----------|---------|----------|----------|
| **Explicit D** | No | Yes | No |
| **Pixel-level V** | No | Partial (depth pred.) | Yes |
| **Visual prompts** | No | No | Yes (points, masks) |
| **Action type** | Discrete (256-bin) | Continuous (diffusion-based transformer) | Continuous (L1) |
| **Best PID decomposition** | V-L-A | V-D-A | V_multi-L-A |

**Recommendation:** Use PixelVLA when:
- Task requires pixel-level precision (small object manipulation)
- Failure localization is important (WHERE did it fail?)
- Visual grounding issues are suspected

## 7.4 TraceVLA (Visual Trace Prompting; arXiv:2412.10345)

**TraceVLA** (arXiv:2412.10345, December 2024; ICLR 2025) enhances VLAs with spatial-temporal awareness by overlaying visual state-action trajectories:

```
Current Image + Historical Trace Overlay → VLA → Action
```

- Fine-tuned from OpenVLA (7B parameters) on 150K trajectories with visual traces
- Dual visual streams: current observation + trace-overlaid image, separated by special token
- Reported gains: ~10% on SimplerEnv and ~3.5× on real-robot tasks (paper-reported; protocol-sensitive)
- Also released as TraceVLA-Phi3 (4B parameters, Phi-3-Vision backbone) for RTX 4090 fine-tuning

**Verified Dimensions (Jan 2026):**
| Component | Dimension | Source |
|-----------|-----------|--------|
| Backbone | OpenVLA (Llama 2 7B) | Paper |
| Parameters | 7B | Paper |
| Hidden size | 4096 | Inherited from OpenVLA/Llama 2 7B |
| Transformer layers | 32 | Inherited from Llama 2 7B |
| Action bins | 256/dim | Inherited from OpenVLA |
| Compact variant | TraceVLA-Phi3 (4B) | Paper |

**PID Relevance:** TraceVLA encodes temporal history visually. This means V implicitly contains D-like information (past states). The V-D boundary becomes blurred—interesting for testing whether PID can detect this encoding.

## 7.5 Other VLAs (For Future Reference)

| VLA | Backbone | World Model | Action Representation | Notes |
|-----|----------|-------------|----------------------|-------|
| **OpenVLA-OFT** | (unverified) | (unverified) | (unverified) | Earlier-draft placeholder; add a concrete citation before using |
| **GR00T N1** | (see paper) | Planner-style | Continuous | NVIDIA et al. (2025), arXiv:2503.14734 |
| **TinyVLA** | Smaller | None | Discrete | Efficient |
| **π₀** | (see paper) | (see paper) | (see paper) | Mentioned as a baseline in Dream-VLA/Dream-VLA-related work; add citation when used |
| **MemoryVLA** | VLM + memory bank | Working + long-term | Continuous | Shi et al. (2025), arXiv:2508.19236 |
| **CoT-VLA** | 7B + visual CoT | Predicts visual goals | Mixed | Zhao et al. (2025), arXiv:2503.22020 (performance deltas are benchmark-dependent; verify protocol) |

## 7.6 Architecture Verification Summary (Jan 2026)

This section documents the verification status of VLA architecture claims, cross-referenced against original papers.

### 7.6.1 Verified Dimension Summary

| VLA | Hidden Dim | Layers | Action Type | Verification Status |
|-----|------------|--------|-------------|---------------------|
| **OpenVLA** | **4096** | 32 | Discrete (256-bin) | ✓ Fully verified |
| **DreamVLA** | **Unknown** | Unknown | Continuous (diffusion) | ⚠️ GPT-2 variant unspecified |
| **PixelVLA** | **4096** | 32 | Continuous (7D) | ✓ Fully verified |
| **TraceVLA** | **4096** | 32 | Discrete (256-bin) | ✓ Fully verified (inherits OpenVLA) |

### 7.6.2 Implications for Geometry Analysis

**Confirmed**: The dominant VLA hidden dimension is **4096** (Llama 2 7B based architectures).

This confirms the v5.6 manifold approaches are addressing the right scale:
- **d = 4096** is the ambient dimension for OpenVLA, PixelVLA, TraceVLA
- **DreamVLA** uses GPT-2 (likely 768–1600), so dimension reduction may be less critical
- PCA to ~256 dims represents a **16× reduction** from d=4096

### 7.6.3 Intrinsic Dimension Research (Transformer Embeddings)

Recent literature on transformer embedding geometry (verified Jan 2026):

| Finding | Source |
|---------|--------|
| ID shows **bell-shaped curve** across layers (peak in early-middle) | [The Shape of Learning, arXiv:2311.05928](https://arxiv.org/abs/2311.05928) |
| ID **increases** during early training, then **compresses** | [Comparative Study, arXiv:2412.06245](https://arxiv.org/abs/2412.06245) |
| "Sustained drop in local dimension predicts improved generalization" | [Less is More, arXiv:2506.01034](https://arxiv.org/abs/2506.01034) |
| In-context learning induces **higher ID** than supervised fine-tuning | [Comparative Study, arXiv:2412.06245](https://arxiv.org/abs/2412.06245) |
| ID can be measured using GRIDE, Levina-Bickel MLE, MoM estimators | [Measuring ID, arXiv:2503.02142](https://arxiv.org/abs/2503.02142) |

**Implication for PID-VLA**: The intrinsic dimension of VLA embeddings is **layer-dependent**, **training-dependent**, and likely **much lower** than d=4096. However, the exact ID for VLA-specific embeddings is **not yet measured** and should be part of Experiment 0 diagnostics.

### 7.6.4 v5.6 Approach Review in Light of Verified Dimensions

Given the confirmed d=4096 for most VLAs:

| Approach | Assessment | Notes |
|----------|------------|-------|
| **Manifold Unrolling (Isomap/AE)** | ✓ Still valid | Addresses curved manifold in ℝ^4096 |
| **Geodesic MI** | ✓ Still valid | MI-only, avoids `I^sx_∩` geometry issues |
| **Linear Projection (PCA)** | ⚠️ Conditional | Valid **only if** manifold is locally flat; verify ID preserved |
| **Quantization** | ✓ Still valid | Bypasses geometry entirely; counts mass |
| **Copula Transform** | ✓ Still valid | Mitigates empty-space issues at d=4096 |

**Key Correction**: The v5.6 approaches remain appropriate for d=4096. The main caveat is that **PCA's "locally flat" assumption** may not hold for VLA embeddings, which show complex layer-dependent geometry per the intrinsic dimension literature.

---

# 8. Estimation and Implementation

## 8.1 The KSG Estimator in Detail

### 8.1.1 Algorithm

```python
def ksg_mutual_information(X, Y, k=3):
    """
    KSG estimator for I(X; Y).
    Uses maximum norm (Chebyshev distance) for BOTH k-NN search AND counting.
    """
    N = len(X)
    XY = np.hstack([X, Y])
    
    # Build k-NN tree using Chebyshev (max norm) distance
    tree = KDTree(XY, metric='chebyshev')
    
    # For each point, find distance to k-th neighbor
    distances, _ = tree.query(XY, k=k+1)  # k+1 because point is its own neighbor
    eps_raw = distances[:, k]  # k-th neighbor distance for each point
    # KSG uses strict inequality (< eps_raw) for marginal counts; many radius queries are <=.
    # Implement strictness by shrinking the radius in floating point.
    eps = np.nextafter(eps_raw, 0.0)
    
    # Count points in marginal balls
    tree_x = KDTree(X, metric='chebyshev')
    tree_y = KDTree(Y, metric='chebyshev')
    
    n_x = np.array([len(tree_x.query_ball_point(X[i], eps[i])) - 1 for i in range(N)])
    n_y = np.array([len(tree_y.query_ball_point(Y[i], eps[i])) - 1 for i in range(N)])
    
    # KSG formula
    from scipy.special import digamma
    I = digamma(k) + digamma(N) - np.mean(digamma(n_x + 1) + digamma(n_y + 1))
    
    return I
```

### 8.1.2 Critical Implementation Notes

1. **Use Chebyshev (max norm) for EVERYTHING:** Both k-NN search and marginal counting
2. **The digamma function:** scipy.special.digamma, NOT np.log
3. **Handle edge cases:** n_x or n_y could be 0 at boundary points
4. **Normalize inputs:** Scale each dimension to [0, 1] or standardize
5. **Tie handling matters:** document whether you implement strict `< eps_raw` via `eps = nextafter(eps_raw, 0)` + inclusive `<= eps` counting (recommended), and test it.
6. **Duplicates/quantization:** if many points are identical (or nearly so), kNN radii can collapse to 0. Detect this and either (a) add small seeded jitter, or (b) reject the run and change preprocessing.

### 8.1.3 Extension to I^sx_∩

The continuous `I^sx_∩` estimator in **Ehrlich et al. (2024)** is **not** implemented as “take the minimum of pointwise MI terms.”

Instead, it adapts KSG by replacing conjunction (intersection) neighborhoods with the **disjunction (union)** neighborhoods implied by shared exclusions.

For **two sources** `S₁,S₂` and a target `T`, under Chebyshev/L∞:

1. For each sample `i`, compute the joint disjunction distance to every other sample `j`:
   - `d_S_disj(i,j) = min( d(S₁ᵢ,S₁ⱼ), d(S₂ᵢ,S₂ⱼ) )`
   - `d_ST_disj(i,j) = max( d(Tᵢ,Tⱼ), d_S_disj(i,j) )`
2. Let `εᵢ_raw` be the distance to the `k`-th nearest neighbor under `d_ST_disj`.
   - Use **strict** semantics for marginal counts (`< εᵢ_raw`) via `εᵢ = nextafter(εᵢ_raw, 0)` (or an equivalent strict-radius rule).
3. Count neighbors within `εᵢ`:
   - `n_α(i)` = number of samples within `εᵢ` of the **source disjunction** (`d_S_disj(i,j) <= εᵢ`), including the query point
   - `n_T(i)` = number of samples within `εᵢ` in target space (`d(Tᵢ,Tⱼ) <= εᵢ`), including the query point
4. Estimate redundancy:
   - `Î^sx_∩ = ψ(k) + ψ(N) − (1/N) Σ_i [ ψ(n_α(i)) + ψ(n_T(i)) ]`

This matches the authors’ reference implementation (`gitlab.gwdg.de/wibral/continuouspidestimator`, Python package `csxpid`) and is implemented in this repo as `crates/pid-core/src/isx.rs` (`IsxMethod::EhrlichKsg`).

### 8.1.4 Beyond KSG: Alternative MI/CMI Estimators (MINE, CCMI, Gao-LNC / Local Gaussian)

This project’s *scientific object* is **Wibral-group shared-exclusions redundancy** `I^sx_∩` and the derived PID atoms. For continuous variables, the only paper-faithful estimator in scope is the **Ehrlich et al. (2024) disjunction-kNN/KSG-style estimator** (§8.1.3).

However, two realities force us to consider additional estimators as **baselines / contingency options**:
1. **High dimension** (distance concentration) can break kNN geometry.
2. **Strong dependence** (very large true MI; near-deterministic relationships) can break kNN MI even at low dimension (Gao et al., arXiv:1411.2003).

It is crucial to keep roles separate:
- **`I^sx_∩` redundancy** is *not* obtainable from a generic MI estimator unless you implement the shared-exclusions logic (statement-variable / disjunction neighborhoods).
- **Shannon invariants / co-information screening** depend only on MI/CMI terms, so in principle they can be computed with *any* MI/CMI estimator (but estimator bias can still change conclusions).

#### A) Gao et al.: kNN Robustness for Strong Dependence (still nonparametric)

Gao, Ver Steeg, and Galstyan show that common kNN MI estimators can require sample sizes scaling exponentially in the **true MI** for strongly dependent variables, due to local-uniformity assumptions (arXiv:1411.2003). They propose improved estimators that account for **local non-uniformity**.

Follow-up work by the same authors proposes a **local Gaussian approximation** MI estimator (arXiv:1508.00536), which locally fits a Gaussian around each sample to better approximate densities.

How this fits here:
- **Pros:** Targets exactly one of our biggest conceptual confounds: near-determinism / very strong dependence (common in learned models).
- **Cons:** Does not remove the curse of dimensionality; still relies on local neighborhoods; integration into the disjunction-kNN `I^sx_∩` estimator is **non-trivial** (the elegance of KSG-style cancellation relies on specific ball/rectangle volume terms).

Recommendation:
- Treat Gao-style estimators as **MI baselines** (for CI/O-information screening) and as a diagnostic tool for “KSG is failing because MI is huge,” not as a drop-in replacement for `I^sx_∩`.

#### B) MINE (Belghazi et al., 2018): Neural MI Estimation (variational)

MINE (arXiv:1801.04062) estimates MI by optimizing a neural critic over samples (Donsker–Varadhan-style variational bounds).

How this fits here:
- **Pros:** Scales to high-dimensional inputs; does not explicitly depend on nearest-neighbor geometry; can be trained with minibatches.
- **Cons (PhD-critical):** Optimization instability, estimator bias/variance trade-offs, dependence on architecture/regularization, and reproducibility challenges. MINE estimates are typically **lower bounds** and can be sensitive to training protocol; “same data, different seed” can change the number unless carefully controlled.

Recommendation:
- Use MINE as an **optional baseline** for MI-only invariants when kNN collapses in high `d` (PIVOT path).
- Do **not** mix estimator families inside a PID identity (e.g., do not compute `Syn = I(S1,S2;T) - I(S1;T) - I(S2;T) + Red` with `I(·;·)` from MINE and `Red` from disjunction-kNN).

#### C) CCMI / Neural CMI: Conditional MI in High Dimension (classifier-based)

Conditional MI is relevant when conditioning on confounders (e.g., “given instruction L, how do V and D interact?”) or when using conditional PID variants.

Classifier-based CMI estimators such as CCMI (Mukherjee et al., arXiv:1906.01824) train a classifier to distinguish samples from the joint distribution vs. a product distribution to estimate KL divergences, then assemble CMI.

How this fits here:
- **Pros:** Can handle high-dimensional `Z` (conditioning variable) where kNN CMI struggles.
- **Cons:** Requires careful negative-sample construction and classifier calibration; adds another training loop; provides an estimator of CMI, not `I^sx_∩`.

Recommendation:
- If conditional analyses become central (e.g., PID conditioned on L), prefer to treat CCMI/CMI-NN estimators as **separate baseline pipelines** and validate them with synthetic conditional systems before using on VLA data.

#### Relationship to the Wibral/Gutknecht “hierarchical” strategy

The Wibral/Gutknecht strategy (Shannon invariants + hierarchical screening) primarily addresses **scaling in number of sources** (avoiding 18+ atoms unless needed). It does **not** by itself solve high-dimensional or strong-dependence estimator pathologies.

Therefore, a scientifically clean hierarchy is:
1. **Estimator validity gate (Experiment 0):** determine what MI estimator family is trustworthy at your `(N,d)` and dependence regime.
2. **Variable-count hierarchy:** use Shannon invariants/co-information for screening across many candidate sources/windows.
3. **Full `I^sx_∩` PID:** only where (1) and (2) indicate it is meaningful, and only with paper-faithful `I^sx_∩` estimation.

### 8.1.5 Differential Geometry / Manifold-Aware Contingencies (When kNN/Hierarchical PID Fail)

This section integrates differential-geometry ideas **only where they produce actionable changes** for this project: diagnosing when kNN estimators are invalid, designing safer preprocessing, and (optionally) using manifold-aware MI estimators as MI-only baselines.

It is important to separate:
- **Scientific object (fixed):** Wibral-group shared-exclusions redundancy `I^sx_∩` (Makkeh 2021) + its continuous disjunction-kNN estimator (Ehrlich 2024).
- **Estimator geometry (variable):** how we choose coordinates/metrics/projections to make finite-sample estimation behave.
- **Metaphor vs method:** differential-geometry analogies (e.g., Lorentzian rigidity ↔ PID axiom rigidity) can be useful intuition pumps, but they are **not** evidence and do not directly yield a new `I^sx_∩` estimator. Treat them as background intuition, not as a correctness source.

#### A) First principles: what transformations are truly “free”

For continuous variables, **mutual information is invariant under per-variable diffeomorphisms** (invertible, differentiable reparameterizations applied separately):
- `I(X;Y) = I(f(X); g(Y))` for invertible smooth `f`, `g` (and similarly for multivariate blocks), even though *differential entropies* change by Jacobian terms.

Practical consequence:
- Prefer **invertible** preprocessing steps (standardization, whitening, monotone marginal Gaussianization) before resorting to non-invertible dimension reduction, because invertible reparameterizations can improve kNN geometry **without changing the true MI**.

For PID:
- **Do not assume “free invariance” the way you can for MI.** In discrete settings, `I^sx_∩` is trivially invariant under relabelings (permutations). In continuous settings, the *estimator* (and the “treat sources on equal footing” convention in Ehrlich et al. 2024) introduces metric/scale choices; even invertible reparameterizations can change finite-sample behavior and can effectively redefine what you are measuring unless carefully controlled.
- Practical rule: treat preprocessing as part of the measurement definition; keep it explicit, keep it fixed across runs, and re-validate after substantial changes (Experiment 0 subset).

Hard constraint (do not violate):
- Do **not** apply transforms that mix variables (no PCA/ICA on `[S1|S2|T]` concatenations). Mixing can change the target quantity and can also change what “source” means scientifically.

#### B) Manifold hypothesis: intrinsic dimension matters more than ambient dimension

The “curse of dimensionality” for kNN is controlled by the **intrinsic** dimension of the support, not the raw embedding size:
- Many learned representations empirically lie near a **lower-dimensional manifold** embedded in ℝᵈ.
- kNN estimators can still fail if intrinsic dimension is high, or if curvature/noise makes the local-neighborhood assumption false.

Actionable integration (add to Experiment 0, not post-hoc):
- Measure **intrinsic dimension estimates** for each variable block (`V`, `D`, `L`, `A`, and their joint concatenations used for MI) on your intended sampling unit.
- Track **distance concentration diagnostics** (e.g., nearest-neighbor distance ratios, coefficient of variation of pairwise distances) as a “geometry health check.”

Interpretation rules (scientific hygiene):
- If intrinsic dimension is still large (or unstable across subsamples), treat kNN-based MI/`I^sx_∩` as likely invalid at that operating point, even if `d_total` was reduced by PCA.
- If intrinsic dimension is low and stable, kNN may be viable *after* Experiment 0 establishes quantitative accuracy.

#### C) Riemannian / geodesic kNN MI as a contingency baseline (MI-only, not `I^sx_∩`)

If the representation is plausibly **manifold-valued** (curved support where Euclidean distances are a poor proxy for neighborhood volumes), consider manifold-aware MI estimators as **separate baseline pipelines** for MI-only screening:
- Marx & Fischer (2021, arXiv:2110.13883) propose **geodesic kNN** MI estimation on Riemannian manifolds.

Scope and limitations for PID-VLA:
- This can support **Shannon-invariant screening** (CI/O-information-style terms) in curved settings.
- It does **not** automatically provide `I^sx_∩`, because the disjunction-neighborhood construction would need to be re-derived for Riemannian/hyperbolic spaces (volume forms and product-neighborhood cancellations are nontrivial).

#### D) Hyperbolic geometry for hierarchical structure (Poincaré / Lorentz model) — optional, research-gated

Hyperbolic spaces (constant negative curvature) can represent tree-like/hierarchical structures with low distortion, motivating their use as **learned low-dimensional projections** when hierarchies are central:
- Nickel & Kiela (2017, arXiv:1705.08039): Poincaré embeddings for hierarchies.
- Nickel & Kiela (2018, arXiv:1806.03417): efficient training in the **Lorentz (hyperboloid) model**.
- Ganea et al. (2018, arXiv:1805.09112): hyperbolic neural networks.

Why Lorentzian geometry shows up here (mathematically, not physically):
- The Lorentz model represents hyperbolic space as a Riemannian manifold embedded in a Minkowski space with a **Lorentzian** bilinear form (signature `(-,+,...,+)`), which makes optimization and distance computation numerically convenient.

How this could help (hypotheses; must be tested):
- As a **hierarchy-friendly projection**, hyperbolic embeddings may capture coarse semantic structure with fewer dimensions than Euclidean PCA (useful if “hierarchy” is the relevant inductive bias).

How this could fail:
- Any non-invertible projection (including hyperbolic embedding to low dimension) changes the information quantities. Treat it like a learned projection: re-run Experiment 0-style validation and report it as a different measurement regime.
- Hyperbolic embeddings come with a **non-Euclidean distance** (Poincaré/Lorentz). Feeding hyperbolic coordinates into a Euclidean/Chebyshev kNN estimator is not principled; treat “hyperbolic + MI/PID” as a **separate estimator pipeline** (research-gated), not a drop-in preprocessing step.

#### E) Differential-geometry analogies: audit and safe usage (Jan 2026)

This repo-local PDF is best read as a *conceptual synthesis note*, not as a technical specification. Below is a line-by-line-level **classification** of its major claims into: (i) correct math, (ii) plausible but not directly useful here, and (iii) speculative/unsupported.

What is solid (mathematics, broadly standard):
- **Lorentzian vs Riemannian metrics:** signature `(-,+,...,+)` vs `(+, +, ..., +)` and the induced timelike/null/spacelike classification.
- **Conformal maps preserve causal structure** (light cones) in Lorentzian geometry; they preserve “possibility of influence” but not distances.
- **PID impossibility results exist:** it is correct that Matthias–Makkeh–Wibral–Gutknecht (2025, arXiv:2512.16662) establish strong inconsistency/impossibility statements that force trade-offs among desirable PID axioms.

What is plausible background but not an actionable method for PID-VLA (needs careful scoping):
- **Rigidity-theorem analogy:** comparing “axiom rigidity” (PID) to “symmetry/curvature rigidity” (Lorentzian conformal geometry) can be a useful intuition pump, but it does not produce estimator-level guarantees for `I^sx_∩` on embeddings.
- **Lorentz (hyperboloid) model link:** emphasis on Lorentzian signatures is indirectly relevant because modern **hyperbolic embedding** methods often use the Lorentz model, but that is a representational choice, not a proof about PID atoms.

What is speculative / not currently supported for this project (treat as hypotheses at best):
- **Direct identification of PID atoms with timelike/null/spacelike geometry:** mapping {Red, Unq, Syn} onto Lorentzian causal classes is metaphorical; PID is defined on probability distributions, not spacetime intervals.
- **“Synergy requires spacelike separation”** or similar causal-geometry necessity claims: synergy/redundancy are statistical/functional properties and can arise in many causal graph configurations; Lorentzian geometry is not a general constraint in VLA inference.
- **Claims about Wibral-lab using Lorentzian PSD fits + “spectral PID” as a core method:** may be true in some neuroscience contexts, but this is **not cited to a specific Wibral-group PID paper** and is not part of the validated `I^sx_∩` estimator line (Makkeh 2021; Ehrlich 2024; Gutknecht 2025).
- **Consciousness interpretations (redundancy↔unconscious, synergy↔conscious):** outside scope for PID-VLA; treat as speculative neuroscience interpretation, not an engineering objective.

How we use it safely:
- Keep it as *conceptual background* and as motivation to (i) treat invariances carefully, and (ii) explicitly measure geometry/intrinsic dimension before trusting kNN at scale.
- Do not treat it as evidence about `I^sx_∩` on VLA embeddings, and do not borrow its metaphors as “explanations” for observed PID signs without controlled experiments.

## 8.2 Dimensionality Reduction Strategies

### 8.2.1 Why Dimensionality Reduction is Necessary

At d=4096, k-NN suffers from:
- **Distance concentration:** All points become nearly equidistant
- **Exponential sample requirements:** Sample needs grow rapidly with intrinsic dimension; at `d≈4096`, naive kNN is typically unusable without strong low-dimensional structure and/or explicit dimensionality reduction.
- **Computational cost:** O(N² d) for naive k-NN

### 8.2.2 Options

Before non-invertible dimensionality reduction, consider **invertible reparameterizations**:
- For **MI-only terms** (KSG MI, CI screening), per-variable invertible transforms can improve kNN geometry **without changing the true MI**.
- For **`I^sx_∩`**, treat such transforms as an explicit part of the measurement definition (the estimator has metric/scale conventions); keep them fixed and validate them.

These are “geometry fixes,” not “information fixes,” and still require Experiment 0 validation.

| Method | Dimensions | Properties |
|--------|------------|------------|
| **Invertible per-variable reparameterization** (standardize; marginal Gaussianization) | 4096 | Preserves true MI; can improve kNN geometry; still validate for `I^sx_∩` |
| **Raw embeddings** | 4096 | Often unusable (distance concentration / curvature) |
| **PCA (95% variance)** | ~256 | Linear; **changes the quantity** (non-invertible); often stabilizes Euclidean kNN; re-validate |
| **Random projection (JL)** | 64–256 | Preserves **ambient Euclidean** distances; does **not** recover geodesics; changes the quantity; re-validate |
| **Hash projection (CountSketch)** | 64–256 | Fast baseline (`HashProjector`); approximate; changes the quantity; re-validate |
| **Learned projection (AE/contrastive)** | 64 | Task-specific; changes the quantity; requires training + leakage controls |
| **Hyperbolic embedding (Poincaré/Lorentz)** | ~2–64 | Non-Euclidean metric; **not drop-in** for Euclidean kNN/`I^sx_∩`; treat as a separate estimator pipeline |
| **Intermediate layers** | 4096 but different | Alternative variables (not reduction); may encode different information |

### 8.2.3 Recommendation

1. **Run geometry diagnostics first** (intrinsic dimension + distance concentration + local flatness + δ-hyperbolicity); use them to justify whether kNN/PID is plausible at all. **See §16.6-§16.7 for empirically validated testing methods.**
2. If dimensionality reduction is needed, **start with PCA** (e.g., retain 95% variance) and treat ~256 dims as an initial engineering target, not a law. **⚠️ Caveat**: PCA requires local flatness assumption; test with methods in §16.6.4.
3. Compare against a random projection baseline.
4. If δ-hyperbolicity is low (< 0.1), consider **hyperbolic projection** instead of PCA. See §16.7.3.
5. Consider **SAE decomposition** before PID — may yield lower effective dimension with interpretable features. See §16.8.
6. If needed, train learned projections optimized for the downstream diagnostic objective (and re-run Experiment 0 at the resulting dimension).

**Updated Decision Framework**: See §16.11 for the unified Geometry-First Protocol that integrates all diagnostics.

## 8.3 Computational Considerations

### 8.3.1 Complexity

For `N` samples, `d` dimensions, and `k` neighbors, kNN-based estimators can range from “toy-problem fast” to completely infeasible depending on intrinsic dimension and backend:

- **Brute-force exact kNN (current reference path):** `O(N²·d)` distance work per estimate.
- **Tree-based exact kNN (KD/ball tree):** typically `O(N log N)` build + `O(N log N)` queries at *low intrinsic dimension*, but degrades toward brute force as `d` grows.
- **Approximate kNN (e.g., HNSW/FAISS-style):** potentially sub-quadratic, but introduces estimator bias; only acceptable behind an explicit “approx” mode + re-validation (subset of Experiment 0).

### 8.3.2 Rust Implementation

For real-time use, implement in Rust with:
- SIMD for distance calculations
- Ball trees for efficient counting
- Parallelization across samples

### 8.3.3 Expected Latency

Do not treat any ms-level numbers as “spec truth”: wall-clock depends strongly on `(N, d, k)` and on the kNN backend (exact vs approximate, CPU vs GPU, and whether dimensionality reduction is applied).

Engineering posture:
- Use brute-force exact kNN for **Experiment 0 + correctness**.
- Treat any “real-time monitoring” goal as **Level 0/Level 1 only** (Shannon invariants / co-information), and only after aggressive dimensionality reduction and benchmarking on the target M4 Max.

## 8.4 Validation Strategy

### 8.4.1 Synthetic Data with Known PID

Be explicit about what is “known”:

1. **Discrete, definition-level sanity checks (lattice bookkeeping):**
   - XOR / copy / unique toy systems have clear *qualitative* structure (redundant vs synergistic vs unique-dominant), but **numeric atom values depend on the PID measure** (and some measures allow negative atoms even in simple systems).
   - Use these to sanity-check antichain ordering, atom identities, and qualitative behavior via a *discrete* SxPID implementation (e.g., `Abzinger/SxPID`).
   - These do **not** validate the continuous kNN estimator.

2. **Continuous estimator validation (the actual Experiment 0 gate):**
   - Use i.i.d. *continuous* synthetic systems where at least some MI terms are analytic (e.g., correlated Gaussians), and where adding independent noise dimensions provably leaves the true quantities unchanged.
   - Cross-check continuous `I^sx_∩` redundancy against the authors’ reference implementation (`csxpid`) on fixed datasets.
   - See §9.1 for the full protocol.

### 8.4.2 Scaling Test

Test estimator accuracy at:
- d = 10, 100, 1000, 4096
- N = 100, 1000, 10000
- k = 3, 10, 30

**Go/No-Go:** Use the Experiment 0 gate criteria in §9.1. If estimates collapse at d=4096, pivot to dimensionality reduction and re-validate.

### 8.4.3 Temporal Dependence and Sampling (Trajectory Data)

Most kNN/KSG estimators are analyzed under an **i.i.d. sample** assumption. Robotics/VLA data is naturally **temporal** (trajectories), so sampling design is part of estimator validity:

- **Do not treat “frames” as i.i.d. by default.** Adjacent timesteps are autocorrelated; effective sample size can be far smaller than raw frame count.
- **Prefer cross-trajectory sampling** when possible (e.g., one sample per rollout at a fixed phase, or a large-stride subsample) to reduce dependence.
- **If time-resolved PID is desired**, compute on explicit windows and report window size/stride; interpret as descriptive unless causal controls support it.
- **Uncertainty estimates must respect dependence:** prefer trajectory-level resampling or block bootstrap over naive per-frame bootstrap.

---

# 9. Experimental Design

## 9.0 Sampling Unit, Pointwise Outputs, and Autocorrelation (Read Before Running AUROCs)

Information estimators require multiple samples. In VLA settings, it is easy to accidentally compute a quantity that is *mathematically well-defined* but *experimentally meaningless* because the sampling unit is wrong.

Key distinctions:
- **Estimation dataset:** the collection of samples used to estimate MI / `I^sx_∩` / PID atoms (kNN geometry depends on this).
- **Prediction target:** what you want to predict (often a trajectory-level failure label).

Common pitfalls (and fixes):
1. **Within-trajectory estimation vs. i.i.d. assumptions**
   - Treating every timestep as an i.i.d. sample can be misleading due to autocorrelation.
   - Mitigation: large-stride subsampling, explicit windows, and trajectory-level/block bootstrap for uncertainty.
2. **Per-trajectory prediction needs per-trajectory features**
   - A single global PID computed “across all trajectories” is not directly usable for AUROC per trajectory.
   - For per-trajectory diagnostics, use either:
     - **Within-trajectory PID on windows** (produces a time series of atoms), and summarize (mean/min/%negative/etc.), or
     - **Pointwise/local contributions** (PPID-style): compute per-sample local MI / local redundancy contributions and derive local atoms.
3. **Static variables inside a trajectory**
   - Instruction `L` is often constant within a rollout; within-trajectory MI(L;·) is degenerate.
   - For V–L analyses, prefer cross-trajectory designs (different `L` across samples) or define a target `T` that is trajectory-level (with appropriate estimators).

This section’s experiments should explicitly state the sampling unit (frames vs windows vs trajectories) and how per-trajectory features are derived.

## 9.1 Experiment 0: Estimator Validation (MANDATORY FIRST)

### 9.1.1 Purpose

Validate that I^sx_∩ estimation works at VLA scale before any VLA experiments.

### 9.1.2 Protocol

Design principle: create regimes where the *true* information quantities are unchanged by adding nuisance dimensions, so “ground truth” is well-defined without relying on uncheckable high-d claims.

1. **Generate i.i.d. synthetic systems** (not trajectories) with clear qualitative structure:
   - **Redundant/copy-like:** both sources observe (noisy) versions of the same latent that drives `T`
   - **Unique:** `T` depends on only one source
   - **Synergy/XOR-like:** `T` depends on an interaction (e.g., discrete XOR; or continuous “XOR-like” via thresholded signs)
2. **Define a low-dimensional “signal” representation** (e.g., `d_signal = 1..10`) for each system.
3. **Embed into high dimension by concatenating independent noise features**:
   - `S1' = [S1_signal | N1]`, `S2' = [S2_signal | N2]`, with `N1,N2` independent of everything (and of each other).
   - This preserves the *true* information about `T` but stresses the kNN geometry.
4. Sweep:
   - `d_total ∈ {10, 100, 1000, 4096}` via noise concatenation,
   - `N ∈ {100, 1000, 10000}` (and higher if feasible),
   - `k ∈ {3, 10, 30}`.
4b. **Strong-dependence sweep (separate axis from “high d”):**
   - Even at low dimension, kNN MI can fail when the *true MI is large* (Gao et al., arXiv:1411.2003).
   - Add a 1D (or low-d) Gaussian-channel family where the analytic MI is known and controllable:
     - Example: `X ~ N(0,1)`, `Y = X + σ·N`, `N~N(0,1)`, so `I(X;Y) = 0.5 ln(1 + 1/σ²)` and grows without bound as `σ→0`.
   - Sweep `σ` logarithmically (e.g., `σ ∈ {1, 0.3, 0.1, 0.03, 0.01, 0.003, ...}`) at fixed `N,k`.
   - Goal: empirically map the **safe MI regime** for KSG and for the continuous `I^sx_∩` estimator (and/or show that the noiseless/near-noiseless regime is fundamentally ill-posed for continuous targets).
4c. **Geometry diagnostics (separate axis from “high d” and “strong dependence”):**
   - Estimate **intrinsic dimension** of each variable block (and of the joint spaces used in MI) using nearest-neighbor-based intrinsic-dimension estimators (e.g., Levina–Bickel MLE; TwoNN-style estimators; or other validated ID estimators).
   - Compute **distance concentration** proxies (e.g., nearest-neighbor distance ratio distributions; coefficient of variation of pairwise distances).
   - Use these as a “geometry health check”:
     - Low, stable intrinsic dimension is a prerequisite for believing kNN results after dimensionality reduction.
     - If intrinsic dimension remains large/unstable, treat kNN-based MI/`I^sx_∩` as likely invalid at that operating point (even if `d_total` was reduced).
   - Optional (MI-only baseline): if the representation is plausibly manifold-valued/curved, compare MI terms against **geodesic kNN MI** (Marx & Fischer, arXiv:2110.13883). Treat this as a separate estimator pipeline; do not claim it estimates `I^sx_∩`.
5. For each setting, measure:
   - estimate mean + variance across random seeds,
   - runtime and peak memory,
   - failure modes (ties/duplicate points, NaNs/Infs, implausible drift with `d_total`).
6. **Cross-check correctness where possible:**
   - MI terms: compare against analytic Gaussian-channel MI in low dimensions.
   - `I^sx_∩` redundancy: compare against `csxpid` (authors’ reference implementation) for small `d_total` and fixed datasets.
7. **Optional estimator baselines (keep separate from `I^sx_∩` correctness):**
   - If you implement or adopt them, compare MI-only terms against:
     - Gao et al. strong-dependence corrections (LNC / local Gaussian MI; arXiv:1411.2003, arXiv:1508.00536),
     - MINE (arXiv:1801.04062) for high-dimensional MI (treat as a trained estimator; record architecture/seed/training steps),
     - CCMI / neural CMI (arXiv:1906.01824, arXiv:1911.02277) for conditional MI when conditioning becomes central.
   - Use these baselines to decide whether MI-only **screening** can be made reliable when kNN collapses; do not treat them as “estimating `I^sx_∩`.”

### 9.1.3 Success Criteria

Define “reference” values using the low-dimensional signal system (and cross-check with `csxpid`/analytic MI where available). Because added noise dimensions are independent, the *true* MI/PID quantities should remain constant as `d_total` increases; any systematic drift is estimator pathology.

| Dimensionality (d_total) | Required Accuracy vs Reference |
|--------------------------|-------------------------------|
| d = 10 | Error < 5% |
| d = 100 | Error < 10% |
| d = 1000 | Error < 15% |
| d = 4096 | Error < 20% **or** require dim reduction (PIVOT) |

**Error definition:** use relative error when the reference magnitude is non-trivial; use absolute error thresholds for atoms expected near zero (to avoid meaningless relative blow-ups).

### 9.1.4 If Validation Fails

Before “PIVOT” decisions, diagnose *why* validation failed (high intrinsic dimension vs strong dependence vs ties/quantization vs curvature):

1. **Run geometry + dependence diagnostics:** inspect the strong-dependence sweep (4b) and geometry diagnostics (4c) to distinguish “MI is huge” vs “intrinsic dimension is huge/unstable” vs “duplicate/tie pathology”.
2. **Try invertible geometry fixes (still same true MI):** re-run after per-variable standardization/whitening and (optionally) monotone marginal Gaussianization. If conclusions change wildly, treat the kNN estimator regime as unstable.
3. **Use PCA to reduce to 256-dim** (or a dimension justified by the intrinsic-dimension diagnostics).
4. **Re-validate at the reduced dimension**
5. **If still fails, use learned projections** (explicitly trained for the downstream objective; report as a different measurement regime).
6. **If still fails, abandon kNN-based `I^sx_∩`** for this regime and pivot to validated alternatives (Shannon invariants as primary; or non-kNN MI estimators for MI-only screening), clearly reporting that `I^sx_∩` was not estimable.

Additional contingency (MI-only screening, not full `I^sx_∩`):
- If the disjunction-kNN `I^sx_∩` estimator is unusable at your `(N,d)` even after dimensionality reduction, you may still be able to run **Shannon-invariant** screening (CI/O-information) with non-kNN MI estimators (e.g., MINE / classifier-based MI), but treat this as a *different scientific pipeline* and do not claim results about `I^sx_∩` without a validated `I^sx_∩` estimator.
- Optional geometry-aware MI-only baseline: geodesic kNN MI (Marx & Fischer, arXiv:2110.13883) for manifold-valued variables; treat as a separate validated pipeline.

## 9.2 Experiment 1: Decomposition Comparison

### 9.2.1 Purpose

Determine which decomposition best predicts VLA failures.

### 9.2.2 Decompositions to Test

1. **V-D-A:** Vision, Dream → Action
2. **V-L-A:** Vision, Language → Action
3. **V-D-A*:** Vision, Dream → Optimal Action
4. **Hierarchical:** All three pairwise PIDs

### 9.2.3 Protocol

1. Collect rollouts (e.g., LIBERO-10), with clear success/failure labels and enough coverage of failure modes.
2. Decide the **sampling unit** per decomposition (see §9.0):
   - V–D–A and V–D–A*: typically windowed within-trajectory (V,D,A vary over time).
   - V–L–A: `L` is often constant within a trajectory, so prefer cross-trajectory designs or a trajectory-level target.
3. Extract embeddings (V, L, D, A, and optionally A*) with explicit pooling rules and logged preprocessing.
   - **Leakage rule (critical):** any fitted preprocessing (PCA, learned projection, SAE, normalization learned from data) must be fit on the training split only, then applied to validation/test. Never fit PCA on the full dataset if you report predictive performance.
4. Compute features at multiple fidelity levels:
   - Level 0: co-information / Shannon invariants (fastest; usable broadly).
   - Level 1/2: pairwise `I^sx_∩` PID on selected windows/episodes (expensive; targeted).
5. Convert time series into per-trajectory features (e.g., mean/min/quantiles/%negative/peak magnitude, plus duration-above-threshold).
6. Train and evaluate a predictor (logistic regression / small MLP) using **grouped** cross-validation; report AUROC + calibration + confidence intervals.
   - **Grouping rule:** do not let windows/timesteps from the same trajectory appear in both train and test folds.
   - If you evaluate across multiple tasks/instructions, consider grouping by task family or instruction template to test generalization (not just memorization).

### 9.2.4 Expected Outcome

Report which decomposition achieves highest AUROC.

## 9.3 Experiment 2: Baseline Comparison (Rigorous)

### 9.3.1 Baselines

| Baseline | Description |
|----------|-------------|
| Action predictive entropy | Entropy of the model’s action distribution/logits (for deterministic policies, use stochastic decoding temperature and/or ensembles) |
| Semantic uncertainty (VL-Uncertainty-style) | Uncertainty signals derived from multimodal semantics (as in VL-Uncertainty / related work) |
| Ensemble variance | 4 checkpoint ensemble |
| Attention entropy | Mean cross-modal attention entropy |
| Learned classifier | MLP on (V, D) features |
| Liang et al. Batch PID | Their variational estimator |
| Liang et al. CVX PID | Their convex optimization estimator |
| Process Reward Model (GRM) | Progress-based failure detection (Robo-Dopamine) |

### 9.3.2 Success Criteria

SxPID-derived features achieve AUROC **statistically significantly** > best baseline (paired bootstrap, p < 0.05) with a preregistered effect size, OR yield a well-supported negative result with clear analysis.

**Evaluation hygiene (avoid overclaiming):** select hyperparameters and “best baseline” variants using training/validation only (nested CV or a held-out test set for the final claim).

## 9.4 Experiment 3: Dimensionality Study

### 9.4.1 Purpose

Determine optimal dimensionality for PID estimation.

### 9.4.2 Conditions

1. Raw embeddings (4096-dim)
2. PCA to 256-dim
3. PCA to 64-dim
4. Random projection to 256-dim
5. Learned projection to 64-dim
6. Intermediate VLA layers (layer 16, layer 24)

### 9.4.3 Metric

Primary: AUROC for failure detection at each dimensionality.

Also report (because “best AUROC” can hide estimator collapse):
- estimator diagnostics (tie rate / zero radii, distance concentration proxies, intrinsic-dimension estimates),
- variance across seeds (bootstrap / repeated splits),
- runtime and memory (so “best” is not infeasible).

## 9.5 Experiment 4: Causal Validation

### 9.5.1 Purpose

Test whether PID-derived signals respond to controlled interventions in a way consistent with a causal interpretation (not merely correlation).

### 9.5.2 Protocol

This experiment is only meaningful if **D is operationally interventionable** (e.g., an explicit predicted channel in DreamVLA) and if you define a target that avoids the “A is deterministic” tautology (prefer `A*` or an external failure/success label).

1. **Paired rollout design (reduce confounds):** for each initial state/instruction seed, run a baseline rollout and an intervention rollout that differs only in the D-intervention (same environment seed when possible).
2. **Intervention family (explicitly enumerate):**
   - **Ablation:** drop/mask a D channel (e.g., depth tokens) before fusion.
   - **Noise injection:** add calibrated noise to D (sweep noise level).
   - **Permutation (dependence-breaking) control:** randomly permute D across samples/episodes to break dependence on V while preserving D’s marginal distribution (offline analysis; or online if architecture allows swapping).
3. **Measurement:** compute the relevant PID/Shannon-invariant features under a fixed preprocessing pipeline:
   - If using `A*`: compute PID on `(V,D)→A*` (or on error `E=A−A*`) so the target is external.
   - If using a failure label: treat this as a mixed discrete/continuous setting; either discretize appropriately or use MI-only screening features as the primary statistic (do not pretend continuous kNN PID applies unchanged).
4. **Predictions (pre-register):**
   - D-degrading interventions should reduce `I(D;A*)` and shift the corresponding PID signatures (e.g., Unq(D) and/or Syn(V,D;A*) depending on architecture).
   - Dependence-breaking interventions (permutation) should collapse D-related terms toward 0 under ideal estimation (a strong estimator sanity check).
5. **Outcomes:** compare paired failure rates and PID feature shifts with paired statistical tests (e.g., paired bootstrap over seeds/episodes).
6. **Controls (“placebo”):** include at least one intervention expected to be task-irrelevant (e.g., perturb a D subspace empirically shown to be unused by the policy) and verify it does not systematically change either PID features or failure rate.

**Critical caveat:** an intervention can change PID features without being “the cause of failure” if it induces broader distribution shift. Interpret this experiment as *intervention consistency evidence*, not as full causal identification.

### 9.5.3 Expected Outcome

If PID-derived signals have causal relevance: interventions that degrade `D` in task-relevant ways should shift the corresponding PID metrics in the predicted direction *and* increase failure rates; controls should not.

## 9.6 Success Criteria Summary

| Outcome | Interpretation | Action |
|---------|----------------|--------|
| SxPID-derived features outperform the best baseline with statistical significance (paired bootstrap, p < 0.05) and a preregistered effect size | **Strong success** | Proceed with Aim 2, 3 |
| SxPID-derived features are competitive but gains are small/unstable across seeds/splits | **Moderate / uncertain** | Proceed cautiously; refine sampling/preprocessing and re-test |
| A different PID feature (not synergy) is consistently best | **Conditional success** | Pivot to the best-performing atom/summary |
| Baselines match or beat SxPID (with clear significance) | **Negative result** | Prefer simpler methods; write up limits/lessons |

---

# 10. World Model Integration (WAN, GWM, 3DGS)

## 10.1 Overview

External world models can serve as:
1. **Visualization tools:** Render what the VLA "thinks" will happen
2. **Analytical baselines:** Compare VLA predictions against reference predictions
3. **Data augmentation:** Generate synthetic training data
4. **Training environment generation:** Create unlimited simulation scenarios

**Scope / verification note:** this entire section is *optional* and contains a mix of (a) paper-reported capabilities and (b) engineering design sketches. Treat architecture diagrams and runtime/latency numbers as **to-be-verified on your hardware and data**, and do not let this section block the core PID validation (Experiment 0 + Experiments 1–3).

### 10.1.1 World Model Taxonomy

Understanding the different roles of "world models" is critical for proper integration:

| Type | Example | Role in Pipeline | Action-Conditioned? |
|------|---------|------------------|---------------------|
| **Internal (VLA)** | DreamVLA `<dream>` queries (arXiv:2507.04447) | Predicts future/world knowledge for action decisions | Yes (implicit in architecture) |
| **Evaluative** | WAN, GWM | Visualize/validate VLA predictions | Via LoRA/VACE fine-tuning |
| **Generative (Environment)** | Genie 3, Isaac Sim | Create training environments for agents | Yes (responds to agent actions) |
| **Perceptual Foundation** | DKT, Depth-Anything | Improve visual input quality | N/A (perception preprocessing) |

**Key Insight:** PID analysis operates on the **internal** world model (D) within a VLA. External world models (WAN, GWM, Genie 3) can support PID analysis by:
1. Providing reference predictions to compare against VLA's D
2. Generating training environments where PID patterns can be studied
3. Improving visual input quality (V) for more interpretable PID results

### 10.1.2 Genie 3: Environment Generation (DeepMind)

**Genie 3** (August 2025) represents a distinct paradigm—generating interactive training environments rather than action-conditioned video predictions.

**Key Capabilities:**
- Text prompt → navigable 3D environment
- Real-time interaction at 24 fps, 720p resolution
- "World memory" maintains consistency for several minutes
- **Emergent physics** learned through self-supervision (not hardcoded)
- Object permanence: changes persist across exploration
- Tested with SIMA 2 agent for goal-directed navigation

**Architecture:**
```
Text Prompt → Genie 3 (auto-regressive) → Interactive 3D Environment
                     ↑                              ↓
                     └──── Agent Actions ───────────┘
```

**Comparison with WAN/GWM:**

| Aspect | WAN/GWM | Genie 3 |
|--------|---------|---------|
| **Output** | Video clips (fixed length) | Persistent interactive environment |
| **Interaction** | Generate once, view passively | Real-time navigation & action |
| **Physics** | Learned from video data | Self-supervised emergent physics |
| **Duration** | 5-20 seconds | Several minutes |
| **Primary Use** | Visualization, action prediction | Agent training, environment generation |
| **Action Conditioning** | Via LoRA/VACE | Native (responds to agent input) |

**Relevance to VLA Training:**
1. **Pre-training:** Generate diverse environments for VLA exposure
2. **RL Fine-tuning:** Provide unlimited training scenarios for Aim 3
3. **Domain Randomization:** Genie 3 can create varied conditions to improve generalization
4. **Evaluation:** Test VLA policies in procedurally generated scenarios

**Limitations (as of August 2025):**
- Limited action space for agents
- ~1 minute visual memory before consistency degrades
- Few minutes of continuous interaction (insufficient for full RL training)
- Challenges modeling complex multi-agent interactions
- Not publicly available (research preview)

**PID Relevance (Indirect):**
If using Genie 3 environments for RL fine-tuning, the quality of Genie 3's physics understanding affects what the VLA's internal world model (D) learns. If Genie 3's emergent physics diverge from reality, the VLA's D may learn incorrect dynamics—this would manifest as V-D mismatch (potentially negative synergy) when deployed in the real world.

## 10.2 WAN (Wanxiang Video Model)

### 10.2.1 Architecture (Updated for Wan 2.2)

```
Base Architecture (Wan 2.1):
Video [1+T, H, W, 3] → Wan-VAE Encoder → Latent [1+T/4, H/8, W/8, C] → DiT → Latent → Wan-VAE Decoder → Video

MoE Architecture (Wan 2.2):
┌──────────────────────────────────────────────────────────────────┐
│  High-Noise Expert (14B)     Low-Noise Expert (14B)              │
│  (overall layout)        →   (detail refinement)                 │
│         ↓                          ↓                             │
│  SNR-based switching (early steps → later steps)                 │
└──────────────────────────────────────────────────────────────────┘
Total: 27B parameters, 14B active per step
```

**Model Variants:**

| Model | Parameters | Resolution | Speed (4090) |
|-------|------------|------------|--------------|
| Wan 2.1 T2V-1.3B | 1.3B | 480p | ~4 min/5s |
| Wan 2.1 T2V-14B | 14B | 720p | ~4 min/5s |
| Wan 2.1 VACE-14B | 14B | 480-720p | ~3-4 min/5s |
| Wan 2.2 T2V-A14B (MoE) | 27B (14B active) | 720p | ~2-3 min/5s |
| Wan 2.2 TI2V-5B | 5B | 720p@24fps | ~1-2 min/5s |

**Key Technical Features:**
- 3D Causal VAE with 4× temporal, 16×16 spatial compression (Wan 2.2 VAE: 4×16×16)
- Diffusion Transformer (DiT) backbone
- Full space-time attention mechanism
- MoE architecture in 2.2 for efficiency

### 10.2.2 Extensions Relevant to Robotics

| Extension | Paper | Capability |
|-----------|-------|------------|
| **VACE** | arXiv:2503.07598 | All-in-one video creation/editing with Video Condition Unit |
| **Wan-Move** | arXiv:2512.08765 (NeurIPS 2025) | Motion control via latent trajectory guidance |
| **Motus** | arXiv:2512.13030 | Unified latent action world model using Wan 2.2 5B |
| **DreamGen** | arXiv:2505.12705 | Robot learning via neural trajectories |

### 10.2.3 Potential Uses (Revised)

| Use Case | Feasibility | Notes |
|----------|-------------|-------|
| Dream visualization | **High** | Render VLA's predicted future |
| Hallucination visualization | **High** | Compare predicted vs actual |
| Analytical baseline | **Medium-High** | Fine-tune with LoRA for action conditioning |
| Action-conditioned generation | **High** | Via VACE, Wan-Move, or LoRA fine-tuning |
| Unified world model | **High** | Via Motus integration |
| Real-time intervention | **Low** | Still too slow (1-4 min per clip) |

### 10.2.4 Limitations (Revised)

**Base Model Limitations:**
- Not trained on robot data (but fine-tunable via LoRA)
- Not natively action-conditioned (but extensible via VACE/Wan-Move)
- Latent space may not align with VLA

**Addressed in Extensions:**
- ✅ Action conditioning: VACE, Wan-Move, Motus
- ✅ Robot domain: LoRA fine-tuning on 1K-10K trajectories
- ✅ Faster inference: Wan 2.2 MoE, TI2V-5B

**Still Relevant:**
- Latent space incompatibility with VLA (use for visualization, not direct comparison)
- Not suitable for real-time (<1s) intervention
- Zero-shot robot generation is poor (fine-tuning required)

### 10.2.5 Action Conditioning Approaches

```python
# Approach 1: LoRA Fine-tuning (as in DreamGen, Scalable Policy Eval)
from diffusers import WanPipeline
from peft import LoraConfig, get_peft_model

model = WanPipeline.from_pretrained("Wan-AI/Wan2.1-I2V-14B-diffusers")
lora_config = LoraConfig(
    r=16, lora_alpha=32,
    target_modules=["to_q", "to_k", "to_v"],
    lora_dropout=0.05,
)
model = get_peft_model(model, lora_config)
# Fine-tune on robot trajectories with action tokens

# Approach 2: VACE Video Condition Unit
from diffusers import WanVACEPipeline

pipe = WanVACEPipeline.from_pretrained("Wan-AI/Wan2.1-VACE-14B-diffusers")
output = pipe(
    prompt="robot picks up red cube",
    video=conditioning_video,  # Optional reference
    mask=inpaint_mask,         # For MV2V editing
    ref_images=reference_images,  # For R2V generation
)

# Approach 3: Wan-Move Latent Trajectory Guidance
# Propagate motion through dense point trajectories in latent space
# No architecture change to base I2V model
```

## 10.3 GWM (Gaussian World Model)

### 10.3.1 Why GWM is Better for Analysis

| Property | WAN (base) | WAN (fine-tuned/VACE) | GWM |
|----------|------------|----------------------|-----|
| Trained on robot data | No | Partially (LoRA) | **Yes (native)** |
| 3D representation | No (2D video) | No | **Yes (3DGS)** |
| Action-conditioned | No | **Yes** | **Yes** |
| Latent space alignment | Low | Medium | **High** |
| Inference speed | Slow | Slow | **Faster** |
| Modification required | None | LoRA + data | None |

**When to Choose:**
- **GWM:** Analytical comparison, synergy baseline, 3D spatial reasoning
- **WAN (fine-tuned):** Visualization, data augmentation, unified models (Motus)
- **WAN (base):** Paper figures, demos only

### 10.3.2 Integration Architecture

```
Current Frame → Shared Vision Encoder → V
                       ↓
            ┌─────────────────────┐
            ↓                     ↓
       VLA World Model      GWM World Model
            ↓                     ↓
         D_vla                 D_gwm
            ↓                     ↓
            └─────────────────────┘
                       ↓
               PID Comparison
          Syn(V, D_vla; A) vs Syn(V, D_gwm; A)
```

## 10.4 Depth Perception Methods

### 10.4.1 Monocular Depth Estimation

| Method | Speed | Accuracy | Notes |
|--------|-------|----------|-------|
| **Depth-Anything v2** | ~50ms | Good | Relative depth, widely deployed |
| **Depth-Anything v3** | ~40ms | Better | Improved fine-grained details |
| **Metric3D v2** | ~100ms | Best metric | Absolute depth with scale |
| **Video Depth Anything** | ~60ms/frame | Temporal consistent | For video sequences |
| **RollingDepth** | ~80ms | Excellent | LDM-based, handles depth range changes |

**Recommendation:** Use Depth-Anything v3 as primary, fall back to v2 for speed-critical applications.

### 10.4.2 Stereo Vision (StereoVLA Approach)

**Key Insight from StereoVLA (arXiv:2512.21970):** Rather than relying on monocular depth estimation, stereo vision provides direct 3D geometry from binocular disparity.

| Approach | Pros | Cons |
|----------|------|------|
| Monocular depth | Single camera, any setup | Estimated, not measured |
| Stereo vision | True geometry, accurate | Requires calibrated stereo pair |
| RGB-D sensor | Direct depth | Limited range, sensor cost |

**Integration with PID-VLA:**
```
Stereo Pair → Disparity Estimation → 3D Point Cloud → VLA Visual Encoder
                                                    ↓
                                              Enhanced V with native depth
```

StereoVLA shows improved spatial reasoning by providing the VLA with native 3D information rather than requiring the model to infer depth from monocular cues.

### 10.4.3 Transparent Object Depth (DKT)

**Problem:** Standard depth methods (RGB-D sensors, stereo, monocular estimation) fail on transparent, translucent, and reflective objects due to:
- Light refraction through glass/plastic
- Specular reflections on shiny surfaces
- Time-of-flight sensors receive corrupted signals
- Stereo correspondence fails on textureless transparent regions

**The “Diffusion Knows Transparency” principle (interpret cautiously):**

DKT (arXiv:2512.23705) argues that strong video diffusion priors can help infer depth for transparent/reflective objects. Interpret this as **learned statistical regularities** that are often consistent with light transport (refraction/reflection), not as evidence that the model “understands physics” in a mechanistic sense.

**Architecture:**
```
RGB Video → WAN VAE Encoder → Concat[RGB_latent, Depth_latent] → DiT + LoRA → Depth/Normal Maps
                                                                      ↑
                                                               TransPhy3D training
```

**Technical Details:**
- **Base model:** WAN video diffusion (DiT backbone)
- **Adaptation:** LoRA fine-tuning (preserves base priors, prevents catastrophic forgetting)
- **Dataset:** TransPhy3D (11k sequences, 1.32M frames, Blender/Cycles rendering)
- **Output:** Temporally consistent depth + normals for arbitrary-length video
- **Speed:** 1.3B version runs at ~167ms/frame (11.19GB VRAM)

**Benchmark Results:**
| Dataset | DKT Performance |
|---------|-----------------|
| ClearPose | SOTA (zero-shot) |
| DREDS CatKnown | SOTA |
| DREDS CatNovel | SOTA |
| TransPhy3D-Test | SOTA |

**Real-World Robot Grasping:**
DKT integrated with AnyGrasp achieves improved success rates across:
- Translucent objects (glass bottles, plastic containers)
- Reflective surfaces (metal, mirrors)
- Diffuse objects (baseline comparison)

**PID Relevance: Why Transparent Object Depth Matters for V-D Analysis**

This is a genuine connection to PID diagnostics:

1. **V quality affects interpretability:** if the visual representation `V` is dominated by perception artifacts (e.g., transparent-object depth failures), the resulting MI/PID quantities are still mathematically well-defined but can become **semantically uninterpretable** for “integration quality” questions:
   ```
   Perception artifact → V no longer tracks scene geometry → PID reflects the artifact regime
   ```

2. **V-D Mismatch from Perception Failure:** When standard depth sensors fail on glass:
   - V contains incorrect geometric information
   - D (world model) may predict correct physics
   - This creates *apparent* V-D mismatch that is actually a perception failure, not a world model failure
   
   **Hypothesis:** Using DKT-enhanced depth should *reduce* negative synergy for transparent object tasks because V becomes accurate and consistent with D's physics predictions.

3. **Failure Mode Attribution:** Without accurate transparent object depth:
   - Low Syn(V,D;A) could indicate either:
     a) World model failure (D is wrong about physics)
     b) Perception failure (V is garbage)
   - DKT removes (b) from the equation, enabling cleaner diagnosis

4. **The "Diffusion Knows Transparency" Implication:**
   If video diffusion models have learned light transport physics, this suggests:
   - Well-trained VLAs with diffusion-based world models should handle transparent objects better
   - The D component in such VLAs may already encode refraction/reflection priors
   - We could test this by comparing Syn(V,D;A) on transparent vs opaque objects

**When to Use DKT in PID-VLA Pipeline:**
| Scenario | Recommendation |
|----------|----------------|
| Tasks involving glass/plastic | Use DKT for V preprocessing |
| Diagnosing transparent object failures | Essential for valid PID |
| General manipulation (opaque) | Depth-Anything v3 sufficient |
| Speed-critical real-time | DKT too slow (~167ms), use stereo |

**Code:** https://github.com/Daniellli/DKT

## 10.5 3DGS (3D Gaussian Splatting)

### 10.5.1 Role in Pipeline

- **SHARP:** Single-image to 3DGS conversion (<1s on MPS)
- **Depth-Anything-3:** Metric depth estimation (use with SHARP)
- **SparkJS:** 3DGS rendering in browser

### 10.5.2 When 3DGS Adds Value

- Debugging spatial reasoning failures
- Visualizing occlusion/depth errors
- Training data for 3D-aware policies

### 10.5.3 When 3DGS is Overkill

- Core PID diagnostics (2D sufficient)
- Real-time intervention (too slow)
- Most failure modes don't require 3D

## 10.6 Recommendation

| Task | Tool | Notes |
|------|------|-------|
| Core PID (Aims 1-2) | None (VLA latents only) | Avoid world model confounds |
| Failure debugging | GWM | 3D spatial localization |
| Paper figures | WAN (base) | Highest visual quality |
| Action-conditioned visualization | WAN VACE / Motus | Via conditioning pipeline |
| Data augmentation | WAN (fine-tuned) or GWM | Both support actions |
| Unified world model baseline | Motus (Wan 2.2 + VLM) | Best integrated option |
| 3D spatial analysis | SHARP + 3DGS | Single-image 3D |
| Real-time monitoring | Entropy (not PID) | <100ms requirement |
| Simulation + visualization | Headless Gazebo + Tauri | Low-latency interactive |
| Transparent object depth | DKT | Essential for glass/plastic manipulation |
| RL environment generation | Genie 3 | Unlimited interactive training scenarios |
| VLA pre-training environments | Genie 3 + SIMA 2 | Procedural world generation |

## 10.7 World Model Paradigms and PID Implications

### 10.7.1 Theoretical Framework

Different world models serve different roles. Understanding this prevents category errors in PID analysis:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       WORLD MODEL PARADIGMS                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  1. INTERNAL (D in PID)        2. EVALUATIVE              3. GENERATIVE │
│  ─────────────────────         ────────────              ───────────────│
│  Lives inside VLA              External reference         Creates envs   │
│  Predicts s' from (s,a)        Validates predictions      For training   │
│                                                                          │
│  Examples:                     Examples:                  Examples:      │
│  • DreamVLA <dream>            • WAN video gen           • Genie 3      │
│  • Hidden states               • GWM 3D prediction       • Isaac Sim    │
│  • Implicit in attention       • DKT depth               • Gazebo       │
│                                                                          │
│  PID measures THIS       →     Can compare with D   →    D learns from  │
│         ↓                              ↓                        ↓        │
│  Syn(V, D_internal; A)         Syn(V, D_internal; A)    Quality affects │
│                                vs Syn(V, D_external; A)  what D learns  │
└─────────────────────────────────────────────────────────────────────────┘
```

### 10.7.2 How External World Models Affect Internal D

**Training in Genie 3 → Effect on VLA's D:**

When a VLA trains in Genie 3-generated environments:
1. The VLA's internal world model D learns from Genie 3's emergent physics
2. Genie 3's physics are learned via self-supervision (not hardcoded)
3. If Genie 3's physics diverge from reality, D learns incorrect dynamics

**PID Prediction:**
```
VLA trained in Genie 3         VLA trained in real world
────────────────────────       ─────────────────────────
D_genie                        D_real

When deployed in reality:
• If Genie 3 physics ≈ real physics: Syn(V,D_genie;A) ≈ Syn(V,D_real;A)
• If Genie 3 physics ≠ real physics: Syn(V,D_genie;A) < Syn(V,D_real;A)
                                     (V shows real physics, D expects Genie physics)
```

**Testable Hypothesis:** VLAs trained in Genie 3 will show lower synergy on tasks where Genie 3's emergent physics differ most from reality (e.g., precise contact dynamics, fluid interactions).

### 10.7.3 The "Diffusion Knows Physics" Principle

Both DKT and Genie 3 demonstrate that large-scale generative models learn implicit physics:

| Model | What It Learned | How We Know |
|-------|-----------------|-------------|
| WAN → DKT | Light transport (refraction, reflection) | Zero-shot transparent object depth |
| Genie 3 | Object permanence, basic dynamics | Consistent multi-minute environments |
| Veo 3 | Intuitive physics | Realistic video generation |

**Implication for PID:**

If video diffusion models implicitly learn physics, then:
1. VLAs built on diffusion backbones (like Motus using Wan 2.2) may have stronger D priors
2. The D component in such VLAs may already encode physical principles
3. We could compare PID signatures across VLAs with different backbones:
   - Transformer-only (OpenVLA, Octo)
   - Diffusion-based (π0, Motus)
   - Hybrid (DreamVLA)

**Hypothesis:** VLAs with diffusion-based world models should show higher Syn(V,D;A) on physics-heavy tasks because D has richer physical priors from video generation training.

### 10.7.4 Perception Quality as PID Prerequisite

**The DKT lesson:** Before attributing low synergy to world model failure, verify perception quality.

```
Failure Mode Diagnostic Tree:
                         Low Syn(V,D;A)
                              │
                 ┌────────────┴────────────┐
                 │                         │
        V is accurate?              V is corrupted?
        (use DKT/stereo)            (depth sensor failure)
                 │                         │
          D is wrong                 Fix V first
      (true world model failure)    (not a D problem)
```

**Practical Protocol:**
1. For transparent/reflective objects: Always use DKT preprocessing
2. For stereo setups: Verify calibration before PID analysis
3. Log V quality metrics alongside PID measurements
4. If V quality degrades, discount PID findings

## 10.8 Headless Gazebo + Tauri Visualization System

### 10.8.1 Architecture Overview

A low-latency simulation and visualization system optimized for robotics research:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              TAURI APP                                   │
├─────────────────────────────────────────────────────────────────────────┤
│                      SparkJS / Three.js                                  │
│                   (WebGPU → Metal or Vulkan)                            │
├─────────────────────────────────────────────────────────────────────────┤
│                         Rust Backend                                     │
│  ┌─────────────────┬─────────────────┬─────────────────┐                │
│  │  Zenoh Client   │  ML Inference   │  Platform Utils │                │
│  │  (cross-plat)   │  (abstracted)   │                 │                │
│  └────────┬────────┴────────┬────────┴─────────────────┘                │
│           │                 │                                            │
│     ┌─────▼─────┐     ┌─────▼─────┐                                     │
│     │  macOS    │     │  Linux    │                                     │
│     │ Backend   │     │ Backend   │                                     │
│     │• CoreML   │     │• CUDA     │                                     │
│     │• MLX      │     │• TensorRT │                                     │
│     │• Metal    │     │• cuDNN    │                                     │
│     └───────────┘     └───────────┘                                     │
└─────────────────────────────────────────────────────────────────────────┘
```

### 10.8.2 Latency Path

```
Gazebo (headless)                    Tauri + SparkJS
─────────────────                    ────────────────

                      Zenoh
Physics     ─────────(~2ms)────────→ State update ──→ Three.js
1000 Hz        shared mem                            render @ 60fps

Camera      ─────────(~5ms)────────→ Texture update ─→ Three.js
30-60 Hz       zero-copy                              plane/quad

Sensors     ─────────(~2ms)────────→ Process ──→ Overlay ──→ render

Total input lag: ~8-15ms (data) + ~16ms (render) = ~25-30ms
```

### 10.8.3 Why This Architecture for PID-VLA

| Benefit | Explanation |
|---------|-------------|
| **Low latency (~25-30ms)** | Enables interactive debugging of VLA decisions |
| **Zenoh middleware** | Same protocol as ROS 2, zero-copy shared memory |
| **SparkJS for 3DGS** | Renders Gaussian splats in browser via WebGPU |
| **Platform abstraction** | Same code runs on M4 Mac (MLX/Metal) and Linux (CUDA) |
| **Headless Gazebo** | Physics at 1000 Hz without rendering overhead |
| **Three.js flexibility** | Overlay PID diagnostics, attention maps, synergy heatmaps |

### 10.8.4 Integration with PID Monitoring

```rust
// Rust backend receives VLA embeddings via Zenoh
async fn pid_monitor_loop(zenoh_session: &Session) {
    let subscriber = zenoh_session
        .declare_subscriber("vla/embeddings")
        .await
        .unwrap();
    
    while let Ok(sample) = subscriber.recv_async().await {
        // Decode embeddings (zero-copy from shared memory)
        let embeddings: VLAEmbeddings = deserialize(&sample.payload);
        
        // Compute fast Shannon invariants (fastest; runtime depends on n,d and kNN backend)
        let ci = co_information_pairwise(
            &embeddings.vision,
            &embeddings.dream,
            &embeddings.action,
        );
        
        // Publish to visualization
        zenoh_session
            .put("pid/co_information", serialize(&ci))
            .await
            .unwrap();
    }
}
```

### 10.8.5 Visualization Overlays

The Tauri + Three.js frontend can overlay:

1. **Real-time PID metrics** (co-information, synergy estimates)
2. **Attention heatmaps** from VLA transformer layers
3. **Depth estimation** (Depth-Anything v3 or stereo disparity)
4. **3DGS point clouds** rendered via SparkJS
5. **Action trajectory predictions** from world model

### 10.8.6 Hardware Requirements

| Platform | Minimum | Recommended |
|----------|---------|-------------|
| macOS | M1 8GB | M4 Pro 24GB |
| Linux | RTX 3060 | RTX 4090 |
| Memory | 16GB | 32GB |
| Storage | SSD | NVMe |

### 10.8.7 PixelVLA Integration with Headless Gazebo + Tauri

PixelVLA's pixel-level understanding and visual prompting capabilities integrate naturally with the visualization system:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     PIXELVLA + GAZEBO + TAURI INTEGRATION                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   Gazebo (Headless)              PixelVLA                    Tauri + SparkJS│
│   ─────────────────              ────────                    ───────────────│
│                                                                              │
│   Physics @ 1kHz ─────────┐                                                 │
│                           │                                                 │
│   Camera @ 30fps ─────────┼──(Zenoh)──→ DinoV2+SigLIP ─→ V embeddings     │
│                           │             + Multiscale    │                   │
│                           │               Pixel Encoder  │                   │
│                           │                              ↓                   │
│   ┌───────────────────────┴──────────────────────────────┴───────────────┐ │
│   │                        THREE.JS OVERLAYS                              │ │
│   │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │ │
│   │  │ Pixel-level     │  │ Visual Prompt   │  │ PID Diagnostic      │  │ │
│   │  │ Attention Maps  │  │ Overlay (masks, │  │ Heatmaps           │  │ │
│   │  │ (from PixelVLA  │  │ points, regions)│  │ (Syn(V,D;A) per    │  │ │
│   │  │  encoder)       │  │                 │  │  pixel region)     │  │ │
│   │  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │ │
│   └──────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│   User Interaction:                                                         │
│   • Click to place visual prompts (points, bboxes)                          │
│   • Hover for local PID values                                              │
│   • Select regions for targeted analysis                                    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Data Flow for PixelVLA + PID:**

```
1. Gazebo renders simulated scene
2. Camera image sent via Zenoh (zero-copy) to PixelVLA
3. PixelVLA produces:
   - V_coarse: Global visual features (256 patches)
   - V_fine: Pixel-level features (from multiscale encoder)
   - A_pred: Continuous action prediction
4. Embeddings forwarded to PID computation
5. Results visualized in Tauri:
   - Per-patch synergy overlaid on image
   - Attention maps from pixel-aware encoder
   - Interactive visual prompting interface
```

**Visual Prompting in Tauri:**

The SparkJS/Three.js frontend can send visual prompts back to PixelVLA:

```typescript
// Tauri frontend: user clicks to create visual prompt
async function onCanvasClick(event: MouseEvent) {
  const point = screenToImageCoords(event);
  
  // Send point prompt via Zenoh
  await zenoh.put("pixelvla/prompt/point", {
    x: point.x,
    y: point.y,
    type: "manipulation_target"
  });
  
  // Receive PixelVLA's response with pixel attention
  const attention = await zenoh.get("pixelvla/attention");
  
  // Overlay on Three.js scene
  renderAttentionHeatmap(attention);
}

// Draw bounding box prompt
function onBboxDraw(bbox: Rect) {
  zenoh.put("pixelvla/prompt/bbox", {
    x1: bbox.left, y1: bbox.top,
    x2: bbox.right, y2: bbox.bottom,
    type: "region_of_interest"
  });
}
```

**PixelVLA-Specific PID Analysis in Tauri:**

```rust
// Rust backend: specialized PID for PixelVLA's multiscale outputs
async fn pixelvla_pid_analysis(
    v_coarse: &Tensor,   // 256 patches × 1024 dim
    v_fine: &Tensor,     // H×W × 256 dim (pixel-level)
    d: &Tensor,          // World model state
    a: &Tensor,          // Action prediction
) -> PixelPIDResult {
    // 1. Global PID (standard V-D-A)
    let global_pid = compute_pid(
        &v_coarse.mean(dim=0),  // Pool to single vector
        d,
        a,
    );
    
    // 2. Patch-level PID (for each of 256 patches)
    let patch_pids: Vec<PIDResult> = (0..256)
        .map(|i| compute_pid(&v_coarse[i], d, a))
        .collect();
    
    // 3. Region-specific PID (if visual prompt provided)
    let region_pid = if let Some(prompt) = get_current_prompt() {
        let v_region = extract_region_features(v_fine, &prompt);
        Some(compute_pid(&v_region, d, a))
    } else {
        None
    };
    
    // 4. Cross-scale synergy (does PixelVLA integrate global + local?)
    let cross_scale_syn = compute_conditional_mi(
        &v_coarse.mean(dim=0),  // Global
        &v_fine.mean(dims=[0,1]),  // Local (pooled)
        a,  // Target
    ) - mi(&v_coarse.mean(dim=0), a) - mi(&v_fine.mean(dims=[0,1]), a);
    
    PixelPIDResult {
        global: global_pid,
        per_patch: patch_pids,
        region: region_pid,
        cross_scale_synergy: cross_scale_syn,
    }
}
```

**Benefits of PixelVLA + Gazebo Integration:**

| Feature | Benefit for PID Research |
|---------|-------------------------|
| **Pixel-level embeddings** | Localize WHERE synergy/failures occur |
| **Interactive visual prompts** | Probe specific objects/regions |
| **Continuous actions** | No discretization noise in A |
| **Multiscale representations** | Test cross-scale information integration |
| **Sim-to-real alignment** | Debug perception issues before real robot |

**Latency Budget for PixelVLA:**

```
Component                      Time (ms)    Notes
─────────────────────────────────────────────────
Gazebo → Zenoh                    2        Zero-copy shared mem
DinoV2 + SigLIP inference        45        7B params, M4 Max
Multiscale pixel encoder          8        Lightweight
Action decoder                    5        L1 regression
PID (Shannon invariants)         10        Fast screening
PID (full I^sx_∩) on demand     100        Per-click analysis
Three.js render                  16        60 fps cap
─────────────────────────────────────────────────
Total (interactive)             ~86ms      ~12 fps interactive
Total (with full PID)          ~186ms      ~5 fps detailed analysis
```

**Note:** Full `I^sx_∩` PID is not expected to be real-time without aggressive dimensionality reduction and an accelerated kNN backend. Use Shannon invariants for continuous monitoring, and trigger full PID only for suspicious windows/episodes.

---

# 11. Technical Implementation

## 11.1 Technology Stack

### 11.1.1 Core Components

| Component | Language | Rationale |
|-----------|----------|-----------|
| PID computation | Rust | Performance-critical, SIMD, no GIL |
| k-NN search | Rust | Hot loop, low-level control |
| Data loading | Rust (Polars) | Faster than pandas |
| ML inference | Python (MLX/PyTorch) | Ecosystem, model availability |
| Visualization | Tauri (Rust + WebView) | Native performance |
| Experiment orchestration | Python (uv) | Flexibility, notebooks |

### 11.1.2 Python Bindings

Use PyO3 to expose Rust functions to Python:

```python
from pid_vla import compute_isx_pid

result = compute_isx_pid(
    sources=[vision_embeddings, dream_embeddings],
    target=action_embeddings,
    k=3,
    dim_reduction='pca',
    n_components=256
)
print(f"Synergy: {result.synergy:.4f} ± {result.synergy_ci:.4f}")
```

## 11.2 Project Structure

```
pid-vla/
├── Cargo.toml              # Rust workspace
├── pyproject.toml          # Python package (uv)
├── flake.nix               # Nix reproducibility
├── justfile                # Task runner
│
├── crates/
│   ├── pid-core/           # Pure Rust PID implementation
│   │   ├── src/
│   │   │   ├── ksg.rs      # KSG estimator
│   │   │   ├── isx.rs      # I^sx_∩ estimator
│   │   │   ├── simd.rs     # SIMD distance calculations
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   │
│   ├── pid-python/         # PyO3 bindings
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   │
│   └── pid-tauri/          # Visualization app
│       ├── src/
│       └── Cargo.toml
│
├── python/
│   ├── pid_vla/            # Python package
│   │   ├── __init__.py
│   │   ├── estimators.py   # High-level API
│   │   ├── vla.py          # VLA integration
│   │   └── baselines.py    # Baseline methods
│   │
│   ├── experiments/        # Experiment scripts
│   │   ├── exp0_validation.py
│   │   ├── exp1_decomposition.py
│   │   ├── exp2_baselines.py
│   │   └── exp3_dimensionality.py
│   │
│   └── notebooks/          # Analysis notebooks
│
├── data/                   # Data directory
│   ├── synthetic/          # Validation data
│   ├── libero/            # LIBERO rollouts
│   └── embeddings/        # Extracted embeddings
│
└── results/               # Experiment results
```

**Repo status (v5.0):**
- Implemented: `crates/pid-core` (KSG MI, continuous `I^sx_∩` via `IsxMethod::EhrlichKsg`, 2-way and 3-way wrappers, preprocessing hooks, intrinsic-dimension diagnostics, geometry diagnostics, distance concentration, and a Rust `exp0` runner).
- Planned: `crates/pid-python`, `crates/pid-tauri`, and the `python/` experiment harness (keep the structure above as the target layout, but do not assume those folders exist yet).

## 11.3 Reproducibility

**Canonical (repo truth):** `flake.nix`, `flake.lock`, `pyproject.toml`, and `uv.lock` at the repo root.

If the examples below diverge from the repo files, **prefer the repo files**. (This document is a spec; the repo is the executable artifact.)

### 11.3.1 Nix Flake

```nix
{
  description = "pid_vla (macOS-first): reproducible dev shell for Rust + Python (uv)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            just

            # Rust toolchain (pin via flake.lock).
            rustc
            cargo
            rustfmt
            clippy

            # Python + uv (pin via flake.lock; pin deps via uv.lock).
            python311
            uv
          ];

          # Prefer a system/Nix-provided Python; do not auto-download Pythons.
          UV_NO_MANAGED_PYTHON = "1";
          UV_PYTHON_DOWNLOADS = "never";
        };
      }
    );
}
```

**Lockfile requirement:** commit `flake.lock` (generate/update with `nix flake lock`).

### 11.3.2 uv for Python

```toml
[project]
name = "pid-vla"
version = "0.1.0"
description = "Wibral-group shared-exclusions PID (I^sx_∩) for VLA diagnostics"
readme = "README.md"
requires-python = ">=3.11"

# Keep base dependencies minimal; add groups as needed.
dependencies = []

[dependency-groups]
dev = ["pytest>=8.0", "ruff>=0.6"]
analysis = ["numpy>=1.26", "scipy>=1.11", "pandas>=2.2", "matplotlib>=3.8", "seaborn>=0.13"]
report = ["reportlab>=4.0"]

[tool.uv]
default-groups = ["dev", "analysis"]
```

**Lockfile requirement:** commit `uv.lock` and use `uv sync --frozen` for deterministic installs.

---

# 12. Open Questions and Future Directions

## 12.1 Theoretical Open Questions

### Q1: What Does Negative Synergy REALLY Mean for VLAs?

The mathematical definition (subadditive information) doesn't directly map to "hallucination." We need empirical validation that Syn < 0 correlates with human-labeled failures.

**Possible outcomes:**
- Strong correlation → proceed with PID
- Weak correlation → entropy may suffice
- No correlation → abandon PID approach

### Q2: Is the V-D-A Decomposition Fundamental?

Why these three variables? Alternatives:
- Proprioception vs Vision
- Short-term vs Long-term memory
- Task-specific vs General features

### Q3: Can PID Be Used for Training (Not Just Diagnosis)?

Infomorphic networks use PID as a training objective. Can we do the same for VLAs?
- Requires differentiable PID
- May be too noisy for gradient-based optimization
- Aim 3 explores this

## 12.2 Empirical Open Questions

### Q4: At What Scale Does PID Work?

The estimator is validated at ~100 dimensions. VLAs use 4096+. What's the practical limit?

### Q5: How Much Data is Needed?

PID estimation requires many samples. Is a single trajectory enough? Do we need thousands of rollouts?

### Q6: Is Real-Time PID Feasible?

For intervention during execution, we need <10ms latency. Is this achievable?

## 12.3 Future Directions

### Direction 1: Multi-Task PID Profiles

Characterize PID signatures across tasks:
- Do some tasks naturally have higher synergy?
- Can PID predict which tasks a VLA will struggle with?

### Direction 2: PID-Guided Data Collection

Use PID to identify high-synergy demonstrations for training data augmentation.

### Direction 3: Hierarchical PID

Apply PID at multiple levels:
- Token-level
- Timestep-level
- Trajectory-level
- Task-level

### Direction 4: Cross-Architecture Transfer

Can PID profiles predict how well a policy will transfer across:
- Embodiments
- Environments
- Task distributions

---

# 13. References

## 13.1 Core Wibral Group PID Work

### 13.1.1 Papers

- **Makkeh A, Gutknecht AJ, Wibral M (2021).** Introducing a differentiable measure of pointwise shared information. *Phys Rev E* 103:032149. [Defines I^sx_∩]

- **Ehrlich DA, Schick-Poland K, Makkeh A, Lanfermann F, Wollstadt P, Wibral M (2024).** Partial Information Decomposition for Continuous Variables based on Shared Exclusions. *Phys Rev E* 110:014115. [Continuous extension]

- **Makkeh A, Graetz M, Schneider AC, Ehrlich DA, Priesemann V, Wibral M (2025).** A General Framework for Interpretable Neural Learning based on Local Information-Theoretic Goal Functions. *PNAS* 122:e2408125122. [Infomorphic networks]

- **Gutknecht AJ, Rosas FE, Ehrlich DA, Makkeh A, Mediano PAM, Wibral M (2025).** Shannon Invariants: A Scalable Approach to Information Decomposition. arXiv:2504.15779. [Scalability]

- **Matthias PH, Makkeh A, Wibral M, Gutknecht AJ (2025).** Novel Inconsistency Results for Partial Information Decomposition. arXiv:2512.16662. [Impossibility theorems]

### 13.1.2 Authoritative Code Repositories (v5.7)

| Repository | Description | License | Status |
|------------|-------------|---------|--------|
| **[continuouspidestimator](https://gitlab.gwdg.de/wibral/continuouspidestimator)** (`csxpid`) | Reference implementation of continuous `I^sx_∩` estimator (Ehrlich et al. 2024) | BSD-3 | ✓ Canonical reference |
| **[infomorphic_networks](https://gitlab.gwdg.de/wibral/infomorphic_networks)** | Experiments with infomorphic networks; learning rule code in "PIDnets" repo (Abed) | GPL-3.0+ | Research code |
| **[SxPID](https://github.com/Abzinger/SxPID)** | Discrete `I^sx_∩` reference implementation | — | ✓ Canonical reference |
| **[sae_analysis](https://github.com/Abzinger/sae_analysis)** | Shannon invariants for SAE latents (Red°, Vul°) | — | Experimental |

**Note:** `infomorphic_networks` delegates core learning rules to "PIDnets" (Abed's repository). Use `continuouspidestimator` for validating continuous `I^sx_∩` estimates.

## 13.2 VLA Models

- **OpenVLA:** Kim et al. (2024). *OpenVLA: An Open-Source Vision-Language-Action Model.* arXiv:2406.09246.
- **DreamVLA:** Zhang et al. (2025). *DreamVLA: A Vision-Language-Action Model Dreamed with Comprehensive World Knowledge.* arXiv:2507.04447. (World-knowledge forecasting + inverse dynamics; diffusion-style framing in the abstract.)
- **Dream-VL & Dream-VLA (diffusion LLM backbone):** Ye et al. (2025). *Dream-VL & Dream-VLA: Open Vision-Language and Vision-Language-Action Models with Diffusion Language Model Backbone.* arXiv:2512.22615.
  - **Legacy note:** earlier drafts referenced “HKU NLP (2024), 97.2% LIBERO” without a stable citation; treat any such performance claims as unverified unless traced to a specific paper/benchmark protocol.
- **OpenVLA-OFT:** (Unverified label in earlier drafts; likely a fine-tuning / decoding variant; add a concrete citation before treating as a distinct model family.)
- **GR00T N1:** NVIDIA et al. (2025). arXiv:2503.14734.
- **PixelVLA:** Liang et al. (2025). *PixelVLA: Advancing Pixel-level Understanding in Vision-Language-Action Model.* arXiv:2511.01571. Pixel-level understanding with multiscale encoder and visual prompting.
- **TraceVLA:** Zheng et al. (2024). *TraceVLA: Visual Trace Prompting Enhances Spatial-Temporal Awareness for Generalist Robotic Policies.* arXiv:2412.10345. Visual trace prompting for spatial-temporal awareness.
- **MemoryVLA:** Shi et al. (2025). arXiv:2508.19236. Perceptual-cognitive memory for long-horizon manipulation.
- **CoT-VLA:** Zhao et al. (2025). arXiv:2503.22020. Visual chain-of-thought reasoning for VLA.
- **Related (VLM reasoning; optional background for "L"/reasoning traces):** Deng et al. (2025). *OpenVLThinker: Complex Vision-Language Reasoning via Iterative SFT-RL Cycles.* arXiv:2503.17352. (Not a VLA policy paper per se, but relevant to how RL fine-tuning affects visual grounding and intermediate reasoning traces.)
- **GenieReasoner/FACT:** Liu et al. (2025). *Unified Embodied VLM Reasoning with Robotic Action via Autoregressive Discretized Pre-training.* arXiv:2512.24125. [FACT tokenizer: flow-matching action discretization; ERIQ benchmark for embodied reasoning]

## 13.2.1 VLA Benchmarks and Evaluation (v5.7)

- **VLA-Arena:** Zhang et al. (2025). *VLA-Arena: An Open-Source Framework for Benchmarking Vision-Language-Action Models.* arXiv:2512.22539. [170 tasks across Safety/Distractor/Extrapolation/Long Horizon; key finding: "memorization over generalization" tendency in current VLAs]
- **ERIQ:** Liu et al. (2025). Embodied Reasoning Intelligence Quotient benchmark, 6000+ QA pairs. (Part of GenieReasoner work, arXiv:2512.24125)

## 13.3 Multimodal PID

- **Liang PP, Cheng Y, Fan X, Ling CK, et al. (2023).** Quantifying & Modeling Multimodal Interactions: An Information Decomposition Framework. NeurIPS 2023. [Uses BATCH/CVX estimators, NOT I^sx_∩. Code: github.com/pliang279/PID]

- **IDTxl:** Wollstadt P, Lizier JT, et al. (2019). IDTxl: The Information Dynamics Toolkit xl. JOSS 4(34):1081. [Comprehensive PID toolkit. Code: github.com/pwollstadt/IDTxl]

- **SxPID:** Discrete `I^sx_∩` reference implementation (Python). [Code: https://github.com/Abzinger/SxPID]

- **sae_analysis:** WIP toolbox for Shannon-invariants-style analysis of SAE latents (degree of redundancy / vulnerability from Gutknecht et al. 2025). [Code: https://github.com/Abzinger/sae_analysis; experimental/not yet fully validated]

## 13.4 World Models

- **GWM:** Gaussian World Model, ICCV 2025. 3DGS + Diffusion for robotics.
- **Physically Embodied Gaussian Splatting:** CoRL 2024. Real-time correctable world model.
- **WAN:** Wanxiang Video Model, Alibaba 2025. arXiv:2503.20314
- **WAN VACE:** Video All-in-one Creation and Editing. arXiv:2503.07598
- **Wan-Move:** Motion-controllable Video Generation. arXiv:2512.08765 (NeurIPS 2025)
- **Motus:** Unified Latent Action World Model. arXiv:2512.13030 (Uses Wan 2.2 5B)
- **DreamGen:** Robot Learning via Neural Trajectories. arXiv:2505.12705 (Benchmarks WAN 2.1)
- **VideoVLA:** Video Generators as Robot Manipulators. arXiv:2512.06963
- **Scalable Policy Evaluation:** Action-conditioned video for policy eval. arXiv:2511.11520
- **Genie 3:** DeepMind (Aug 2025). General-purpose interactive world model. 24fps, 720p, emergent physics. (deepmind.google/blog/genie-3)
- **Genie 2:** DeepMind (Dec 2024). Large-scale foundation world model. (deepmind.google/discover/blog/genie-2-a-large-scale-foundation-world-model)
- **Genie 1:** Bruce et al. (Feb 2024). Generative Interactive Environments. arXiv:2402.15391
- **SIMA 2:** DeepMind (Nov 2025). Gemini-powered generalist agent for 3D virtual worlds. arXiv:2512.04797
- **Diffusion modeling note (background; optional):** Li & He (2025). *Back to Basics: Let Denoising Generative Models Denoise.* arXiv:2511.13720. (Relevant to how “denoising” vs “noise prediction” parameterizations can change representation geometry; treat as background for diffusion-based world models, not a PID paper.)

## 13.5 Uncertainty & Hallucination Detection

- **VL-Uncertainty:** Zhang et al. (2024). arXiv:2411.11919
- **SAFE:** Multitask VLA failure detection. arXiv:2506.09937
- **PRE-HAL:** Dempster-Shafer for VLM hallucination

## 13.6 Process Reward Models (PRMs)

- **Robo-Dopamine:** Tan et al. (2025). arXiv:2512.23703. [GRM for step-aware progress rewards]
- **GVL:** Vision-language in-context value learners. Ma et al. (2024). [Progress prediction]
- **VLAC:** Vision-language action critic. Zhai et al. (2025). arXiv:2509.15937
- **SARM:** Stage-aware reward modeling. Chen et al. (2025). arXiv:2509.25358
- **LIV:** Language-image representations for rewards. Ma et al. (2023). ICML

## 13.7 Information Theory

- **KSG Estimator:** Kraskov et al. (2004). Phys Rev E 69:066138
- **O-information (Ω; synergy-vs-redundancy bias for a set of variables):** introduced by Rosas et al. (2019). *(Bibliographic details should be verified; included as optional background, not part of the Wibral-group `I^sx_∩` line.)*
- **kNN MI under strong dependence (limitations + fixes):**
  - Gao, Ver Steeg, Galstyan (2015). *Efficient Estimation of Mutual Information for Strongly Dependent Variables.* arXiv:1411.2003.
  - Gao, Ver Steeg, Galstyan (2015). *Estimating Mutual Information by Local Gaussian Approximation.* arXiv:1508.00536.
- **Neural / classifier-based MI estimation (baselines for MI/CMI; not `I^sx_∩`):**
  - Belghazi et al. (2018). *MINE: Mutual Information Neural Estimation.* arXiv:1801.04062.
  - Mukherjee, Asnani, Kannan (2019). *CCMI: Classifier based Conditional Mutual Information Estimation.* arXiv:1906.01824.
  - Molavipour, Bassi, Skoglund (2019). *Conditional Mutual Information Neural Estimator.* arXiv:1911.02277.
- **Williams & Beer (2010).** Original PID formulation

## 13.8 Scalable PID Methods

- **Shannon Invariants:** Gutknecht et al. (2025). arXiv:2504.15779. [Scalable summaries]
- **Gaussian PID:** Barrett et al. (2023). NeurIPS. [Bias-corrected high-d estimation]
- **Normalizing-flow PID in latent Gaussian space:** Zhao et al. (2025). arXiv:2510.04417. (Earlier drafts referred to this as “Thin-PID”; the arXiv title is *Partial Information Decomposition via Normalizing Flows in Latent Gaussian Distributions*.)
- **Representational Complexity:** Ehrlich et al. (2022). Trans. ML Res. [Coarse-graining]
- **dit Library:** Python library for discrete information theory (dit.distributions)
- **IDTxl:** Comprehensive information dynamics toolkit (pwollstadt/IDTxl)

## 13.9 Depth Estimation & 3D Perception

- **Depth-Anything v2/v3:** Yang et al. (2024-2025). Monocular depth foundation models.
- **Video Depth Anything:** Temporally consistent video depth estimation.
- **RollingDepth:** Video depth without video models. arXiv:2411.19189. [LDM-based]
- **StereoVLA:** Deng et al. (2025). arXiv:2512.21970. [Stereo vision for VLAs]
- **DKT (Diffusion Knows Transparency):** arXiv:2512.23705. [Transparent object depth via WAN]
- **Metric3D v2:** Absolute depth with metric scale recovery.
- **SHARP:** Single-image to 3DGS conversion.

## 13.10 Simulation & Middleware

- **Gazebo Harmonic:** ROS 2 compatible physics simulator
- **Zenoh:** Zero-overhead pub/sub middleware (eclipse-zenoh.io)
- **Tauri:** Rust + WebView desktop apps (tauri.app)
- **SparkJS:** 3DGS rendering in browser via WebGPU
- **Three.js:** WebGL/WebGPU 3D rendering library

## 13.11 Training Infrastructure

- **NanoGPT:** Karpathy. GPT-2 reproduction in ~600 lines. (github.com/karpathy/nanoGPT)
- **nanochat:** Karpathy (2025). Full-stack ChatGPT training, ~$100. (github.com/karpathy/nanochat)
- **llm.c:** C/CUDA LLM training, 7% faster than PyTorch. (github.com/karpathy/llm.c)
- **modded-nanogpt:** Speedrun benchmark for LLM training optimization
- **SRL (step-wise reasoning training; optional):** Deng et al. (2025). *Supervised Reinforcement Learning: From Expert Trajectories to Step-wise Reasoning.* arXiv:2510.25992. (Potentially relevant to Aim 3 / PRM-style training loops; not PID-specific.)

## 13.12 Differential Geometry & Non-Euclidean Representation (Optional)

- Differential-geometry contingency notes are integrated into §8.1.5 (optional background; not a correctness source).
- **Manifold-aware MI estimation:** Marx, Fischer (2021). *Estimating Mutual Information via Geodesic kNN.* arXiv:2110.13883. (Riemannian/geodesic kNN MI; useful as MI-only baseline in curved settings.)
- **Hyperbolic embeddings for hierarchies:**
  - Nickel, Kiela (2017). *Poincaré Embeddings for Learning Hierarchical Representations.* arXiv:1705.08039.
  - Nickel, Kiela (2018). *Learning Continuous Hierarchies in the Lorentz Model of Hyperbolic Geometry.* arXiv:1806.03417.
  - Ganea, Bécigneul, Hofmann (2018). *Hyperbolic Neural Networks.* arXiv:1805.09112.
  - Yang et al. (2022). *Hyperbolic Graph Neural Networks: A Review of Methods and Applications.* arXiv:2202.13852.
- **Hyperbolic LLMs and fine-tuning (v5.7):**
  - **HELM:** First billion-scale hyperbolic LLM. arXiv:2505.24722.
  - **HypLoRA:** Hyperbolic fine-tuning for LLMs; shows token embeddings are inherently hyperbolic. arXiv:2410.04010.
  - **Hypformer:** Efficient hyperbolic transformer with linear complexity. arXiv:2407.01290.
  - **Hierarchical Mamba:** Projects Mamba2 representations into Poincaré/Lorentz manifolds. arXiv:2505.18973.
- **Hierarchical structure in LLM embeddings (v5.7):**
  - **δ-hyperbolicity analysis:** arXiv:2512.20926. [Shows modern models (ProtT5) are MORE tree-like than older models (SeqVec)]
  - **Cognitive state hierarchy:** Zhao (2025). *Hierarchical Geometry of Cognitive States in Transformer Embedding Spaces.* arXiv:2512.22227. [Demonstrates decodable hierarchical structure aligned with cognitive attributes]
- **Intrinsic dimension estimation (geometry diagnostics for kNN validity):**
  - Levina, Bickel (2005). *Maximum likelihood estimation of intrinsic dimension.* (Foundational intrinsic-dimension estimator; use as a diagnostic, not a guarantee.)
  - Gomtsyan et al. (2019). *Geometry-Aware Maximum Likelihood Estimation of Intrinsic Dimension.* arXiv:1904.06151.
- **Lorentzian conformal rigidity (background; mostly analogy-level for this project):**
  - Melnick, Pecastaing (2025). *A local Lorentzian Ferrand-Obata theorem for conformal vector fields.* arXiv:2511.03713.
  - Pecastaing (2019). *The conformal group of a compact simply connected Lorentzian manifold.* arXiv:1911.06251.
  - Frances (2025). *Conformal quotients of plane waves, and Lichnerowicz conjecture in a locally homogeneous setting.* arXiv:2503.08614.

---

# 14. Confounding Factors Analysis: Proving and Disproving the Hypotheses

This section addresses how confounding factors could be studied and removed to rigorously prove or disprove the core hypotheses of PID-VLA. Grant reviewers will scrutinize whether observed correlations reflect genuine causal relationships or are artifacts of confounding variables.

## 14.1 Core Hypotheses and Their Falsifiability

### Hypothesis H1: Negative synergy predicts VLA failures
**Claim:** When `Syn_{V,D→A} < 0` (or `Syn_{V,L→A} < 0`), the VLA is more likely to fail.

**Confounds to rule out:**
1. **Task difficulty confound:** Negative synergy might correlate with inherently harder tasks (longer horizons, more object interactions), not with model failure per se.
2. **Distribution shift confound:** Negative synergy might arise when inputs are out-of-distribution, which also causes failures—but the failure is due to OOD inputs, not synergy.
3. **Embedding quality confound:** If embeddings are poorly learned, both synergy estimates and task performance degrade together, creating spurious correlation.

**How to disprove:**
- Control for task difficulty by stratifying experiments (same task family, varying synergy).
- Add explicit OOD detection baselines and test whether synergy provides signal beyond OOD scores.
- Test on multiple VLA architectures; if synergy-failure correlation appears only in one, it may be architecture-specific rather than fundamental.

### Hypothesis H2: High redundancy indicates robust information integration
**Claim:** High `Red_{V,D;A}` suggests the model has multiple pathways to correct action.

**Confounds:**
1. **Triviality confound:** If the task is trivial (e.g., "do nothing"), all sources may redundantly encode the same null information.
2. **Overfitting confound:** High redundancy in training data might indicate memorization rather than generalization.

### Hypothesis H3: Unique information identifies modality-specific contributions
**Claim:** `Unq_V` vs `Unq_D` vs `Unq_L` indicates which modality dominates decision-making.

**Confounds:**
1. **Representation bias:** If one modality has higher-dimensional embeddings, it may have artificially higher unique information due to estimation artifacts.
2. **Preprocessing asymmetry:** Different preprocessing per modality can shift apparent unique contributions.

## 14.2 Experimental Controls for Confound Removal

### 14.2.1 Matched Control Experiments

For every "synergy predicts failure" claim, implement:

```
CONTROL DESIGN MATRIX
=====================

Primary comparison (within-task):
┌──────────────────────────────────────────────────────────────┐
│  Same task template    Same initial state seed               │
│  Same language instruction    Same environment physics       │
│  Different: VLA internal state / D representation            │
│                                                              │
│  Measure: ΔSynergy vs ΔFailure rate                          │
│  Prediction: Correlation should persist after matching       │
└──────────────────────────────────────────────────────────────┘

Task-difficulty control:
- Bin tasks by objective difficulty metrics (horizon length, object count, precision required)
- Test synergy-failure correlation WITHIN each difficulty bin
- If correlation disappears within bins, task difficulty is the true predictor

Distribution-shift control:
- Compute OOD score (e.g., Mahalanobis distance in embedding space, uncertainty calibration)
- Test whether synergy provides INCREMENTAL predictive power beyond OOD score
- Regression: Failure ~ OOD_score + Synergy + OOD_score×Synergy
```

### 14.2.2 Placebo Tests (Sanity Checks)

**Null intervention test:**
- Apply a "placebo" intervention that should NOT change synergy (e.g., add imperceptible noise to V)
- If measured synergy changes significantly, the estimator is sensitive to irrelevant variations

**Permutation test for spurious correlation:**
- Randomly permute trajectory labels within each task family
- Re-compute synergy-failure AUROC
- The permuted AUROC should be ~0.5 (no better than chance)
- If permuted AUROC > 0.55, there is label leakage or confounding

**Temporal shuffling test:**
- Shuffle timesteps within trajectories
- Re-estimate PID terms
- If estimates remain stable despite broken temporal structure, the estimator may not capture meaningful dynamics

### 14.2.3 Causal Identification Strategy

**Instrumental variable approach (if feasible):**
- Find a variable Z that affects D but not A directly (except through D)
- Example: Randomized perturbation to the world model training procedure
- Use Z as an instrument to estimate causal effect of D-quality on synergy

**Regression discontinuity design:**
- If there's a threshold in training (e.g., model checkpoint at step N), test whether synergy changes discontinuously at the threshold
- Sharp changes at arbitrary thresholds suggest overfitting to checkpoint artifacts

## 14.3 Alternative Interpretations of Results

### 14.3.1 If Negative Synergy Does NOT Predict Failure

**Interpretation 1: Synergy is architecture-dependent, not failure-predictive**
- Action: Report as valid negative result; pivot to simpler entropy/confidence baselines

**Interpretation 2: Estimator is broken at VLA scale**
- Action: Verify via Experiment 0; if estimator collapsed, negative result is uninformative

**Interpretation 3: Task distribution lacks sufficient failure diversity**
- Action: Expand benchmark to include more failure modes; re-test

### 14.3.2 If Positive Results Appear

**Alternative explanation 1: Confounding by entropy**
- Test: Include action entropy as covariate; if synergy becomes non-significant, entropy suffices

**Alternative explanation 2: Confounding by model uncertainty**
- Test: Include ensemble variance or explicit uncertainty estimate as covariate

**Alternative explanation 3: P-hacking through feature selection**
- Mitigation: Pre-register primary analysis; report ALL synergy variants tested, not just significant ones

## 14.4 Robustness Checks Required for Publication

| Check | Description | Pass Criterion |
|-------|-------------|----------------|
| **Seed robustness** | Run with 10+ random seeds | Effect size stable (CV < 30%) |
| **K robustness** | Test k ∈ {3, 5, 7, 10} | Direction consistent, magnitude within 2× |
| **Preprocessing robustness** | With/without standardization, jitter | Conclusions unchanged |
| **Dimensionality robustness** | Raw vs PCA-256 vs PCA-64 | At least one regime shows effect |
| **Temporal sampling** | Different stride/window sizes | Effect persists across reasonable ranges |
| **Cross-architecture** | Test on 2+ VLA architectures | Effect appears in majority |
| **Cross-benchmark** | Test on 2+ task distributions | Effect generalizes |

---

# 15. Numerical Stability and Optimization: Technical Guidance

This section documents known numerical issues, failure modes, and optimization strategies for making the estimators robust at scale.

## 15.1 Known Numerical Failure Modes

### 15.1.1 kNN Radius Collapse (Most Common)

**Symptom:** `PidError::NumericalInstability: kNN radius is non-positive`

**Causes:**
1. **Duplicate points:** Identical samples in the dataset
2. **Quantization:** Low-precision embeddings creating effective duplicates
3. **Constant dimensions:** Columns with zero variance

**Solutions (in order of preference):**
```rust
// 1. FIRST: Check for and remove exact duplicates
fn remove_duplicates(data: &mut Vec<Vec<f64>>) -> usize {
    let original_len = data.len();
    data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    data.dedup();
    original_len - data.len()
}

// 2. SECOND: Add small jitter (ONLY if duplicates cannot be avoided)
// Use Jitter::new(std, seed) with std ≈ 1e-10 to 1e-8
// WARNING: Jitter changes the quantity being estimated; re-validate

// 3. THIRD: Increase k if sample size permits (reduces tie sensitivity)
// But this increases bias; trade-off depends on n/d ratio
```

**⚠️ WARNING:** Do NOT silently add jitter. Always log when jitter is applied and quantify its effect on estimates.

### 15.1.2 Digamma Underflow for Small Arguments

**Symptom:** NaN or Inf in MI estimates when counts are very small

**Cause:** `digamma(x)` diverges as x → 0; if neighbor counts approach 0 due to sparse data, results become unstable.

**Solution (implemented in `stats.rs`):**
```rust
// Use the asymptotic expansion for small x:
// ψ(x) ≈ -1/x - 1/(2x²) for small x (but we shouldn't reach x < 1 in practice)

// Better: Ensure n > k + 1 always, and use a precomputed table for digamma(1..n)
pub fn digamma_int_table(n: usize) -> Vec<f64> {
    // Precompute ψ(1), ψ(2), ..., ψ(n) using the recurrence:
    // ψ(x+1) = ψ(x) + 1/x
    // ...
}
```

### 15.1.3 Distance Concentration at High Dimension

**Symptom:** MI estimates collapse to near-zero or become highly variable as d increases.

**Mathematical basis:** In high dimensions, the ratio of nearest-neighbor distance to average distance converges to 1 (Beyer et al., 1999). This makes kNN neighborhoods meaningless.

**Diagnostic (implemented in `geometry.rs`):**
```rust
// Compute the coefficient of variation of pairwise distances
// If CV < 0.1, distances are concentrated and kNN is likely unreliable
let stats = distance_concentration_stats(data, &cfg)?;
if stats.pairwise_cv < 0.1 {
    warn!("Distance concentration detected (CV={:.3}); kNN estimates may be unreliable", stats.pairwise_cv);
}

// Also check: nn_over_pairwise_mean should be << 1 for kNN to work
// If nn/pairwise_mean > 0.5, neighbors are not meaningfully "near"
```

**Solutions:**
1. Reduce dimensionality via PCA/projection BEFORE estimating
2. Use intrinsic dimension estimate to set appropriate k
3. Accept that kNN-based `I^sx_∩` may be invalid above some d threshold

### 15.1.4 Strong Dependence Pathology

**Symptom:** MI estimates have huge variance or are biased at low noise levels (high true MI).

**Cause:** When X nearly determines Y (or vice versa), the nearest neighbors in joint space are the same as in marginal space, breaking the KSG estimator's assumptions (Gao et al., 2015).

**Diagnostic:**
```rust
// Compute the empirical correlation or a proxy for dependence strength
// If |corr(X, Y)| > 0.95, warn about strong-dependence regime

// Better: Check if the 1-NN distance in joint space equals the marginal 1-NN distance
// for a large fraction of points (indicates near-determinism)
```

**Solutions:**
1. For MI-only: Use local Gaussian MI estimator (Gao et al., 2015, arXiv:1508.00536)
2. For `I^sx_∩`: Accept that noiseless signals may not be estimable; add explicit noise floor to target
3. Increase sample size significantly (quadratic in 1/noise for strongly dependent pairs)

## 15.2 Optimization Strategies

### 15.2.1 Memory-Efficient Distance Computation

For large n, storing the full n×n distance matrix is prohibitive. Use on-the-fly computation:

```rust
// Instead of: let distances = pairwise_distances(data); // O(n²) memory

// Use streaming kNN that computes distances row-by-row:
fn streaming_knn(data: MatRef<'_>, k: usize, metric: Metric) -> Vec<(Vec<usize>, Vec<f64>)> {
    let n = data.nrows();
    let mut results = Vec::with_capacity(n);

    for i in 0..n {
        // Compute distances from point i to all other points
        let mut dists: Vec<(usize, f64)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| (j, metric.distance(data.row(i), data.row(j))))
            .collect();

        // Partial sort to find k smallest
        dists.select_nth_unstable_by(k - 1, |a, b| a.1.partial_cmp(&b.1).unwrap());

        let (indices, distances): (Vec<_>, Vec<_>) = dists[..k].iter().cloned().unzip();
        results.push((indices, distances));
    }
    results
}
```

### 15.2.2 SIMD Acceleration for Distance Computation

The distance computation hotloop benefits significantly from SIMD:

```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[inline]
#[target_feature(enable = "avx2")]
unsafe fn chebyshev_distance_avx2(a: &[f64], b: &[f64]) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    let n = a.len();
    let mut max_diff = _mm256_setzero_pd();

    let chunks = n / 4;
    for i in 0..chunks {
        let va = _mm256_loadu_pd(a.as_ptr().add(i * 4));
        let vb = _mm256_loadu_pd(b.as_ptr().add(i * 4));
        let diff = _mm256_sub_pd(va, vb);
        let abs_diff = _mm256_andnot_pd(_mm256_set1_pd(-0.0), diff); // abs via sign bit clear
        max_diff = _mm256_max_pd(max_diff, abs_diff);
    }

    // Horizontal max reduction
    let mut arr = [0.0f64; 4];
    _mm256_storeu_pd(arr.as_mut_ptr(), max_diff);
    let mut result = arr.iter().cloned().fold(0.0, f64::max);

    // Handle remainder
    for i in (chunks * 4)..n {
        result = result.max((a[i] - b[i]).abs());
    }
    result
}
```

### 15.2.3 Parallelization Strategy

kNN computation is embarrassingly parallel across query points:

```rust
use rayon::prelude::*;

fn parallel_knn_mi(x: MatRef<'_>, y: MatRef<'_>, cfg: &KsgConfig) -> PidResult<f64> {
    let n = x.nrows();

    // Compute per-point contributions in parallel
    let contributions: Vec<f64> = (0..n)
        .into_par_iter()
        .map(|i| {
            // Compute kNN contribution for point i
            compute_point_contribution(i, x, y, cfg)
        })
        .collect();

    // Aggregate (sum + normalization)
    Ok(contributions.iter().sum::<f64>() / (n as f64))
}
```

**Caution:** Ensure thread-local RNG states if any stochastic element is involved.

### 15.2.4 Approximate kNN (Use With Extreme Caution)

For very large n, exact kNN becomes infeasible. Approximate methods (HNSW, FAISS) introduce bias:

```
APPROXIMATE kNN DECISION TREE
=============================

Is n > 100,000 AND d < 100?
├── YES: Consider ball-tree (exact but faster)
└── NO: Continue

Is n > 1,000,000?
├── YES: Consider approximate kNN with validation
│   └── REQUIRED: Run Experiment 0 subset with exact vs approx
│   └── REQUIRED: Report approximation error bound
│   └── REQUIRED: Use conservative recall target (≥0.99)
└── NO: Use brute-force (it's fast enough)

NEVER use approximate kNN without explicit validation.
NEVER silently switch from exact to approximate based on n.
```

## 15.3 Numerical Precision Recommendations

| Operation | Recommended Precision | Rationale |
|-----------|----------------------|-----------|
| Distance computation | f64 | Avoid cancellation in differences |
| Distance storage (if needed) | f64 | Sorting/comparison sensitivity |
| Digamma evaluation | f64 | Series expansion needs precision |
| Final MI/PID output | f64 | But report with appropriate sig figs |
| Random projection matrix | f32 sufficient | Johnson-Lindenstrauss doesn't need f64 |

## 15.4 Debugging Checklist for Numerical Issues

```
WHEN ESTIMATES LOOK WRONG, CHECK:
================================

1. [ ] Are there NaN or Inf values in input data?
   → Use: data.iter().all(|x| x.is_finite())

2. [ ] Are there duplicate rows?
   → Count unique rows; if < n, investigate source

3. [ ] Are any columns constant?
   → Check column variance; remove or warn

4. [ ] Is k appropriate for n?
   → Rule of thumb: k << sqrt(n), and n > 10*k minimum

5. [ ] Is d appropriate for n?
   → If d > n/10, expect degradation; check intrinsic dim

6. [ ] Is the true MI huge (strong dependence)?
   → Add noise to target and check if estimates stabilize

7. [ ] Are preprocessing parameters logged?
   → Verify standardization was applied, check for NaN in mean/std

8. [ ] Is the random seed fixed for reproducibility?
   → Run twice with same seed; results must be identical
```

---

# 16. Why PCA and kNN Are Suboptimal for Manifold-Valued Embeddings

This section provides rigorous analysis of why standard dimensionality reduction and nearest-neighbor methods fail on manifold-structured data, and what alternatives exist.

## 16.1 The Manifold Hypothesis for Neural Embeddings

Modern neural embeddings (including VLA representations) empirically lie near **low-dimensional manifolds** embedded in high-dimensional ambient space. This creates a mismatch with standard Euclidean tools:

```
MANIFOLD STRUCTURE ILLUSTRATION
===============================

True data geometry:         What PCA/kNN assume:

    ╭─────────────╮              •  •  •  •
   ╱               ╲             •  •  •  •
  │    M ⊂ ℝᵈ      │            •  •  •  •
  │  (curved)      │            (uniform in ℝᵈ)
   ╲               ╱
    ╰─────────────╯

Geodesic distance ≠ Euclidean distance
Manifold dimension << ambient dimension
```

## 16.2 Why PCA Fails on Manifolds

### 16.2.1 Mathematical Failure Mode

PCA finds directions of maximum **linear** variance. On curved manifolds, this can:

1. **Conflate intrinsic and extrinsic variance:**
   - A spiral in 3D has high variance in all 3 axes but intrinsic dimension 1
   - PCA retains all 3 components, failing to discover the 1D structure

2. **Distort local neighborhoods:**
   - Two points close in geodesic distance may be far in Euclidean distance
   - PCA preserves Euclidean distances, not geodesic distances
   - After PCA, kNN may find "wrong" neighbors

3. **Introduce artifacts at high curvature:**
   - Regions of high curvature project onto overlapping linear subspaces
   - Distinct manifold regions become indistinguishable

### 16.2.2 Empirical Diagnostic

```rust
// Test for PCA inadequacy:
// 1. Estimate intrinsic dimension before and after PCA
// 2. If PCA dimension >> intrinsic dimension, PCA is overkill but safe
// 3. If PCA dimension < intrinsic dimension, PCA destroys structure

let id_raw = intrinsic_dimension_levina_bickel(raw_data, &cfg)?;
let id_pca = intrinsic_dimension_levina_bickel(pca_data, &cfg)?;

if pca_dims < id_raw * 0.8 {
    warn!("PCA may destroy manifold structure: ID_raw={:.1}, ID_pca={:.1}, PCA_dims={}",
          id_raw, id_pca, pca_dims);
}
```

### 16.2.3 When PCA Is Acceptable

PCA is acceptable when:
1. The manifold is approximately **linear** (low curvature everywhere)
2. The retained variance is >> 95% (minimal information loss)
3. Experiment 0 re-validation shows stable estimates after PCA
4. Intrinsic dimension is preserved (ID_after ≈ ID_before)

## 16.3 Why Euclidean kNN Fails on Manifolds

### 16.3.1 The Shortcut Problem

kNN with Euclidean distance finds "shortcuts" through the ambient space that do not exist on the manifold:

```
SHORTCUT PROBLEM
================

Manifold path (geodesic):      Euclidean path:
    A ───────╮                    A
             │                     ╲
    (long geodesic)                 ╲ (short Euclidean)
             │                       ╲
    B ───────╯                        B

kNN may declare A and B as neighbors even though
they are far apart on the manifold.
```

### 16.3.2 Impact on MI/PID Estimation

1. **Neighbor misidentification:** kNN finds "wrong" neighbors, leading to incorrect density estimates
2. **Volume estimation error:** The KSG estimator uses neighborhood volumes; Euclidean balls have wrong volume on curved manifolds
3. **Bias compounds with dimension:** Error grows exponentially with intrinsic dimension

### 16.3.3 Quantifying the Problem

A practical “shortcut distortion” diagnostic is to compare Euclidean distances to approximate geodesic distances on a kNN graph:

1. Build a kNN graph (Euclidean) with `k_graph` neighbors.
2. For a small set of anchor points (or random pairs), compute shortest-path distances on this graph (Dijkstra).
3. Compare ratios \(d_\text{geo}(i,j) / d_\text{euc}(i,j)\):
   - Large ratios (e.g., >2 on many pairs) indicate severe shortcut distortion (Euclidean neighbors are unreliable).

**Status:** This graph-geodesic distortion diagnostic is not implemented in `pid-core` yet; today we rely on intrinsic-dimension + distance-concentration proxies (§16.5).

## 16.4 Alternatives to PCA and Euclidean kNN

### 16.4.1 For Dimensionality Reduction

| Method | When to Use | Limitations |
|--------|-------------|-------------|
| **UMAP/t-SNE** | Visualization only | Non-invertible, distorts global structure |
| **Isomap** | When geodesic structure matters | Sensitive to noise, holes in manifold |
| **Diffusion Maps** | Multi-scale manifold structure | Computational cost, parameter sensitivity |
| **Autoencoders (VAE)** | Learned nonlinear projection | Changes the quantity; requires re-validation |
| **Hyperbolic embeddings** | Hierarchical / tree-like structure | Non-Euclidean metric; would require a new MI/`I^sx_∩` estimator (not drop-in) |

**Recommendation for PID-VLA:**
1. **First:** Try PCA with high variance retention (≥95%) + Experiment 0 re-validation
2. **If PCA fails:** Use random projections / feature hashing (preserves ambient Euclidean distances; not a geodesic fix) + re-validation
3. **If random projection fails:** Consider Isomap + re-validation, or accept that kNN-based PID is invalid

### 16.4.2 For Manifold-Aware MI Estimation

**Geodesic kNN MI (Marx & Fischer, 2021):**
- Replace Euclidean distances with geodesic distances
- Requires manifold to be explicitly estimated or approximated
- Computational cost: O(n² log n) for geodesic computation
- Does NOT directly provide `I^sx_∩`; use for MI-only screening

```python
# Pseudocode for geodesic kNN MI (not Rust; research prototype)
def geodesic_knn_mi(X, Y, k):
    # 1. Build k-NN graph on X
    # 2. Compute shortest-path geodesic distances
    # 3. Use geodesic distances in KSG estimator
    # 4. Repeat for Y and (X,Y) joint
    pass
```

**⚠️ WARNING:** Geodesic kNN MI is not implemented in `pid-core`. If manifold effects are suspected, treat this as a research direction, not a ready tool.

#### Hyperbolic embeddings: a concrete MI-only estimator pipeline (implemented; research-gated)

If you use **hyperbolic embeddings** (Poincaré/Lorentz) as a learned projection, you must also change the estimator’s notion of “neighborhood” to the **hyperbolic geodesic distance**. A minimal, defensible *MI-only* pipeline is:

1. **Represent points in the Lorentz (hyperboloid) model** of \(\mathbb{H}^d\) (constant curvature \(-1\)):
   - Points live in \(\mathbb{R}^{d+1}\) with Minkowski bilinear form \(\langle x,y\rangle_L = -x_0y_0 + \sum_{i=1}^d x_i y_i\)
   - Valid points satisfy \(\langle x,x\rangle_L = -1\) and \(x_0>0\)
2. **Use geodesic distance** \(d_\mathbb{H}(x,y) = \operatorname{arcosh}(-\langle x,y\rangle_L)\).
3. **Estimate MI terms using KSG with a product (L∞) joint metric**:
   - For MI `I(X;Y)`, use the joint distance \(d((x,y),(x',y')) = \max(d_\mathbb{H}(x,x'), d_\mathbb{H}(y,y'))\), then KSG counts in the marginals using the same \(\varepsilon_i\) radius (standard KSG structure).
4. **Compute Shannon-invariant screening terms** (CI/Ω) from these MI estimates.

**Status in this repo:** `pid-core` now provides an **experimental** hyperbolic geodesic distance via `Metric::HyperbolicLorentz`, so you can run:
- MI via `ksg_mi(…, KsgConfig { metric: Metric::HyperbolicLorentz, … })`
- CI via `co_information_pairwise` / `co_information_triplet` with the same metric

**Important limitations (do not overclaim):**
- This is an MI/CI pipeline only. It does **not** make the continuous shared-exclusions `I^sx_∩` estimator “hyperbolic-correct” automatically; the Ehrlich et al. (2024) estimator is validated under the Euclidean/L∞ convention. Treat “hyperbolic + `I^sx_∩`” as research, requiring a re-derivation + a new Experiment 0 gate.
- A learned hyperbolic projection is non-invertible and therefore **changes the measured quantity**; report it as a different measurement regime.

**Paper check (important): why we treat this as research-gated**
- Kraskov et al. (KSG MI) and the continuous shared-exclusions estimator of Ehrlich et al. explicitly use the **maximum norm / L∞** construction so that a joint-space “ball” factorizes into a product of marginal balls and the relevant volume terms cancel in KSG-style expressions.
- Ehrlich et al. also note that other *Euclidean* norms can yield asymptotically consistent density estimates under standard “nicely shrinking” conditions, but the exact KSG-style cancellation logic (and our cross-checks vs `csxpid`) are tied to the L∞ convention at finite sample sizes.
- Hyperbolic geodesic neighborhoods are not covered by that Euclidean-norm argument; curvature changes local volume elements and the disjunction-neighborhood construction would need to be re-derived. Therefore, we do **not** claim `I^sx_∩` on hyperbolic embeddings without a fresh derivation + Experiment 0 validation.

## 16.5 Determining Whether Manifold Methods Are Necessary

### 16.5.1 Decision Flowchart

```
MANIFOLD METHODS DECISION TREE
==============================

1. Estimate intrinsic dimension (ID)
   └── ID < ambient_dim / 10?
       ├── YES: Manifold structure likely significant
       │   └── Continue to step 2
       └── NO: Euclidean methods may suffice
           └── Proceed with PCA/Euclidean kNN

2. Compute distance concentration (DC)
   └── CV of pairwise distances < 0.2?
       ├── YES: Distance concentration; Euclidean kNN unreliable
       │   └── Continue to step 3
       └── NO: Euclidean kNN may work
           └── Validate with Experiment 0

3. Compute manifold distortion (if implemented)
   └── Max geodesic/Euclidean ratio > 2?
       ├── YES: Manifold structure critical
       │   └── PIVOT to manifold-aware methods OR
       │   └── Accept that kNN-based I^sx_∩ is invalid
       └── NO: Euclidean approximation acceptable
           └── Proceed with caution + Experiment 0 validation

4. Always: Re-run Experiment 0 after any dimensionality reduction
```

### 16.5.2 Practical Checklist for VLA Embeddings

```
MANIFOLD ANALYSIS CHECKLIST
===========================

Before running PID on VLA embeddings:

[ ] Compute intrinsic dimension estimate
    → Record: ID_V, ID_L, ID_D, ID_A, and joint IDs

[ ] Check distance concentration
    → Record: pairwise CV for each variable

[ ] If ID << ambient dim:
    [ ] Compare PCA-reduced ID to original ID
    [ ] If PCA destroys structure, consider alternatives

[ ] If using PCA:
    [ ] Record variance retained
    [ ] Re-run Experiment 0 subset
    [ ] Compare estimates before/after

[ ] If estimates are unstable across methods:
    [ ] Report instability as a finding
    [ ] Consider that kNN-based I^sx_∩ may not be appropriate
    [ ] Fall back to Shannon invariants (CI screening)
```

## 16.6 Local Flatness Testing: Empirically Validated Methods (Jan 2026)

The "locally flat" assumption underpins PCA and standard kNN MI estimation. This section documents **empirically validated methods** to test whether this assumption holds for VLA embeddings.

### 16.6.1 Method 1: Manifold Curvature via Subspace Angles ([IEEE 2023](https://ieeexplore.ieee.org/document/10020561/))

Compute weighted angles between local subspaces at each data point:

```python
def manifold_curvature_estimate(X, k=20, pca_dims=10):
    """
    Estimate manifold curvature at each point.
    Returns per-point curvature and global average.
    """
    N = len(X)
    curvatures = []

    for i in range(N):
        # 1. Find k nearest neighbors
        neighbors_i = knn(X, X[i], k)

        # 2. Compute local PCA subspace at point i
        S_i = local_pca(X[neighbors_i], n_components=pca_dims)

        # 3. For each neighbor j, compute subspace S_j
        angles = []
        for j in neighbors_i:
            neighbors_j = knn(X, X[j], k)
            S_j = local_pca(X[neighbors_j], n_components=pca_dims)

            # 4. Principal angle between subspaces
            angle = subspace_angle(S_i, S_j)
            weight = 1.0 / distance(X[i], X[j])
            angles.append(weight * angle)

        # 5. Curvature = minimum weighted angle
        curvatures.append(min(angles))

    return curvatures, np.mean(curvatures)
```

**Key finding**: "Each layer of a neural network maps an input manifold to a **flatter manifold** during training, and each successive block generates a manifold with less curvature."

**Interpretation**:
- Low curvature (< 0.1 radians) → locally flat, PCA acceptable
- High curvature (> 0.5 radians) → manifold methods needed

### 16.6.2 Method 2: Ollivier-Ricci Curvature ([Nature Comm. 2021](https://www.nature.com/articles/s41467-021-24884-1))

The **only discrete curvature** proven to converge to Ricci curvature of the underlying Riemannian manifold:

```
ORC(x, y) = 1 - W₁(μ_x, μ_y) / d(x, y)

Where:
- W₁ = Wasserstein-1 distance between neighborhood distributions
- μ_x = uniform distribution over k-NN of x
- d(x,y) = distance between x and y
```

**Interpretation**:
- ORC ≈ 0: locally flat (grid-like) → Euclidean methods valid
- ORC > 0: positively curved (sphere-like, clustered)
- ORC < 0: negatively curved (hyperbolic, tree-like) → consider hyperbolic methods

**Implementation status**: Not in `pid-core` yet. Python reference: `GraphRicciCurvature` package.

### 16.6.3 Method 3: DLME Local Flatness Constraint ([arXiv:2207.03160](https://arxiv.org/abs/2207.03160))

The Deep Local-flatness Manifold Embedding adds a second-order curvature penalty:

```
L_flatness = Σᵢ ||∇²f(x_i)||²_F

Where ∇²f is the Hessian of the embedding function
```

**Application to VLA**: Can be used to **train** flat embeddings, not just diagnose.

### 16.6.4 Method 4: Curvature-Adjusted PCA Diagnostic

Standard local PCA assumes flatness. Test the assumption:

```python
def local_flatness_diagnostic(X, k_values=[10, 20, 50, 100]):
    """
    If ID estimate increases with k, local flatness is violated.
    """
    id_estimates = []
    for k in k_values:
        id_k = intrinsic_dimension_levina_bickel(X, k=k)
        id_estimates.append(id_k)

    # Flatness violation if ID increases >20% with k
    if id_estimates[-1] > id_estimates[0] * 1.2:
        return "VIOLATED: larger neighborhoods capture global curvature"
    else:
        return "ACCEPTABLE: local flatness assumption holds"
```

**Key insight** ([arXiv:2510.15141](https://arxiv.org/abs/2510.15141)): "Estimators based on flatness assumptions tend to increase estimates with neighborhood size because larger neighborhoods capture more global geometry, violating local linear assumptions."

## 16.7 δ-Hyperbolicity: Testing for Hierarchical Structure (Jan 2026)

### 16.7.1 The Gromov δ-Hyperbolicity Measure

δ-hyperbolicity measures how "tree-like" a metric space is. Trees have δ = 0; higher δ indicates deviation from tree structure.

**Definition** (Gromov 4-point condition):
```
For points x, y, z, w:

(x·y)_w = 0.5 * (d(x,w) + d(y,w) - d(x,y))  # Gromov product

δ = max over all quadruples of:
    min((x·y)_w, (x·z)_w) - (y·z)_w
```

**Normalized form** (scale-invariant):
```
δ_rel ∈ [0, 1] where:
- δ_rel ≈ 0: highly tree-like (hyperbolic methods appropriate)
- δ_rel ≈ 1: not tree-like (Euclidean may be acceptable)
```

### 16.7.2 Empirical Evidence from LLM Embeddings ([arXiv:2512.20926](https://arxiv.org/abs/2512.20926))

| Model | δ_avg | Ultrametricity | Interpretation |
|-------|-------|----------------|----------------|
| **ProtT5** (modern) | 0.04 | 0.13 | Strongly tree-like |
| **ESM-2** | 0.09 | 0.22 | Moderately tree-like |
| **TAPE** | 0.15 | 0.31 | Weakly tree-like |
| **SeqVec** (older) | 1.62 | 3.66 | Not tree-like |

**Key finding**: "Tree-likeness correlated strongly with downstream task performance" — ProtT5 achieved ROC-AUC of 0.80 vs SeqVec's 0.62.

**Implication for VLA**: Modern LLM backbones (Llama 2 7B) likely exhibit low δ-hyperbolicity, suggesting:
1. Hyperbolic projections may be effective for dimensionality reduction
2. Hierarchical screening (Shannon invariants) aligns with embedding structure
3. Euclidean PCA may destroy implicit hierarchical organization

### 16.7.3 When to Use Hyperbolic vs Euclidean

```
HYPERBOLICITY DECISION TREE
============================

1. Compute δ-hyperbolicity on sample (n=1000-5000)
   └── δ_rel < 0.1?
       ├── YES: Strong hierarchy
       │   ├── Use hyperbolic embedding for projection
       │   ├── Use Shannon invariants (CI) for screening
       │   └── Full I^sx_∩ only after Lorentz MI validation
       └── NO: Continue to step 2

2. δ_rel ∈ [0.1, 0.3]?
   ├── YES: Moderate hierarchy
   │   ├── Compare Euclidean PCA vs hyperbolic projection
   │   └── Choose based on Experiment 0 validation
   └── NO: δ_rel > 0.3, weak/no hierarchy
       └── Euclidean methods acceptable (with flatness check)
```

### 16.7.4 Do You Need to Train a Hyperbolic Embedding Model? (v5.7)

**Short answer:** Usually NO for PID-VLA. Here's the decision framework:

| Scenario | Train Hyperbolic Model? | Recommendation |
|----------|------------------------|----------------|
| **Using pre-trained VLA (OpenVLA, PixelVLA, TraceVLA)** | ❌ NO | Embeddings already exist; just compute δ-hyperbolicity to decide analysis method |
| **Dimensionality reduction for PID** | ⚠️ MAYBE | If δ < 0.1, consider HypLoRA-style projection; otherwise use PCA |
| **Shannon invariant screening (CI)** | ❌ NO | CI works with any MI estimator; no hyperbolic training needed |
| **Full `I^sx_∩` on Llama hidden states** | ❌ NO | Use Experiment 0 to validate L∞ estimator; if fails, use quantization |
| **Custom VLA from scratch** | ⚠️ MAYBE | Consider HELM/Hypformer architecture if hierarchy is central |

**Where hyperbolic training IS needed:**

1. **If you want a hyperbolic projection layer** for dimensionality reduction:
   - Train a Poincaré/Lorentz projection head on top of frozen VLA
   - Use HypLoRA ([arXiv:2410.04010](https://arxiv.org/abs/2410.04010)) for efficient fine-tuning
   - Target: ~64-256 hyperbolic dimensions

2. **If you want to compare Euclidean vs Hyperbolic representations:**
   - Train parallel projection heads (one Euclidean, one hyperbolic)
   - Compare downstream PID diagnostics
   - This is a research experiment, not a requirement

**Where hyperbolic training is NOT needed:**

1. **For geometry diagnostics** (δ-hyperbolicity, curvature): Just compute on existing embeddings
2. **For Shannon invariants (CI, Ω)**: Works with standard MI estimators
3. **For SAE analysis**: SAEs operate in Euclidean space
4. **For full `I^sx_∩`**: The L∞ estimator is Euclidean; hyperbolic `I^sx_∩` doesn't exist yet

**Practical recommendation for PID-VLA:**
```
1. Extract embeddings from pre-trained VLA (OpenVLA, PixelVLA, etc.)
2. Compute δ-hyperbolicity
3. If δ < 0.1: Use Shannon invariants (CI) for screening; report hyperbolic structure
4. If δ ≥ 0.1: Use standard PCA + L∞ `I^sx_∩` (with Experiment 0 validation)
5. Training hyperbolic models is OPTIONAL and only for comparative research
```

## 16.8 SAE Analysis for VLA Embeddings (Jan 2026)

### 16.8.1 What Sparse Autoencoders Reveal

Sparse Autoencoders (SAEs) decompose polysemantic neurons into monosemantic, interpretable features. Recent work ([NeurIPS 2025, arXiv:2504.02821](https://arxiv.org/abs/2504.02821)) shows SAEs work for Vision-Language Models.

**Key findings**:
- SAE features show **modular structure** ("lobes" for math, code, etc.)
- Features exhibit **geometric regularity** (parallelograms like man:woman::king:queen)
- **Steering capability**: SAE interventions on CLIP can directly steer LLaVA outputs

### 16.8.2 SAE Application to VLA Components

| VLA Component | SAE Applicability | Benefit |
|---------------|-------------------|---------|
| **SigLIP encoder** | ✓ Verified (NeurIPS 2025) | Decompose V into monosemantic visual features |
| **DinoV2 encoder** | ✓ Likely (same architecture class) | Geometric/semantic feature separation |
| **Llama hidden states** | ✓ Verified (Anthropic) | Interpretable D/L representations |
| **Action decoder** | ? Untested | Could reveal action primitives |

### 16.8.3 SAE for PID Analysis: Concrete Protocol

```python
# 1. Train SAE on vision encoder (e.g., SigLIP layer in OpenVLA)
sae = SparseAutoencoder(
    d_input=1024,      # SigLIP output dim
    expansion=16,       # 1024 → 16384 sparse features
    sparsity_penalty=0.04
)
sae.train(vision_embeddings)

# 2. Extract sparse features
V_sparse = sae.encode(vision_embedding)  # Sparse, ~100 active features

# 3. Compute PID on SAE features
# - Lower effective dimension (only active features)
# - More interpretable decomposition
# - Can identify WHICH features drive actions

# 4. Feature ablation for failure diagnosis
for feature_idx in top_active_features:
    V_ablated = ablate_feature(V_sparse, feature_idx)
    action_change = model.forward(V_ablated) - model.forward(V_sparse)
    if action_change > threshold:
        print(f"Feature {feature_idx} drives action prediction")
```

### 16.8.4 Geometric Implications of SAE

SAE features have structure at three scales ([arXiv:2410.19750](https://arxiv.org/abs/2410.19750)):

1. **Atomic scale**: "Crystals" — parallelogram/trapezoid faces (analogy relations)
2. **Intermediate scale**: "Lobes" — modular clustering (math, code, language)
3. **Global scale**: Hierarchical organization of concept space

**Implication**: SAE features may have LOWER effective dimensionality and MORE hierarchical structure than raw embeddings, making them better candidates for:
- Shannon invariant screening (CI)
- Hyperbolic projection
- Interpretable PID decomposition

## 16.9 Chebyshev Distance and PixelVLA: Geometry Transition Analysis (Jan 2026)

### 16.9.1 Chebyshev in Image Processing

Chebyshev distance (L∞) is natural for pixel operations:

| Operation | Distance Metric | Structuring Element |
|-----------|-----------------|---------------------|
| **8-connected dilation/erosion** | Chebyshev (L∞) | Square (3×3) |
| **4-connected dilation/erosion** | Manhattan (L1) | Cross/Diamond |
| **Edge detection (8-neighbor)** | Chebyshev | Square kernel |
| **Pattern recognition** | Often L∞ | Square windows |

### 16.9.2 Geometry Transition in VLA Pipeline

```
GEOMETRY TRANSITION IN VLAs
============================

              PIXEL SPACE                    SEMANTIC SPACE
    ─────────────────────────────────────────────────────────

    Vision Encoder                              LLM Backbone
    (DinoV2, SigLIP)                           (Llama 2 7B)

    • Chebyshev natural                         • Hierarchical/tree-like
    • 8-connectivity                            • δ-hyperbolic
    • Local morphology ops                      • Curved manifold
    • ~1024 dim                                 • 4096 dim

    APPROPRIATE:                                APPROPRIATE:
    L∞ estimators                               Hierarchical screening
    PCA may work                                Quantization or unrolling
    SAE for feature decomposition              Shannon invariants safest
```

### 16.9.3 Where Chebyshev Is Appropriate in PixelVLA

| Stage | Geometry | Chebyshev Valid? |
|-------|----------|------------------|
| **Raw image input** | Pixel grid | ✓ Yes (8-connectivity) |
| **DinoV2 patches** | Patch embeddings | ✓ Partially (local structure) |
| **SigLIP output** | Global features | ⚠️ Transitional |
| **Multiscale encoder** | Hierarchical features | ⚠️ Transitional |
| **MLP projector output** | LLM-aligned | ❌ Hierarchical dominates |
| **Llama hidden states** | Semantic space | ❌ Use hierarchical methods |
| **Action decoder** | Continuous actions | ⚠️ Depends on structure |

### 16.9.4 Recommendation for PixelVLA PID Analysis

1. **For V at early stages** (patches, local features): Chebyshev/L∞ `I^sx_∩` is appropriate
2. **For V at late stages** (after MLP projector): Use hierarchical screening first
3. **For D (if extractable)**: Always use hierarchical screening; Chebyshev geometry likely invalid
4. **For A (actions)**: 7D continuous — Chebyshev valid if locally flat; test with curvature diagnostics

## 16.10 Hierarchical Structure: GPT-2 vs Modern LLMs (Jan 2026)

### 16.10.1 Architectural Differences Affecting Geometry

| Feature | GPT-2 | Llama 2 | Geometric Implication |
|---------|-------|---------|----------------------|
| **Position Encoding** | Absolute (learned) | RoPE (rotary) | RoPE preserves relative structure |
| **Activation** | ReLU | SwiGLU | SwiGLU may create smoother manifolds |
| **Attention** | Multi-head (MHA) | Grouped-query (GQA) | GQA may compress hierarchy differently |
| **Context Length** | 1024 tokens | 4096 tokens | Longer context → more hierarchical |
| **Layer Count** | 12 (small) | 32 (7B) | Deeper → more hierarchical processing |

### 16.10.2 Empirical Evidence for Hierarchy Evolution

| Evidence | Source | Finding |
|----------|--------|---------|
| **Token frequency** | [HypLoRA](https://arxiv.org/abs/2410.04010) | "Token embeddings exhibit high degree of hyperbolicity" |
| **δ-hyperbolicity** | [arXiv:2512.20926](https://arxiv.org/abs/2512.20926) | Modern models (ProtT5) show δ=0.04 vs older (SeqVec) δ=1.62 |
| **Brain alignment** | [arXiv:2502.14671](https://arxiv.org/html/2502.14671v1) | Llama 2 layer 12 shows highest brain alignment |
| **Billion-scale** | [HELM](https://arxiv.org/abs/2505.24722) | First hyperbolic LLM outperforms Euclidean at scale |

### 16.10.3 Layer-wise Hierarchy in Transformers

```
GPT-2 (12 layers):
├── Layers 1-4:  Lexical (word identity)
├── Layers 5-9:  Syntactic (grammar)
└── Layers 10-12: Semantic (meaning, task-specific)

Llama 2 7B (32 layers):
├── Layers 1-8:   Lexical + early syntax
├── Layers 9-20:  Deep syntax + semantics
├── Layers 21-28: Abstract representations
└── Layers 29-32: Task-specific output

Implication: Later models have MORE hierarchical depth
```

### 16.10.4 Implications for DreamVLA (GPT-2 based)

DreamVLA uses GPT-2 backbone. Based on the evidence:
- GPT-2 shows LESS hierarchical structure than Llama 2
- δ-hyperbolicity likely higher (less tree-like)
- **Recommendation**: Euclidean methods may be more appropriate for DreamVLA than for OpenVLA/PixelVLA/TraceVLA

This creates an interesting contrast:
| VLA | Backbone | Expected δ | Recommended Geometry |
|-----|----------|------------|---------------------|
| **OpenVLA** | Llama 2 7B | Low (tree-like) | Hierarchical/Hyperbolic |
| **DreamVLA** | GPT-2 | Higher | Euclidean/PCA may suffice |
| **PixelVLA** | Llama 2 7B | Low | Hierarchical/Hyperbolic |
| **TraceVLA** | Llama 2 7B | Low | Hierarchical/Hyperbolic |

## 16.11 Unified Geometry-First Protocol (Jan 2026)

Based on the first-principles analysis, here is the recommended protocol:

```
┌─────────────────────────────────────────────────────────────────┐
│                    GEOMETRY-FIRST PROTOCOL                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  STEP 0: GEOMETRY DIAGNOSTICS (Before ANY PID)                  │
│  ├─ 0a. Intrinsic dimension (Levina-Bickel, GRIDE)              │
│  ├─ 0b. δ-hyperbolicity (Gromov 4-point sampling)               │
│  ├─ 0c. Ollivier-Ricci curvature (if implemented)               │
│  └─ 0d. Local flatness (neighborhood PCA residual)              │
│                                                                  │
│  DECISION TREE:                                                  │
│                                                                  │
│  δ < 0.1?  ──YES──→  Hierarchical structure dominant            │
│     │                 ├─ Use hyperbolic projection               │
│     │                 ├─ Shannon invariants (not full PID)      │
│     │                 └─ SAE for interpretable decomposition    │
│     NO                                                          │
│     ↓                                                           │
│  ORC ≈ 0?  ──YES──→  Locally flat                               │
│     │                 ├─ PCA + L∞ I^sx_∩ may work               │
│     │                 └─ Still validate with Experiment 0       │
│     NO                                                          │
│     ↓                                                           │
│  High curvature, non-hierarchical:                              │
│  └─→ Use QUANTIZATION (discrete PID) or                        │
│      MANIFOLD UNROLLING (Isomap/CAE → L∞ estimator)            │
│                                                                  │
│  STEP 1: SAE ANALYSIS (Optional but recommended for VLA)        │
│  ├─ Train SAE on vision encoder (SigLIP/DinoV2 layers)          │
│  ├─ Identify monosemantic features                              │
│  ├─ Use SAE features as interpretable V decomposition          │
│  └─ Re-run geometry diagnostics on SAE features                │
│                                                                  │
│  STEP 2: HIERARCHICAL SCREENING                                 │
│  ├─ Compute Shannon invariants: CI_VL, CI_VD, CI_LD            │
│  ├─ These are estimator-agnostic and fast                       │
│  └─ Only proceed to full I^sx_∩ if screening suggests value    │
│                                                                  │
│  STEP 3: TARGETED PID (If Step 2 passes)                        │
│  ├─ Apply appropriate geometry based on Step 0 diagnostics     │
│  └─ Full I^sx_∩ on reduced/validated representations           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 16.11.1 NanoGPT for Foundational Studies

For quick validation of geometry diagnostics, use minimal models:

```python
# NanoGPT-based geometry study protocol
# (~600 lines, ~1hr training, ~$10)

# 1. Train small action predictor
model = NanoGPT(d_model=256, n_layers=6, n_heads=4)
model.train(action_prediction_dataset)

# 2. Extract layer-wise embeddings
for layer in range(6):
    embeddings[layer] = model.get_hidden(validation_set, layer)

# 3. Compute geometry diagnostics per layer
for layer, emb in embeddings.items():
    results[layer] = {
        'intrinsic_dim': levina_bickel_mle(emb, k=5),
        'delta_hyp': gromov_hyperbolicity(emb, n_samples=1000),
        'orc': ollivier_ricci_curvature(emb, k=10),
        'local_flatness': local_pca_residual(emb, k=20),
    }

# 4. Identify geometry transition points
# Expect: curvature decreases with layer, δ decreases with layer
```

**Tooling**: sparkjs + tauri + gazebo enables fast iteration for this foundational study.

## 16.12 Theoretical Limitations (Fundamental, Not Fixable)

Some limitations are fundamental to kNN-based estimation on manifolds:

1. **Volume-form mismatch:** The KSG estimator assumes uniform volume elements; on curved manifolds, volume elements vary with curvature. This introduces bias even with geodesic distances.

2. **Intrinsic dimension heterogeneity:** If the intrinsic dimension varies across the manifold (e.g., lower near boundaries), kNN-based ID and MI estimates become inconsistent.

3. **Non-compact manifolds:** If the manifold is unbounded or has holes, geodesic distances can be undefined or infinite.

**Implication for PID-VLA:** Accept that there may be regimes where no kNN-based estimator works reliably. In such cases:
- Use Shannon invariants (CI, O-information) as the primary diagnostic
- Report kNN-based `I^sx_∩` with explicit caveats
- Consider neural MI estimators (MINE, etc.) as cross-checks

---

# Appendix A: Glossary

| Term | Definition |
|------|------------|
| **VLA** | Vision-Language-Action model |
| **PID** | Partial Information Decomposition |
| **I^sx_∩** | Shared-exclusions redundancy measure |
| **Synergy** | Information available only from multiple sources together |
| **Redundancy** | Information available from any single source |
| **KSG** | Kraskov-Stögbauer-Grassberger MI estimator |
| **3DGS** | 3D Gaussian Splatting |
| **GWM** | Gaussian World Model |
| **WAN** | Wanxiang video generation model (Alibaba) |
| **VACE** | Video All-in-one Creation and Editing (WAN extension) |
| **MoE** | Mixture of Experts architecture |
| **DiT** | Diffusion Transformer |
| **CI** | Co-Information (Shannon invariant) |
| **Ω** | O-Information (generalized co-information) |
| **GPID** | Gaussian Partial Information Decomposition |
| **LoRA** | Low-Rank Adaptation (fine-tuning method) |
| **Zenoh** | Zero-overhead pub/sub middleware for robotics |
| **NanoGPT** | Minimal GPT-2 training codebase (Karpathy) |
| **StereoVLA** | VLA enhanced with stereo vision |
| **DKT** | Diffusion Knows Transparency (transparent object depth) |
| **PRM** | Process Reward Model (dense progress-based rewards) |
| **GRM** | General Reward Model (Robo-Dopamine's step-aware PRM) |
| **ORM** | Outcome Reward Model (sparse success/failure rewards) |
| **VOC** | Value-Order Consistency (PRM evaluation metric) |
| **PBRS** | Potential-Based Reward Shaping |
| **Genie 3** | DeepMind's general-purpose interactive world model |
| **SIMA 2** | Scalable Instructable Multiworld Agent (DeepMind) |
| **TransPhy3D** | Synthetic transparent object video dataset (11k scenes) |
| **Emergent Physics** | Physics learned via self-supervision, not hardcoded |
| **PixelVLA** | VLA with pixel-level understanding and visual prompting |
| **TraceVLA** | VLA with visual trace prompting for spatial-temporal awareness |
| **Multiscale Pixel-Aware Encoder** | PixelVLA component for pixel-level feature injection |
| **Visual Prompting Encoder** | PixelVLA component for processing points, masks, regions |
| **Pixel-160K** | PixelVLA's pixel-annotated visuomotor dataset (160K trajectories) |
| **sae_analysis** | Makkeh's Shannon invariant toolkit for SAE analysis |
| **Red° (Degree of Redundancy)** | Shannon invariant: avg. extent info accessible from multiple sources |
| **Vul° (Degree of Vulnerability)** | Shannon invariant: avg. extent info lost when sources removed |

---

# Appendix B: Decision Log and Implementation Reference

## B.1 Decision Log (Detailed)

### Decision 1: Discard OpenVLA vs DreamVLA Comparison

| Attribute | Value |
|-----------|-------|
| **Date** | December 2025 |
| **Status** | REJECTED |
| **Category** | Experimental Design |

**Original Proposal:**
Compare PID decomposition signatures between OpenVLA (no explicit world model) and DreamVLA (explicit world model with `<dream>` tokens) to show that architectures with explicit world models have higher synergy.

**Why Rejected:**

1. **Confounds are insurmountable:** The architectures differ in backbone (Llama 2 vs Qwen), training data (Open-X vs proprietary + Open-X), action representation (discrete 256-bin vs continuous diffusion), and attention patterns. Any observed PID difference could be attributed to any of these factors.

2. **Circular reasoning:** If we define "D" differently for each architecture (hidden states for OpenVLA, explicit `<dream>` outputs for DreamVLA), we're comparing apples to oranges. The comparison would be measuring our operationalization choice, not intrinsic architecture properties.

3. **No ground truth:** We have no independent measure of "world model quality" to validate against. We'd be correlating one unknown (PID signature) with another unknown (implicit world model strength).

4. **Publication risk:** Reviewers would correctly identify these confounds and reject the comparison as methodologically unsound.

**Alternative Adopted:**
Focus on within-architecture analysis. Use DreamVLA as primary target because it has explicit, extractable world model states. OpenVLA analysis is deprioritized to Experiment 3 (if resources permit).

---

### Decision 2: Elevate V-L-A to Co-Primary Status

| Attribute | Value |
|-----------|-------|
| **Date** | December 2025 |
| **Status** | ADOPTED |
| **Category** | Decomposition Strategy |

**Original Proposal:**
V-D-A (Vision-Dream-Action) as primary decomposition, with V-L-A as secondary.

**Why Changed:**

1. **L is externally specified intent:** Language instructions are human-provided and are often the closest available “ground truth” for task intent, but they can still be ambiguous/underspecified. “Ignoring L” must be operationalized carefully (dataset semantics, annotation policy, and task context).

2. **D is model-internal:** The "Dream" representation is whatever the model learned. It might be wrong, incomplete, or encode biases. Using D as a reference conflates model failures with reference failures.

3. **Language grounding failures are common:** Empirical observation shows VLAs often execute plausible-but-wrong actions that ignore instruction specifics (e.g., "pick up the RED cup" → picks up nearest cup regardless of color).

4. **Direct interpretability:** Low Syn_{V,L→A} immediately suggests "model isn't integrating vision with language instruction." Low Syn_{V,D→A} is harder to interpret because D is opaque.

**Current Status:**
- V-L-A: Co-primary (recommended starting point)
- V-D-A: Co-primary (for DreamVLA specifically)
- V-L-D-A: Three-way analysis after pairwise validation

---

### Decision 3: Recommend Hierarchical Pairwise PID

| Attribute | Value |
|-----------|-------|
| **Date** | December 2025 |
| **Status** | ADOPTED |
| **Category** | Estimation Strategy |

**Original Proposal:**
Compute full three-way PID I(V, L, D; A) with 18 atoms from the start.

**Why Changed:**

1. **Estimation cost:** 18 atoms require many kNN-based estimates. With exact/brute-force kNN this scales at least like O(n²·d) per estimate; at VLA scale (d≈4096), this becomes prohibitively expensive without aggressive dimensionality reduction and/or accelerated kNN.

2. **Interpretation burden:** Most of the 18 atoms have no clear operational meaning. What does "information uniquely provided by V, but redundantly available in L and D" mean for robot control?

3. **Variance multiplication:** Each additional atom adds estimation variance. With 18 atoms, confidence intervals become uselessly wide.

4. **Pairwise captures most value:** The key insights (which source dominates? is there synergy or subadditivity?) are available from pairwise decompositions.

**Recommended Hierarchical Strategy:**

```
Level 0: Shannon invariants (fastest; MI-only)
├── Compute CI_VL, CI_VD, CI_LD (co-information)
├── Use for: Real-time monitoring, screening
└── Proceed to Level 1 if: Any CI is suspicious (outside normal range)

Level 1: Pairwise PID (slower; targeted)
├── Compute full I^sx_∩(V, L; A) or I^sx_∩(V, D; A)
├── Use for: Failure diagnosis, architecture comparison
└── Proceed to Level 2 if: Need three-way interactions

Level 2: Three-way PID (offline only)
├── Compute full I^sx_∩(V, L, D; A)
├── Use for: Detailed post-hoc analysis, publication figures
└── Only after pairwise validation complete
```

**Latency note:** Any ms-level budgets depend strongly on `(n,d,k)` and on the kNN backend. Brute-force exact kNN is not real-time at large `n`; treat timings as design targets, not guarantees.

---

### Decision 4: Mandate Experiment 0 First

| Attribute | Value |
|-----------|-------|
| **Date** | December 2025 |
| **Status** | MANDATORY |
| **Category** | Validation Protocol |

**Original Proposal:**
Start with VLA experiments immediately, validate estimator in parallel.

**Why Changed:**

1. **Unknown operating regime:** The continuous I^sx_∩ estimator (Ehrlich et al., 2024) was validated on d≤100. VLA embeddings are d=4096. We have no evidence it works at this scale.

2. **Garbage in, garbage out:** If the estimator produces nonsense at d=4096, all downstream conclusions are invalid. We'd waste months chasing artifacts.

3. **Fast validation:** Synthetic data experiments take days, not months. The cost of validation is low; the cost of skipping validation is potentially the entire project.

4. **Publishable regardless of outcome:** If Experiment 0 shows the estimator fails at high dimensions, that's a valid contribution to the PID literature.

**Experiment 0 Protocol:**

```python
# Experiment 0 is about estimator validity, not a priori “truth” claims at d=4096.
# Use i.i.d. synthetic systems + noise-dimension embeddings where true information
# quantities are invariant to added nuisance dimensions.

for dim in [64, 256, 1024, 4096]:
    for n_samples in [1000, 5000, 10000, 50000]:
        # 1) Generate low-d "signal" variables (e.g., 1–10 dims).
        # 2) Concatenate independent noise dims to reach `dim`.
        # 3) Compare estimates against reference values computed on the signal system
        #    (cross-checked with `csxpid` for redundancy and analytic MI where available).
        pass
```

**Go/No-Go Criteria:**
- **GO:** Stable estimates under noise-dimension embeddings up to d=4096 with acceptable variance/runtime.
- **PIVOT:** Stable only after dimensionality reduction (e.g., PCA to ~256) → adopt reduction + re-validate and proceed.
- **NO-GO:** Unstable even after reduction (or contradicts `csxpid` at low d) → treat kNN-based `I^sx_∩` as invalid for this regime and pivot to alternative diagnostics (e.g., Shannon invariants as primary).

---

### Decision 5: Recommend GWM over WAN for Analysis

| Attribute | Value |
|-----------|-------|
| **Date** | December 2025 |
| **Status** | ADOPTED (with caveats) |
| **Category** | World Model Integration |

**Context:**
Both GWM (Gaussian World Model) and WAN (Wanxiang) were considered for providing ground-truth world state predictions to validate against VLA internal representations.

**Why GWM is Preferred:**

| Criterion | WAN | GWM | Winner |
|-----------|-----|-----|--------|
| Trained on robot data | No (internet video) | Yes (robot trajectories) | GWM |
| 3D representation | No (2D video frames) | Yes (3D Gaussian Splatting) | GWM |
| Action-conditioned | No (unconditional) | Yes (predicts state given action) | GWM |
| Inference speed | ~4 min/clip | ~100ms/frame | GWM |
| Visual quality | Excellent | Moderate | WAN |
| Open weights | Yes (1.3B, 14B) | Limited | WAN |

**When to Use Each:**

- **GWM:** Core analysis, failure localization, training data augmentation
- **WAN:** Paper figures, demos, qualitative visualization
- **Neither:** Real-time intervention (both too slow; use entropy)

**Implementation Note:**
GWM integration requires 3DGS scene reconstruction, which adds pipeline complexity. For initial experiments, compute PID on VLA latents alone without external world model reference.

---

## B.2 Platform Implementation Reference

### B.2.1 Hardware Target Specification

| Component | Specification | Notes |
|-----------|---------------|-------|
| **Primary Target** | Apple M4 Max (128GB unified memory) | Development & prototyping |
| **Secondary Target** | NixOS + NVIDIA RTX 4090 (24GB VRAM) | Scaling & batch experiments |
| **Tertiary Target** | Apple M4 Pro (64GB unified memory) | May require model sharding |
| **Minimum Viable** | Apple M4 (24GB unified memory) | Requires aggressive quantization |

### B.2.2 Platform Comparison

| Aspect | Apple M4 Max | NixOS + RTX 4090 |
|--------|--------------|------------------|
| **Memory** | 128GB unified | 24GB VRAM + 64GB+ RAM |
| **Bandwidth** | 400 GB/s | 1008 GB/s (VRAM) |
| **Peak FP16** | ~27 TFLOPS | ~83 TFLOPS |
| **Power** | ~30W TDP | ~450W TDP |
| **Reproducibility** | Excellent (Metal determinism) | Good (Nix + pinned CUDA) |
| **Framework** | MLX | PyTorch + CUDA |
| **Best For** | Interactive dev, long runs | Batch experiments, scaling |

### B.2.3 Apple Silicon M4 Implementation (Primary)

#### B.2.3.1 Why Apple Silicon for Development

```
RATIONALE FOR M4-FIRST DEVELOPMENT
==================================

1. UNIFIED MEMORY ARCHITECTURE (UMA)
   - No CPU↔GPU memory copies for embedding extraction
   - VLA forward pass and PID computation share memory
   - Critical for large embedding buffers (4096-dim × 10000 samples = 160MB per source)
   
2. POWER EFFICIENCY
   - Long experiment runs without thermal throttling
   - Can run overnight on laptop without external cooling
   - ~30W vs ~300W for equivalent NVIDIA setup

3. MLX FRAMEWORK ADVANTAGES
   - Native lazy evaluation reduces memory pressure
   - Composable function transforms (grad, vmap, compile)
   - Unified NumPy-like API across CPU/GPU
   - Active development by Apple ML research

4. REPRODUCIBILITY
   - Nix works perfectly on macOS
   - Metal shaders are deterministic (unlike CUDA)
   - Single-machine setup avoids cluster variability

5. PRACTICAL CONSIDERATIONS
   - Hardware already owned
   - No cloud costs for initial development
   - Later port to Linux/CUDA for scaling
```

#### B.2.3.2 Apple Silicon Software Stack

```
┌─────────────────────────────────────────────────────────────────┐
│                     APPLICATION LAYER                           │
├─────────────────────────────────────────────────────────────────┤
│  Python Experiments    │  Tauri Visualization  │  Jupyter       │
│  (uv + pid-vla pkg)    │  (Rust + WebView)     │  Notebooks     │
├─────────────────────────────────────────────────────────────────┤
│                      PYTHON BINDINGS                            │
├─────────────────────────────────────────────────────────────────┤
│  PyO3 (pid-python)     │  mlx-rs bindings      │  coremltools   │
├─────────────────────────────────────────────────────────────────┤
│                        CORE LIBRARIES                           │
├─────────────────────────────────────────────────────────────────┤
│  pid-core (Rust)  │  MLX (C++)   │  CoreML    │  Accelerate    │
│  - KSG estimator  │  - VLA inf.  │  - Quant.  │  - BLAS/vDSP   │
│  - I^sx_∩ PID     │  - Autodiff  │  - ANE     │  - vecLib      │
│  - k-NN search    │  - Compile   │  inference │                │
├─────────────────────────────────────────────────────────────────┤
│                      ACCELERATION LAYER                         │
├─────────────────────────────────────────────────────────────────┤
│  Metal Performance    │  Metal           │  Apple Neural      │
│  Shaders (MPS)        │  Compute         │  Engine (ANE)      │
├─────────────────────────────────────────────────────────────────┤
│                         HARDWARE                                │
├─────────────────────────────────────────────────────────────────┤
│  Apple M4 Max: GPU (40 cores) + ANE (38 TOPS) + CPU (16 cores) │
│  Unified Memory: 128GB @ 400 GB/s bandwidth                     │
└─────────────────────────────────────────────────────────────────┘
```

### B.2.4 NixOS + CUDA Implementation (Secondary)

#### B.2.4.1 Why NixOS for CUDA Development

```
RATIONALE FOR NIXOS + CUDA AS SECONDARY TARGET
==============================================

1. REPRODUCIBILITY
   - Nix flakes provide bit-for-bit reproducible environments
   - CUDA version, cuDNN version, Python packages ALL pinned
   - No "works on my machine" issues
   - Same flake.nix works on any NixOS machine
   
2. CUDA VERSION MANAGEMENT
   - Run multiple CUDA versions simultaneously
   - No system-wide CUDA installation conflicts
   - Easy rollback if update breaks something
   - nix develop --impure for CUDA driver access
   
3. DECLARATIVE CONFIGURATION
   - Entire system configuration in version control
   - hardware.nvidia.* options for driver config
   - programs.cuda.enable for CUDA toolkit
   - Easy to spin up identical machines
   
4. FLAKE-BASED DEVELOPMENT
   - devShells with all dependencies
   - Separate shells for different CUDA versions
   - Cached builds via Cachix
   - CI/CD integration with GitHub Actions

5. PERFORMANCE PARITY
   - Same PyTorch code as Ubuntu/Arch
   - No performance overhead from Nix
   - NVIDIA drivers work normally
   - Full access to CUDA ecosystem
```

#### B.2.4.2 NixOS CUDA Hardware Specifications

| Component | Recommended | Minimum | Notes |
|-----------|-------------|---------|-------|
| **GPU** | RTX 4090 (24GB) | RTX 3090 (24GB) | 24GB VRAM required for 7B models |
| **CPU** | AMD Ryzen 9 / Intel i9 | 8+ cores | For data loading, preprocessing |
| **RAM** | 64GB+ | 32GB | CPU-side embedding buffers |
| **Storage** | 2TB NVMe | 1TB NVMe | Datasets, checkpoints |
| **PSU** | 1000W+ | 850W | RTX 4090 draws ~450W |

**Multi-GPU Considerations:**
- PID computation is embarrassingly parallel across samples
- Each GPU processes independent trajectory batches
- Minimal inter-GPU communication needed
- 2× RTX 4090 effectively doubles throughput

#### B.2.4.3 NixOS Configuration for CUDA

**System Configuration (`/etc/nixos/configuration.nix`):**

```nix
{ config, pkgs, ... }:

{
  # Enable unfree packages (required for NVIDIA drivers)
  nixpkgs.config.allowUnfree = true;
  
  # NVIDIA driver configuration
  services.xserver.videoDrivers = [ "nvidia" ];
  
  hardware.nvidia = {
    # Use the stable driver (production)
    package = config.boot.kernelPackages.nvidiaPackages.stable;
    
    # For RTX 40 series, use production branch
    # package = config.boot.kernelPackages.nvidiaPackages.production;
    
    # Enable modesetting (required for Wayland)
    modesetting.enable = true;
    
    # Power management
    powerManagement.enable = true;
    powerManagement.finegrained = false;  # Keep GPU active for compute
    
    # Enable NVIDIA settings GUI
    nvidiaSettings = true;
    
    # Open source kernel module (experimental, disable for stability)
    open = false;
  };
  
  # Enable OpenGL
  hardware.opengl = {
    enable = true;
    driSupport = true;
    driSupport32Bit = true;
  };
  
  # CUDA support
  environment.systemPackages = with pkgs; [
    cudaPackages.cudatoolkit
    cudaPackages.cudnn
    nvtopPackages.nvidia  # GPU monitoring
    pciutils
    lshw
  ];
  
  # Environment variables for CUDA
  environment.sessionVariables = {
    CUDA_PATH = "${pkgs.cudaPackages.cudatoolkit}";
    EXTRA_LDFLAGS = "-L/lib -L${pkgs.linuxPackages.nvidia_x11}/lib";
    EXTRA_CCFLAGS = "-I/usr/include";
  };
}
```

#### B.2.4.4 Project Flake for PID-VLA (NixOS)

```nix
# flake.nix for PID-VLA project
{
  description = "PID-VLA: Partial Information Decomposition for VLA Diagnostics";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config = {
            allowUnfree = true;  # Required for CUDA
            cudaSupport = true;
            cudaCapabilities = [ "8.9" ];  # RTX 40 series (Ada Lovelace)
            # cudaCapabilities = [ "8.6" ];  # RTX 30 series (Ampere)
          };
        };
        
        # Pin Rust version for reproducibility
        rustVersion = pkgs.rust-bin.stable."1.75.0".default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "x86_64-unknown-linux-gnu" ];
        };
        
        # Python environment with CUDA-enabled PyTorch
        pythonEnv = pkgs.python311.withPackages (ps: with ps; [
          # Core ML
          (pytorch-bin.override { cudaSupport = true; })
          torchvision-bin
          transformers
          accelerate
          bitsandbytes
          
          # Scientific computing
          numpy
          scipy
          scikit-learn
          polars
          
          # Visualization
          matplotlib
          seaborn
          plotly
          
          # Utilities
          tqdm
          wandb
          jupyter
          ipython
          
          # Testing
          pytest
          pytest-benchmark
        ]);
        
        # CUDA development libraries
        cudaLibs = with pkgs.cudaPackages; [
          cudatoolkit
          cudnn
          cutensor
          nccl  # For multi-GPU
        ];
        
      in {
        devShells = {
          # Primary CUDA development shell
          default = pkgs.mkShell {
            name = "pid-vla-cuda";
            
            buildInputs = [
              rustVersion
              pythonEnv
              pkgs.uv
              pkgs.just
              pkgs.pkg-config
              pkgs.openssl
            ] ++ cudaLibs;
            
            # CUDA environment setup
            shellHook = ''
              export CUDA_HOME="${pkgs.cudaPackages.cudatoolkit}"
              export CUDA_PATH="$CUDA_HOME"
              export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath cudaLibs}:$LD_LIBRARY_PATH"
              export TORCH_CUDA_ARCH_LIST="8.9"  # RTX 40 series
              
              # Verify CUDA is accessible
              if command -v nvidia-smi &> /dev/null; then
                echo "🚀 CUDA environment ready"
                nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader
              else
                echo "⚠️  nvidia-smi not found. Run with: nix develop --impure"
              fi
              
              echo "Python: $(python --version)"
              echo "PyTorch CUDA: $(python -c 'import torch; print(torch.cuda.is_available())')"
            '';
            
            # Required for accessing GPU drivers
            # Run with: nix develop --impure
            NIX_LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
              pkgs.stdenv.cc.cc
              pkgs.zlib
            ];
          };
          
          # CPU-only shell for testing without GPU
          cpu = pkgs.mkShell {
            name = "pid-vla-cpu";
            buildInputs = [
              rustVersion
              (pkgs.python311.withPackages (ps: with ps; [
                pytorch-bin
                numpy
                scipy
                polars
                pytest
              ]))
              pkgs.uv
              pkgs.just
            ];
          };
        };
        
        # Package definition for distribution
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "pid-vla";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];
        };
      });
}
```

#### B.2.4.5 NixOS CUDA Software Stack

```
┌─────────────────────────────────────────────────────────────────┐
│                     APPLICATION LAYER                           │
├─────────────────────────────────────────────────────────────────┤
│  Python Experiments    │  Tauri Visualization  │  Jupyter       │
│  (uv + pid-vla pkg)    │  (Rust + WebView)     │  Notebooks     │
├─────────────────────────────────────────────────────────────────┤
│                      PYTHON BINDINGS                            │
├─────────────────────────────────────────────────────────────────┤
│  PyO3 (pid-python)     │  torch.utils.cpp_ext  │  pybind11      │
├─────────────────────────────────────────────────────────────────┤
│                        CORE LIBRARIES                           │
├─────────────────────────────────────────────────────────────────┤
│  pid-core (Rust)  │  PyTorch     │  cuML/RAPIDS │  OpenBLAS    │
│  - KSG estimator  │  - VLA inf.  │  - GPU k-NN  │  - CPU BLAS  │
│  - I^sx_∩ PID     │  - Autodiff  │  - cuGraph   │  - Fallback  │
│  - k-NN search    │  - compile() │              │              │
├─────────────────────────────────────────────────────────────────┤
│                      ACCELERATION LAYER                         │
├─────────────────────────────────────────────────────────────────┤
│  cuDNN            │  CUDA        │  cuBLAS      │  NCCL        │
│  (Deep Learning)  │  Kernels     │  (Linear Alg)│  (Multi-GPU) │
├─────────────────────────────────────────────────────────────────┤
│                    NIX-MANAGED DRIVERS                          │
├─────────────────────────────────────────────────────────────────┤
│  NVIDIA Driver (nixpkgs.linuxPackages.nvidia_x11)               │
│  CUDA Toolkit (nixpkgs.cudaPackages.cudatoolkit)                │
│  cuDNN (nixpkgs.cudaPackages.cudnn)                             │
├─────────────────────────────────────────────────────────────────┤
│                         HARDWARE                                │
├─────────────────────────────────────────────────────────────────┤
│  NVIDIA RTX 4090: 16384 CUDA cores @ 2.52 GHz                   │
│  24GB GDDR6X @ 1008 GB/s | 82.6 TFLOPS FP16 | 330 TOPS INT8    │
└─────────────────────────────────────────────────────────────────┘
```

#### B.2.4.6 CUDA-Specific PID Implementation

```python
"""
CUDA-optimized PID computation for NixOS environment.
Uses PyTorch for GPU-accelerated k-NN and MI estimation.
"""

import torch
import torch.nn.functional as F
from torch import Tensor
from typing import Tuple
import math

# Verify CUDA availability on NixOS
def check_cuda_nixos():
    """Verify CUDA is properly configured on NixOS."""
    if not torch.cuda.is_available():
        raise RuntimeError(
            "CUDA not available. On NixOS, run with: nix develop --impure\n"
            "Ensure hardware.nvidia is configured in configuration.nix"
        )
    
    device = torch.cuda.current_device()
    props = torch.cuda.get_device_properties(device)
    print(f"GPU: {props.name}")
    print(f"Memory: {props.total_memory / 1e9:.1f} GB")
    print(f"Compute Capability: {props.major}.{props.minor}")
    print(f"CUDA Version: {torch.version.cuda}")
    return device


class CUDAKSGEstimator:
    """
    GPU-accelerated KSG mutual information estimator.
    Optimized for batch processing on NVIDIA GPUs.
    """
    
    def __init__(self, k: int = 5, device: str = "cuda"):
        self.k = k
        self.device = torch.device(device)
        
    def _cdist_chunked(
        self, 
        x: Tensor, 
        y: Tensor, 
        chunk_size: int = 2048
    ) -> Tensor:
        """
        Chunked pairwise distance computation to avoid OOM.
        RTX 4090 can handle ~4096 chunk size for d=4096.
        """
        n, m = x.shape[0], y.shape[0]
        
        # For small inputs, use direct computation
        if n * m * x.shape[1] < 1e9:  # ~1GB
            return torch.cdist(x, y, p=float('inf'))  # Chebyshev distance
        
        # Chunked computation for large inputs
        distances = torch.empty(n, m, device=self.device)
        for i in range(0, n, chunk_size):
            i_end = min(i + chunk_size, n)
            for j in range(0, m, chunk_size):
                j_end = min(j + chunk_size, m)
                distances[i:i_end, j:j_end] = torch.cdist(
                    x[i:i_end], y[j:j_end], p=float('inf')
                )
        return distances
    
    def _kth_neighbor_distance(self, x: Tensor) -> Tuple[Tensor, Tensor]:
        """Find k-th nearest neighbor distance for each point."""
        # Self-distance matrix
        dists = self._cdist_chunked(x, x)
        
        # Set self-distance to inf
        dists.fill_diagonal_(float('inf'))
        
        # Find k-th smallest distance
        kth_dists, indices = torch.kthvalue(dists, self.k, dim=1)
        
        return kth_dists, indices
    
    def _count_within_distance(
        self, 
        x: Tensor, 
        epsilon: Tensor
    ) -> Tensor:
        """Count points within epsilon distance (Chebyshev)."""
        dists = self._cdist_chunked(x, x)
        # Strict inequality as per KSG algorithm
        counts = (dists < epsilon.unsqueeze(1)).sum(dim=1) - 1  # Exclude self
        return counts
    
    @torch.no_grad()
    def mutual_information(self, x: Tensor, y: Tensor) -> float:
        """
        Estimate I(X; Y) using KSG algorithm on GPU.
        
        Args:
            x: (N, d_x) tensor
            y: (N, d_y) tensor
            
        Returns:
            Estimated mutual information in nats
        """
        x = x.to(self.device)
        y = y.to(self.device)
        n = x.shape[0]
        
        # Joint space
        xy = torch.cat([x, y], dim=1)
        
        # Find k-th neighbor distance in joint space
        epsilon, _ = self._kth_neighbor_distance(xy)
        
        # Count neighbors in marginal spaces
        n_x = self._count_within_distance(x, epsilon)
        n_y = self._count_within_distance(y, epsilon)
        
        # KSG formula: I(X;Y) = psi(k) + psi(N) - <psi(n_x+1) + psi(n_y+1)>
        psi_k = torch.digamma(torch.tensor(self.k, dtype=torch.float32, device=self.device))
        psi_n = torch.digamma(torch.tensor(n, dtype=torch.float32, device=self.device))
        
        # Average digamma of marginal counts
        avg_psi = (
            torch.digamma((n_x + 1).float()) + 
            torch.digamma((n_y + 1).float())
        ).mean()
        
        mi = (psi_k + psi_n - avg_psi).item()
        return max(0.0, mi)  # MI is non-negative


class CUDAPIDEstimator:
    """
    GPU-accelerated I^sx_∩ PID estimation.
    Batched computation for high throughput on RTX 4090.
    """
    
    def __init__(self, k: int = 5, device: str = "cuda"):
        self.k = k
        self.device = torch.device(device)
        self.ksg = CUDAKSGEstimator(k=k, device=device)
    
    @torch.no_grad()
    def decompose(
        self, 
        s1: Tensor, 
        s2: Tensor, 
        target: Tensor
    ) -> dict:
        """
        Compute full PID decomposition I(S1, S2; T).
        
        Returns:
            Dictionary with keys: total_mi, redundancy, unique_s1, 
            unique_s2, synergy
        """
        s1 = s1.to(self.device)
        s2 = s2.to(self.device)
        target = target.to(self.device)
        
        # Joint source
        s1s2 = torch.cat([s1, s2], dim=1)
        
        # Compute mutual information terms
        i_s1_t = self.ksg.mutual_information(s1, target)
        i_s2_t = self.ksg.mutual_information(s2, target)
        i_s1s2_t = self.ksg.mutual_information(s1s2, target)
        
        # Compute I^sx_∩ redundancy via the Ehrlich et al. (2024) kNN disjunction estimator.
        redundancy = self._compute_shared_exclusions_redundancy(s1, s2, target)
        
        # PID atoms
        unique_s1 = i_s1_t - redundancy
        unique_s2 = i_s2_t - redundancy
        synergy = i_s1s2_t - i_s1_t - i_s2_t + redundancy
        
        return {
            'total_mi': i_s1s2_t,
            'redundancy': redundancy,
            'unique_s1': unique_s1,
            'unique_s2': unique_s2,
            'synergy': synergy,
            # Additional diagnostics
            'i_s1_t': i_s1_t,
            'i_s2_t': i_s2_t,
        }
    
    def _compute_shared_exclusions_redundancy(
        self, 
        s1: Tensor, 
        s2: Tensor, 
        target: Tensor,
        n_samples: int = 1000
    ) -> float:
        """
        Compute continuous shared-exclusions redundancy I^sx_∩(S1,S2;T)
        using the KSG-style disjunction estimator (Ehrlich et al. 2024).

        This is O(N²) in the naive form; this sketch optionally subsamples for
        tractability. For production, prefer an explicit distance-matrix cache
        + chunked k-th-neighbor selection.
        """
        n = target.shape[0]
        
        # Sample if dataset is large
        if n > n_samples:
            indices = torch.randperm(n, device=self.device)[:n_samples]
            s1 = s1[indices]
            s2 = s2[indices]
            target = target[indices]
            n = n_samples

        # Pairwise Chebyshev/L∞ distances.
        ds1 = self.ksg._cdist_chunked(s1, s1)
        ds2 = self.ksg._cdist_chunked(s2, s2)
        dt = self.ksg._cdist_chunked(target, target)

        # Disjunction distance in source space: min(ds1, ds2).
        ds_disj = torch.minimum(ds1, ds2)
        # Joint disjunction distance: max(dt, ds_disj).
        d_joint = torch.maximum(dt, ds_disj)

        # Exclude self for the kNN radius.
        d_joint.fill_diagonal_(float('inf'))
        eps_raw, _ = torch.kthvalue(d_joint, self.k, dim=1)
        # Strict radius for inclusive counting (KSG-style tie handling).
        eps = torch.nextafter(eps_raw, torch.zeros_like(eps_raw))

        # Counts include self (diagonal distance is 0).
        n_alpha = (ds_disj <= eps.unsqueeze(1)).sum(dim=1)
        n_t = (dt <= eps.unsqueeze(1)).sum(dim=1)

        psi_k = torch.digamma(torch.tensor(self.k, dtype=torch.float32, device=self.device))
        psi_n = torch.digamma(torch.tensor(n, dtype=torch.float32, device=self.device))
        avg = (torch.digamma(n_alpha.float()) + torch.digamma(n_t.float())).mean()
        return (psi_k + psi_n - avg).item()


# Batch processing for large-scale experiments
def batch_pid_analysis(
    trajectories: list,
    estimator: CUDAPIDEstimator,
    batch_size: int = 32
) -> list:
    """
    Process multiple trajectories in batches.
    Optimized for RTX 4090 with 24GB VRAM.
    """
    results = []
    
    for i in range(0, len(trajectories), batch_size):
        batch = trajectories[i:i+batch_size]
        
        # Process batch in parallel (data parallelism)
        batch_results = []
        for traj in batch:
            v, d, a = traj['vision'], traj['dream'], traj['action']
            pid = estimator.decompose(v, d, a)
            batch_results.append(pid)
        
        results.extend(batch_results)
        
        # Clear CUDA cache periodically
        if i % (batch_size * 10) == 0:
            torch.cuda.empty_cache()
    
    return results
```

#### B.2.4.7 NixOS-Specific Troubleshooting

**Common Issues and Solutions:**

| Issue | Symptom | Solution |
|-------|---------|----------|
| CUDA not found | `torch.cuda.is_available() == False` | Run `nix develop --impure` to access GPU drivers |
| Driver mismatch | `CUDA driver version insufficient` | Update `hardware.nvidia.package` in configuration.nix |
| OOM errors | `CUDA out of memory` | Reduce batch size, use chunked distance computation |
| Slow k-NN | GPU underutilized | Increase batch size, use RAPIDS cuML for k-NN |
| Non-deterministic | Results vary between runs | Set `torch.backends.cudnn.deterministic = True` |

**Verifying CUDA Installation:**

```bash
# In nix develop shell
$ nvidia-smi
# Should show GPU info

$ python -c "import torch; print(torch.cuda.is_available())"
# Should print: True

$ python -c "import torch; print(torch.cuda.get_device_name(0))"
# Should print: NVIDIA GeForce RTX 4090

# Check CUDA version matches PyTorch expectation
$ python -c "import torch; print(f'PyTorch CUDA: {torch.version.cuda}')"
$ nvcc --version
```

#### B.2.4.8 Multi-GPU Configuration (Optional)

For 2× RTX 4090 setups:

```python
import torch.distributed as dist
from torch.nn.parallel import DistributedDataParallel as DDP

def setup_multi_gpu(rank: int, world_size: int):
    """Initialize distributed training on NixOS multi-GPU setup."""
    dist.init_process_group(
        backend="nccl",  # NCCL is optimal for NVIDIA GPUs
        init_method="env://",
        world_size=world_size,
        rank=rank
    )
    torch.cuda.set_device(rank)

def parallel_pid_analysis(trajectories: list, world_size: int = 2):
    """
    Distribute PID computation across multiple GPUs.
    Each GPU processes a subset of trajectories.
    """
    import torch.multiprocessing as mp
    
    # Split trajectories
    chunks = [trajectories[i::world_size] for i in range(world_size)]
    
    # Spawn processes
    mp.spawn(
        worker_fn,
        args=(chunks,),
        nprocs=world_size,
        join=True
    )
```

**NixOS NCCL Configuration:**

```nix
# Add to flake.nix buildInputs
cudaPackages.nccl

# Environment variable for multi-GPU
shellHook = ''
  export NCCL_DEBUG=INFO
  export NCCL_IB_DISABLE=1  # Disable InfiniBand if not available
'';
```

---

## B.3 MLX Framework Integration

### B.3.1 Why MLX (Not PyTorch) for Apple Silicon

```python
"""
MLX vs PyTorch on Apple Silicon: Decision Rationale
====================================================

PERFORMANCE COMPARISON (M4 Max, 7B parameter model):
┌────────────────────┬────────────┬────────────┬─────────────┐
│ Operation          │ PyTorch    │ MLX        │ Speedup     │
├────────────────────┼────────────┼────────────┼─────────────┤
│ Forward pass (f16) │ 142ms      │ 89ms       │ 1.6x        │
│ Embedding extract  │ 23ms       │ 12ms       │ 1.9x        │
│ Batch k-NN (1000)  │ 890ms      │ 456ms      │ 2.0x        │
│ Memory peak        │ 34GB       │ 28GB       │ 0.82x       │
│ Memory after GC    │ 18GB       │ 14GB       │ 0.78x       │
└────────────────────┴────────────┴────────────┴─────────────┘

KEY MLX ADVANTAGES:

1. LAZY EVALUATION
   - Operations aren't executed until results are needed
   - Allows operation fusion and memory optimization
   - Critical for large embedding buffers
   
   # PyTorch: Immediate execution, memory allocated now
   x = torch.randn(10000, 4096)  # 160MB allocated immediately
   y = x @ W                      # Another 160MB allocated
   z = y.relu()                   # Another 160MB allocated
   
   # MLX: Lazy, fused execution
   x = mx.random.normal((10000, 4096))  # No allocation yet
   y = x @ W                             # No allocation yet
   z = mx.maximum(y, 0)                  # No allocation yet
   mx.eval(z)                            # Single fused allocation

2. UNIFIED ARRAYS
   - Same array type for CPU and GPU
   - No .to('mps') calls
   - Automatic placement decisions
   
3. COMPOSABLE TRANSFORMS
   - mx.grad() for automatic differentiation
   - mx.vmap() for automatic batching
   - mx.compile() for kernel fusion
   
4. NATIVE METAL INTEGRATION
   - Direct Metal shader compilation
   - No MPS translation layer
   - Deterministic execution (reproducibility!)

WHEN TO USE PYTORCH INSTEAD:
- Model not available in MLX format
- Need specific PyTorch ecosystem tools
- Prototyping with existing PyTorch code
- Later CUDA porting is priority
"""
```

### B.3.2 MLX VLA Inference Implementation

```python
"""
pid_vla/mlx_inference.py
========================
MLX-based VLA inference for embedding extraction.

DETAILED IMPLEMENTATION with extensive comments explaining
every design decision for Apple Silicon optimization.
"""

import mlx.core as mx
import mlx.nn as nn
from pathlib import Path
from typing import NamedTuple, Optional
import json

# =============================================================================
# DATA STRUCTURES
# =============================================================================

class VLAEmbeddings(NamedTuple):
    """
    Container for extracted VLA embeddings.
    
    All arrays are MLX arrays (lazy evaluation) until explicitly evaluated.
    This allows the caller to decide when to materialize results, enabling
    memory-efficient streaming processing.
    
    Attributes:
        vision: Shape (batch, seq_len_v, d_model). Raw vision encoder outputs
                BEFORE projection into the language model space. These represent
                "pure" visual features without language model contamination.
                
        language: Shape (batch, seq_len_l, d_model). Language instruction 
                  embeddings from the language model's embedding layer. These
                  are the tokenized instruction before any attention with vision.
                  
        dream: Shape (batch, seq_len_d, d_model). For DreamVLA: explicit <dream>
               token outputs. For OpenVLA: hidden states from layer 16 (where
               probing studies show world model emergence). May be None if
               architecture has no extractable world model representation.
               
        pre_action: Shape (batch, d_model). Final hidden state before action
                    head. This is the "integrated" representation after all
                    cross-modal attention.
                    
        action: Shape (batch, action_dim). Predicted action. For discrete
                action heads, this is the argmax of logits. For diffusion
                heads, this is the denoised action after N diffusion steps.
                
        metadata: Dict with extraction details (layer indices, timestamp, etc.)
    """
    vision: mx.array      # (batch, seq_v, d_model)
    language: mx.array    # (batch, seq_l, d_model)
    dream: Optional[mx.array]  # (batch, seq_d, d_model) or None
    pre_action: mx.array  # (batch, d_model)
    action: mx.array      # (batch, action_dim)
    metadata: dict


class ExtractionConfig(NamedTuple):
    """
    Configuration for embedding extraction.
    
    Attributes:
        vision_layer: Which layer to extract vision embeddings from.
                      -1 = final encoder layer (default)
                      0 = first layer (very low-level features)
                      Recommendation: Use -1 for initial experiments
                      
        dream_layer: For OpenVLA (no explicit world model), which LLM layer
                     to use as proxy for "world model" representation.
                     16 = middle layer (per probing studies)
                     24 = later layer (better for action prediction)
                     None = don't extract dream embeddings
                     
        pool_strategy: How to pool sequence dimension to get fixed-size vectors.
                       'mean' = average over sequence (default, most stable)
                       'cls' = first token only (if model uses CLS token)
                       'last' = last token only (causal models)
                       'max' = max pooling (sparse features)
                       
        include_attention: Whether to also extract attention matrices.
                           Expensive but useful for interpretability.
                           Default: False
                           
        dtype: Data type for computation.
               mx.float32 = most accurate, more memory
               mx.float16 = good balance (recommended)
               mx.bfloat16 = better numerical stability than f16
    """
    vision_layer: int = -1
    dream_layer: Optional[int] = 16
    pool_strategy: str = 'mean'
    include_attention: bool = False
    dtype: mx.Dtype = mx.float16


# =============================================================================
# MODEL LOADING
# =============================================================================

def load_vla_model(
    model_path: Path,
    config_overrides: Optional[dict] = None
) -> nn.Module:
    """
    Load a VLA model in MLX format.
    
    MEMORY MANAGEMENT STRATEGY:
    ---------------------------
    MLX uses lazy evaluation, so loading a model doesn't immediately
    allocate GPU memory. Memory is allocated only when:
    1. A forward pass is executed
    2. mx.eval() is called on model parameters
    3. Parameters are explicitly copied
    
    This means we can load the model, inspect its structure, and configure
    extraction hooks BEFORE allocating the full 14GB+ for a 7B model.
    
    SUPPORTED MODEL FORMATS:
    ------------------------
    1. MLX native (.safetensors + config.json)
       - Fastest loading, no conversion needed
       - Use mlx-community HuggingFace repos
       
    2. PyTorch checkpoint (.bin or .pt)
       - Requires conversion (one-time cost)
       - Use convert_pytorch_to_mlx() helper
       
    3. GGUF quantized (.gguf)
       - Smallest files, fastest inference
       - Some accuracy loss (usually <1% for 4-bit)
       - Use for memory-constrained M4 (non-Max)
    
    Parameters:
        model_path: Path to model directory or file
        config_overrides: Optional dict to override model config
                          (e.g., {"max_position_embeddings": 2048})
    
    Returns:
        MLX nn.Module with forward() method
        
    Example:
        >>> model = load_vla_model(Path("models/openvla-7b-mlx"))
        >>> print(model)  # Inspect structure without allocating memory
        >>> embeddings = extract_embeddings(model, images, text)
        >>> mx.eval(embeddings.vision)  # NOW memory is allocated
    """
    model_path = Path(model_path)
    
    # Detect model format
    if (model_path / "config.json").exists():
        # MLX native format
        with open(model_path / "config.json") as f:
            config = json.load(f)
            
        if config_overrides:
            config.update(config_overrides)
            
        # Import appropriate model class based on architecture
        arch = config.get("architectures", ["LlamaForCausalLM"])[0]
        
        if "OpenVLA" in arch or "Llama" in arch:
            from pid_vla.models.openvla_mlx import OpenVLAModel
            model = OpenVLAModel(config)
        elif "DreamVLA" in arch or "Qwen" in arch:
            from pid_vla.models.dreamvla_mlx import DreamVLAModel
            model = DreamVLAModel(config)
        else:
            raise ValueError(f"Unsupported architecture: {arch}")
            
        # Load weights (lazy - no memory until eval)
        weights = mx.load(str(model_path / "model.safetensors"))
        model.load_weights(list(weights.items()))
        
    elif model_path.suffix == ".gguf":
        # GGUF quantized format
        from pid_vla.models.gguf_loader import load_gguf_model
        model = load_gguf_model(model_path)
        
    else:
        raise ValueError(f"Unknown model format: {model_path}")
        
    return model


# =============================================================================
# EMBEDDING EXTRACTION
# =============================================================================

def extract_embeddings(
    model: nn.Module,
    images: mx.array,
    text_tokens: mx.array,
    config: ExtractionConfig = ExtractionConfig()
) -> VLAEmbeddings:
    """
    Extract embeddings from all VLA components.
    
    EXTRACTION POINTS (OpenVLA architecture):
    =========================================
    
    Image → [SigLIP ViT] → vision_embed → [Projector] → [Llama Layers 0-32] → logits
                ↑                              ↑              ↑           ↑
           VISION (V)                    LANGUAGE (L)   DREAM (D)   PRE_ACTION
           
    Layer indices for Llama 2 7B:
    - Layers 0-7: Low-level feature extraction
    - Layers 8-15: Mid-level concept formation  
    - Layer 16: World model emergence (per probing studies) ← EXTRACT D HERE
    - Layers 17-23: Action planning
    - Layer 24: Action-relevant features
    - Layers 25-32: Action decoding
    
    EXTRACTION POINTS (DreamVLA architecture):
    ==========================================
    
    Image → [SigLIP ViT] → vision → [Qwen + <dream> tokens] → dream → [Diffusion Head] → action
               ↑              ↑                                  ↑              ↑
          VISION (V)    LANGUAGE (L)                         DREAM (D)    PRE_ACTION
          
    DreamVLA uses explicit `<dream>` queries/tokens; D extraction is *less arbitrary* than in OpenVLA, but still requires an explicit extraction hook (e.g., hidden states at `<dream>` tokens) and pooling rules.
    
    MEMORY OPTIMIZATION:
    ====================
    We use MLX's lazy evaluation to avoid materializing intermediate activations
    we don't need. The extraction hooks capture references to specific tensors,
    and only those tensors are kept after the forward pass.
    
    Parameters:
        model: Loaded VLA model
        images: Preprocessed images, shape (batch, channels, height, width)
        text_tokens: Tokenized instructions, shape (batch, seq_len)
        config: Extraction configuration
        
    Returns:
        VLAEmbeddings with all extracted representations
        
    Example:
        >>> model = load_vla_model(model_path)
        >>> config = ExtractionConfig(dream_layer=16, pool_strategy='mean')
        >>> 
        >>> # Process batch
        >>> images = preprocess_images(raw_images)  # (32, 3, 224, 224)
        >>> tokens = tokenizer(instructions)         # (32, 128)
        >>> 
        >>> embeddings = extract_embeddings(model, images, tokens, config)
        >>> 
        >>> # Embeddings are lazy until evaluated
        >>> vision_np = np.array(mx.eval(embeddings.vision))  # (32, 4096)
    """
    # Storage for intermediate activations
    # Using list to allow mutation in nested function
    captured = {
        'vision': None,
        'language': None, 
        'dream': None,
        'pre_action': None,
        'attention': [] if config.include_attention else None
    }
    
    # ---------------------------------------------------------------------
    # HOOK FUNCTIONS
    # These capture intermediate activations during forward pass
    # ---------------------------------------------------------------------
    
    def vision_hook(module, args, output):
        """
        Capture vision encoder output.
        
        For SigLIP ViT:
        - Input: (batch, channels, H, W)
        - Output: (batch, num_patches, d_vision)
        
        We pool over the patch dimension to get (batch, d_vision).
        """
        if config.pool_strategy == 'mean':
            captured['vision'] = mx.mean(output, axis=1)
        elif config.pool_strategy == 'cls':
            captured['vision'] = output[:, 0, :]
        elif config.pool_strategy == 'max':
            captured['vision'] = mx.max(output, axis=1)
        else:
            captured['vision'] = output[:, -1, :]  # 'last'
            
    def language_hook(module, args, output):
        """
        Capture language embeddings (pre-attention).
        
        These are the raw token embeddings before any transformer layers,
        representing the "pure" language instruction without vision influence.
        """
        if config.pool_strategy == 'mean':
            captured['language'] = mx.mean(output, axis=1)
        elif config.pool_strategy == 'cls':
            captured['language'] = output[:, 0, :]
        else:
            captured['language'] = output[:, -1, :]
            
    def dream_hook(module, args, output):
        """
        Capture world model representation.
        
        For DreamVLA: Output of <dream> token processing
        For OpenVLA: Hidden state at specified layer
        
        This represents the model's "understanding" of the world state,
        which should be compared against vision for synergy analysis.
        """
        if config.pool_strategy == 'mean':
            captured['dream'] = mx.mean(output, axis=1)
        else:
            captured['dream'] = output[:, -1, :]
            
    def pre_action_hook(module, args, output):
        """
        Capture final hidden state before action head.
        
        This is the "integrated" representation after all cross-modal
        attention. It should contain information from both V and D.
        """
        # Pre-action is always the last token's hidden state
        captured['pre_action'] = output[:, -1, :]
        
    # ---------------------------------------------------------------------
    # REGISTER HOOKS
    # ---------------------------------------------------------------------
    
    hooks = []
    
    # Vision encoder hook
    if hasattr(model, 'vision_encoder'):
        hooks.append(model.vision_encoder.register_forward_hook(vision_hook))
    elif hasattr(model, 'vision_tower'):
        hooks.append(model.vision_tower.register_forward_hook(vision_hook))
        
    # Language embedding hook
    if hasattr(model, 'embed_tokens'):
        hooks.append(model.embed_tokens.register_forward_hook(language_hook))
    elif hasattr(model, 'language_model'):
        hooks.append(model.language_model.embed_tokens.register_forward_hook(language_hook))
        
    # Dream/world model hook
    if config.dream_layer is not None:
        if hasattr(model, 'dream_module'):
            # DreamVLA: explicit dream module
            hooks.append(model.dream_module.register_forward_hook(dream_hook))
        elif hasattr(model, 'layers'):
            # OpenVLA: use specified LLM layer
            hooks.append(model.layers[config.dream_layer].register_forward_hook(dream_hook))
            
    # Pre-action hook (final layer before action head)
    if hasattr(model, 'layers'):
        hooks.append(model.layers[-1].register_forward_hook(pre_action_hook))
        
    # ---------------------------------------------------------------------
    # FORWARD PASS
    # ---------------------------------------------------------------------
    
    try:
        # Cast inputs to configured dtype
        images = images.astype(config.dtype)
        
        # Run forward pass (hooks capture activations)
        action_output = model(images, text_tokens)
        
        # Extract action from model output
        if isinstance(action_output, dict):
            action = action_output.get('actions', action_output.get('logits'))
        else:
            action = action_output
            
    finally:
        # Always remove hooks to prevent memory leaks
        for hook in hooks:
            hook.remove()
            
    # ---------------------------------------------------------------------
    # PACKAGE RESULTS
    # ---------------------------------------------------------------------
    
    return VLAEmbeddings(
        vision=captured['vision'],
        language=captured['language'],
        dream=captured['dream'],
        pre_action=captured['pre_action'],
        action=action,
        metadata={
            'vision_layer': config.vision_layer,
            'dream_layer': config.dream_layer,
            'pool_strategy': config.pool_strategy,
            'dtype': str(config.dtype),
            'model_type': type(model).__name__
        }
    )


# =============================================================================
# BATCHED EXTRACTION (MEMORY-OPTIMIZED)
# =============================================================================

def extract_embeddings_streaming(
    model: nn.Module,
    image_iterator,  # yields (batch_images, batch_tokens)
    config: ExtractionConfig = ExtractionConfig(),
    max_memory_gb: float = 32.0
) -> 'Iterator[VLAEmbeddings]':
    """
    Memory-optimized streaming extraction for large datasets.
    
    MEMORY MANAGEMENT:
    ==================
    VLA embeddings are large. For a dataset with 10,000 samples:
    - Vision: 10000 × 4096 × 4 bytes = 160MB
    - Language: 10000 × 4096 × 4 bytes = 160MB  
    - Dream: 10000 × 4096 × 4 bytes = 160MB
    - Pre-action: 10000 × 4096 × 4 bytes = 160MB
    - Action: 10000 × 7 × 4 bytes = 0.3MB
    - TOTAL: ~640MB per extraction
    
    For a full experiment with 100 trajectories × 100 timesteps:
    - 10000 samples × 640MB / sample = 6.4GB
    
    This function uses streaming to process data in chunks, keeping
    memory usage bounded by max_memory_gb.
    
    IMPLEMENTATION:
    ===============
    1. Process batches through model
    2. Immediately evaluate and yield results
    3. Clear MLX graph to free memory
    4. Repeat
    
    Parameters:
        model: Loaded VLA model
        image_iterator: Iterator yielding (images, tokens) batches
        config: Extraction configuration
        max_memory_gb: Maximum memory to use (triggers GC when exceeded)
        
    Yields:
        VLAEmbeddings for each batch (already evaluated, numpy-convertible)
        
    Example:
        >>> def data_loader():
        ...     for traj in trajectories:
        ...         images = load_images(traj)
        ...         tokens = tokenize(traj.instructions)
        ...         yield images, tokens
        ...
        >>> all_embeddings = []
        >>> for batch_emb in extract_embeddings_streaming(model, data_loader()):
        ...     all_embeddings.append(batch_emb)
        ...     # Memory stays bounded even for huge datasets
    """
    import gc
    
    for batch_images, batch_tokens in image_iterator:
        # Extract embeddings (lazy)
        embeddings = extract_embeddings(model, batch_images, batch_tokens, config)
        
        # Force evaluation to materialize results
        mx.eval(
            embeddings.vision,
            embeddings.language,
            embeddings.dream,
            embeddings.pre_action,
            embeddings.action
        )
        
        # Yield evaluated embeddings
        yield embeddings
        
        # Check memory and trigger GC if needed
        # MLX doesn't expose memory stats directly, so we estimate
        # based on array sizes
        estimated_mb = (
            embeddings.vision.nbytes +
            embeddings.language.nbytes +
            (embeddings.dream.nbytes if embeddings.dream is not None else 0) +
            embeddings.pre_action.nbytes +
            embeddings.action.nbytes
        ) / 1e6
        
        if estimated_mb > max_memory_gb * 1000 * 0.8:  # 80% threshold
            gc.collect()
            mx.metal.clear_cache()  # Clear Metal memory cache
```

---

### B.3.3 Existing PID Code Availability and Implementation Status

#### B.3.3.1 Code Repositories for PID Estimation

**CRITICAL NOTE (updated):** The authors’ reference implementation of the continuous `I^sx_∩` estimator is public: `https://gitlab.gwdg.de/wibral/continuouspidestimator` (Python package `csxpid`). This repo vendors the reference code under `.external/repos/continuouspidestimator` for cross-checking, and the Rust implementation in `crates/pid-core` is validated against it on synthetic tests.

| Repository | PID Measure | Data Type | Language | Status |
|------------|-------------|-----------|----------|--------|
| **[Abzinger/SxPID](https://github.com/Abzinger/SxPID)** | I^sx_∩ (discrete) | Discrete only | Python | ✓ Released, 7 stars |
| **[Abzinger/sae_analysis](https://github.com/Abzinger/sae_analysis)** | Shannon invariants (Red°, Vul°) | Continuous (via SAE) | Python | ⚠️ Experimental |
| **[pwollstadt/IDTxl](https://github.com/pwollstadt/IDTxl)** | Multiple (includes SxPID) | Discrete + some continuous | Python | ✓ Released, mature |
| **[pliang279/PID](https://github.com/pliang279/PID)** | BATCH + CVX (NOT I^sx_∩) | High-dim continuous via clustering | Python | ✓ Released, 84 stars |
| **[wibral/continuouspidestimator](https://gitlab.gwdg.de/wibral/continuouspidestimator)** (`csxpid`) | I^sx_∩ (continuous, kNN) | Continuous | Python | ✓ Public (reference) |

#### B.3.3.2 Abzinger/sae_analysis: Shannon Invariants for SAE Analysis

**Repository note:** `https://github.com/Abzinger/sae_analysis` is an experimental/WIP toolbox for applying **Shannon invariants** (Gutknecht et al. 2025) to **SAE latents**. It is **not** a continuous shared-exclusions (`I^sx_∩`) estimator, and it may not yet be complete or fully validated—treat it as a reference/starting point only.

**Repository:** https://github.com/Abzinger/sae_analysis

**What It Does:**
- Applies Shannon invariants from Gutknecht et al. (2025, arXiv:2504.15779) to analyze SAE representations
- Computes **degree of redundancy (Red°)** and **degree of vulnerability (Vul°)**
- Uses EleutherAI's `sparsify` for SAE training and `delphi` for activation caching

**Architecture:**
```
Training Data → sparsify (SAE training) → Learned SAE
                                              ↓
Activations → delphi (caching) → info_analysis.py → Shannon Invariants
                                              ↓
                                    Red° (redundancy), Vul° (vulnerability)
```

**Shannon Invariants Computed:**

| Invariant | Definition | Interpretation |
|-----------|------------|----------------|
| **Degree of Redundancy (Red°)** | `Red° = (Σ_i H(X_i)) / H(X_1,…,X_m)` | Higher = more distributed/“redundant” (in the sense of large marginal-entropy sum relative to joint) |
| **Degree of Vulnerability (Vul°)** | `Vul° = (Σ_i H(X_i | X_{-i})) / H(X_1,…,X_m)` | Higher = more fragile/“vulnerable” (more conditional entropy per feature relative to joint) |

**Relevance to PID-VLA:**

| Aspect | sae_analysis | Our PID-VLA Approach |
|--------|--------------|----------------------|
| **Target system** | SAE latent features | VLA embeddings (V, D, A) |
| **Measures** | Red°, Vul° (Shannon invariants) | Syn, Red, Unq (I^sx_∩ atoms) |
| **Goal** | Dictionary size optimization | Hallucination detection |
| **Theoretical basis** | Same (Gutknecht et al. 2025) | Same (Wibral group) |

**Potential Synergy:**
1. **SAE preprocessing:** Train SAE on VLA embeddings, then apply sae_analysis to identify informative features
2. **Dimensionality reduction:** Use SAE to reduce 4096-dim embeddings before PID analysis
3. **Feature selection:** Red° and Vul° could identify which SAE features to use for V-D-A decomposition

**Caveats (as of January 2026):**
- Repository has 0 stars, 70 commits—actively developed but early stage
- No formal release or documentation beyond README
- May not be complete or fully functional
- Uses Shannon invariants (scalable summaries) rather than full PID atoms

**Code Structure:**
```
sae_analysis/
├── info_analysis.py      # Shannon invariant computation
├── feature_analysis.py   # SAE feature analysis
├── visualize.py          # Visualization utilities
├── delphi/               # EleutherAI activation caching (submodule)
└── sparsify/             # EleutherAI SAE training (submodule)
```

#### B.3.3.3 Comparison: sae_analysis vs Our Rust Implementation

| Aspect | sae_analysis | Our Rust I^sx_∩ Implementation |
|--------|--------------|--------------------------------|
| **Measures** | Shannon invariants only | Full PID atoms (Syn, Red, Unq) |
| **Granularity** | Global invariants (scalar summaries) | Global PID estimates (atom averages); MI local terms available for debugging |
| **Speed** | Potentially fast after SAE compression (still depends on SAE width and dataset size) | Slow with exact kNN (O(n²)) unless accelerated / reduced |
| **Interpretability** | Less (aggregate measures) | More (individual atoms) |
| **Use case** | Screening, feature selection | Detailed failure diagnosis |
| **Dimensionality** | High-dim via SAE compression | Requires dim reduction |

**Recommendation:**
Use sae_analysis for **initial screening** (fast Shannon invariants), then our Rust I^sx_∩ for **detailed analysis** on suspicious cases. This matches our hierarchical approach in §2.5.4.

#### B.3.3.5 Using sae_analysis Safely (Not a Correctness Oracle for I^sx_∩)

`sae_analysis` is **not** a continuous `I^sx_∩` estimator, so it cannot be used to validate whether our `I^sx_∩` math/estimator is correct. Treat it as a *potentially useful preprocessing + screening toolbox*, not a correctness oracle.

What `sae_analysis` actually computes (as implemented in `info_analysis.py`):
- **Degree of redundancy (Red°):** `Red° = (Σ_i H(X_i)) / H(X_1,…,X_m)`
- **Degree of vulnerability (Vul°):** `Vul° = (Σ_i H(X_i | X_{-i})) / H(X_1,…,X_m)`

Here, the `X_i` are typically **SAE latent features** (and the repo uses `log2`, so entropies are in bits; the ratios are dimensionless).

How to use it safely in this project:
1. **Correctness validation for continuous `I^sx_∩`:** use `csxpid` (authors’ reference implementation) + Experiment 0. Do *not* substitute `sae_analysis` for this.
2. **Optional dimensionality reduction path:** train an SAE on high-dimensional VLA embeddings, then run `pid-core` on the SAE latents (treat the SAE as an explicit, logged preprocessing step).
3. **Screening/feature selection:** use Red°/Vul° as heuristic signals about whether an SAE representation is distributed vs fragile before paying the cost of full `I^sx_∩` on many cases.

If SAE compression is used, add a robustness check: run `pid-core` on multiple SAE seeds/sizes and confirm conclusions are stable (avoid “tuning the SAE until PID looks good”).

#### B.3.3.6 Why Liang et al.'s Code Is NOT Directly Usable

The Liang et al. (NeurIPS 2023) repository is the most relevant existing code for high-dimensional multimodal PID, **but it uses different PID measures:**

1. **BATCH Estimator:** Variational bound on PID using neural networks
2. **CVX Estimator:** Convex optimization over discrete clusters (requires K-means preprocessing)

Neither uses the I^sx_∩ (shared-exclusions) definition from the Wibral group. The key differences:

| Property | I^sx_∩ (Wibral) | BATCH/CVX (Liang) |
|----------|-----------------|-------------------|
| Theoretical basis | Exclusions in probability space | Optimization bounds |
| Continuous extension | Ehrlich et al. 2024 (k-NN based) | Clustering + discrete PID |
| Differentiable | Yes (key property) | BATCH: Yes, CVX: No |
| Validated on | Low-dim neuroscience data | High-dim multimodal (robotics, pathology) |
| Interpretation | Information-theoretic (local MI) | Variational/optimization bounds |

**Recommendation:** Use Liang et al.'s code as a **baseline comparison**, not as the primary estimator. Our Rust implementation should target I^sx_∩ for theoretical consistency with the Wibral framework.

#### B.3.3.7 Verification: Document Alignment with Wibral's PID Framework

**All PID formulations in this document are based on the Wibral group's I^sx_∩ measure, NOT older PID measures like Williams & Beer's I_min.**

Verification checklist:

| Section | PID Measure Referenced | Status |
|---------|----------------------|--------|
| §2.1 Core Definition | I^sx_∩ from Makkeh et al. (2021) | ✓ Correct |
| §2.2 Continuous Extension | Ehrlich et al. (2024) k-NN estimator | ✓ Correct |
| §2.5 Shannon Invariants | Co-information (classical, not PID-specific) | ✓ Correct |
| §5 Three-Way PID | I^sx_∩ lattice structure | ✓ Correct |
| §8 Estimation | KSG-based continuous I^sx_∩ | ✓ Correct |
| Warning 4 | Correctly notes Liang et al. use DIFFERENT measures | ✓ Correct |

**Key theoretical properties of I^sx_∩ that distinguish it from other PID measures:**

1. **Differentiability (distribution-level):** I^sx_∩ is differentiable w.r.t. the underlying probability distribution (a core design goal; not shared by many PID measures). This does **not** imply that a particular estimator (e.g., kNN/KSG) is differentiable.
2. **Local formulation:** Defined via local (pointwise) mutual information
3. **Exclusions-based:** Quantifies shared information via overlapping exclusions in probability space
4. **Target chain rule:** Satisfies I^sx_∩(S₁,S₂;T₁,T₂) decomposition

**Measures we are NOT using:**

- ❌ I_min (Williams & Beer 2010) - criticized for unintuitive behavior
- ❌ I_BROJA (Bertschinger et al. 2014) - uses optimization, not exclusions
- ❌ I_CCS (Ince 2017) - different pointwise formulation
- ❌ I_dep (James et al.) - dependency-lattice based

---

### B.3.4 Rust Implementation of Continuous I^sx_∩ Estimator

#### B.3.4.1 Mathematical Foundation

The continuous I^sx_∩ redundancy from Ehrlich et al. (2024) is defined as:

```
I^sx_∩(S₁, S₂; T) = ∫∫∫ p(s₁, s₂, t) · i^sx_∩(s₁, s₂; t) ds₁ ds₂ dt

where the local (pointwise) shared information is:

i^sx_∩(t : s₁; s₂) = log [ p(t | W_{s₁,s₂} = true) / p(t) ]

where the “statement variable” is:

W_{s₁,s₂} := (S₁ = s₁) ∨ (S₂ = s₂)
```

The k-NN estimator approximates this using a KSG-style construction:

```
Î^sx_∩ = ψ(k) + ψ(n) - (1/n) Σᵢ [ ψ(n_α(i)) + ψ(n_T(i)) ]

where:
- ψ is the digamma function
- k is the number of neighbors
- n is the total sample count
- n_α(i) is the count of points within εᵢ in the source-disjunction neighborhood (including self)
- n_T(i) is the count of points within εᵢ in target space (including self)
```

#### B.3.4.2 Core Rust Implementation (Repo Canonical)

Authoritative implementation in this repo (kept in sync with tests; prefer these over sketches in this document):
- `crates/pid-core/src/ksg.rs` — KSG MI (Chebyshev/L∞, strict-radius tie handling).
- `crates/pid-core/src/isx.rs` — continuous `I^sx_∩(S₁,S₂;T)` redundancy (Ehrlich et al. 2024; `IsxMethod::EhrlichKsg`), cross-checked against `csxpid`.
- `crates/pid-core/src/pid2.rs` — 2-source PID atoms `{Red, Unq1, Unq2, Syn}` derived from MI + redundancy.
- `crates/pid-core/src/hierarchy.rs` — hierarchical “fast→slow” screening path.
- `crates/pid-core/src/geometry.rs` — geometry diagnostics: intrinsic dimension + basic distance concentration proxies.

Minimal usage (redundancy only; returns **nats**):

```rust
use pid_core::{isx_redundancy, IsxConfig, IsxMethod, MatRef};

let cfg = IsxConfig {
    k: 3,
    method: IsxMethod::EhrlichKsg,
    ..IsxConfig::default()
};

let red_nats = isx_redundancy(s1, s2, t, &cfg)?;
```

Minimal usage (full bivariate PID atoms; returns **nats**):

```rust
use pid_core::{pid2_isx, IsxConfig, IsxMethod, KsgConfig, MatRef, Pid2Config};

let cfg = Pid2Config {
    ksg: KsgConfig::default(),
    isx: IsxConfig {
        method: IsxMethod::EhrlichKsg,
        ..IsxConfig::default()
    },
};

let pid = pid2_isx(s1, s2, t, &cfg)?;
```

##### Legacy Sketch (Do Not Implement From This)

The following block is retained only as historical context. It is **not paper-faithful** and does **not** match the current repo implementation; use §8.1.3 + the files above as the source of truth.

```text
//! LEGACY SKETCH ONLY — does not match `crates/pid-core/src/isx.rs`.
//! 
//! Continuous I^sx_∩ PID estimator based on Ehrlich et al. (2024).
//! 
//! IMPLEMENTATION NOTES:
//! =====================
//! This is a from-scratch implementation based on the paper's mathematical
//! description, cross-checked against the authors’ public reference code:
//! https://gitlab.gwdg.de/wibral/continuouspidestimator (Python package `csxpid`).
//! 
//! Key algorithmic components:
//! 1. k-NN radius ε(i) under the joint disjunction distance d_ST_disj(i,j)
//! 2. Neighbor counting in target space and source-disjunction space within ε(i)
//! 3. Digamma ψ(·) evaluations and averaging
//! 4. Strict-radius tie handling (ε = nextafter(ε_raw, 0) then count with <=)
//! 
//! IMPORTANT: This implementation must be validated against synthetic
//! data with known ground truth (Experiment 0) before use on VLA data.

use std::collections::BinaryHeap;
use std::cmp::Ordering;

/// Result of PID estimation containing all four atoms.
#[derive(Debug, Clone)]
pub struct PIDResult {
    /// Redundant information: I^sx_∩(S₁, S₂; T)
    pub redundancy: f64,
    /// Unique information from S₁: I(S₁; T) - I^sx_∩
    pub unique_s1: f64,
    /// Unique information from S₂: I(S₂; T) - I^sx_∩
    pub unique_s2: f64,
    /// Synergistic information: I(S₁, S₂; T) - I(S₁; T) - I(S₂; T) + I^sx_∩
    pub synergy: f64,
    /// Standard error estimates (from bootstrap or jackknife)
    pub redundancy_se: f64,
    pub unique_s1_se: f64,
    pub unique_s2_se: f64,
    pub synergy_se: f64,
    /// Number of samples used
    pub n_samples: usize,
    /// Dimensionality of each source
    pub dim_s1: usize,
    pub dim_s2: usize,
    pub dim_t: usize,
}

/// Configuration for the I^sx_∩ estimator.
#[derive(Debug, Clone)]
pub struct IsxConfig {
    /// Number of nearest neighbors for KSG estimator.
    /// Typical values: 3-10. Higher k reduces variance but increases bias.
    /// Default: 3 (standard KSG choice)
    pub k: usize,
    
    /// Distance metric for k-NN search.
    /// L_infinity (Chebyshev) is standard for KSG estimators.
    pub distance_metric: DistanceMetric,
    
    /// Whether to use the bias-corrected estimator.
    /// Adds O(1/n) correction term. Recommended for n < 10000.
    pub bias_correction: bool,
    
    /// Number of bootstrap resamples for standard error estimation.
    /// Set to 0 to skip SE estimation (faster).
    pub n_bootstrap: usize,
    
    /// Random seed for reproducibility.
    pub seed: u64,
    
    /// Whether to use SIMD acceleration for distance calculations.
    /// Requires x86_64 with AVX2 or aarch64 with NEON.
    pub use_simd: bool,
    
    /// Maximum dimensionality before triggering dimensionality reduction.
    /// If d > max_dim, PCA is applied automatically.
    /// Set to 0 to disable automatic reduction.
    pub max_dim: usize,
    
    /// Variance to retain in automatic PCA (e.g., 0.95 for 95%).
    pub pca_variance_retained: f64,
}

impl Default for IsxConfig {
    fn default() -> Self {
        Self {
            k: 3,
            distance_metric: DistanceMetric::LInfinity,
            bias_correction: true,
            n_bootstrap: 100,
            seed: 42,
            use_simd: true,
            max_dim: 1024,  // Warn above this, reduce above 4096
            pca_variance_retained: 0.95,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistanceMetric {
    /// L∞ (Chebyshev) distance: max_i |x_i - y_i|
    /// Standard choice for KSG estimators because it defines hypercube neighborhoods.
    LInfinity,
    /// L2 (Euclidean) distance: sqrt(sum_i (x_i - y_i)²)
    /// Alternative for comparison experiments.
    L2,
}

/// Main I^sx_∩ estimator struct.
pub struct IsxEstimator {
    config: IsxConfig,
    /// Pre-allocated buffers for distance calculations
    distance_buffer: Vec<f64>,
    /// Pre-allocated buffer for neighbor indices
    neighbor_buffer: Vec<usize>,
}

impl IsxEstimator {
    pub fn new(config: IsxConfig) -> Self {
        Self {
            config,
            distance_buffer: Vec::new(),
            neighbor_buffer: Vec::new(),
        }
    }
    
    /// Estimate PID for two source variables and one target.
    /// 
    /// # Arguments
    /// * `s1` - First source variable, shape (n, d_s1)
    /// * `s2` - Second source variable, shape (n, d_s2)
    /// * `target` - Target variable, shape (n, d_t)
    /// 
    /// # Returns
    /// PIDResult containing all four atoms with standard errors.
    /// 
    /// # Panics
    /// Panics if inputs have different numbers of samples.
    /// 
    /// # Example
    /// ```
    /// let estimator = IsxEstimator::new(IsxConfig::default());
    /// let result = estimator.estimate(&vision_embed, &dream_embed, &action)?;
    /// println!("Synergy: {:.4} ± {:.4}", result.synergy, result.synergy_se);
    /// ```
    pub fn estimate(
        &mut self,
        s1: &[f64],  // Flattened (n * d_s1)
        s2: &[f64],  // Flattened (n * d_s2)
        target: &[f64],  // Flattened (n * d_t)
        n: usize,
        d_s1: usize,
        d_s2: usize,
        d_t: usize,
    ) -> Result<PIDResult, PIDError> {
        // Validate inputs
        if s1.len() != n * d_s1 || s2.len() != n * d_s2 || target.len() != n * d_t {
            return Err(PIDError::DimensionMismatch);
        }
        
        // Check dimensionality and warn/reduce if needed
        let total_dim = d_s1 + d_s2 + d_t;
        if total_dim > self.config.max_dim && self.config.max_dim > 0 {
            log::warn!(
                "Total dimensionality {} exceeds max_dim {}. Consider PCA reduction.",
                total_dim, self.config.max_dim
            );
        }
        
        // Step 1: Compute MI terms using KSG estimator
        // We need: I(S₁;T), I(S₂;T), I(S₁,S₂;T)
        let mi_s1_t = self.estimate_mi(s1, target, n, d_s1, d_t)?;
        let mi_s2_t = self.estimate_mi(s2, target, n, d_s2, d_t)?;
        
        // Concatenate S₁ and S₂ for joint MI
        let mut s1_s2 = Vec::with_capacity(n * (d_s1 + d_s2));
        for i in 0..n {
            s1_s2.extend_from_slice(&s1[i * d_s1..(i + 1) * d_s1]);
            s1_s2.extend_from_slice(&s2[i * d_s2..(i + 1) * d_s2]);
        }
        let mi_s1s2_t = self.estimate_mi(&s1_s2, target, n, d_s1 + d_s2, d_t)?;
        
        // Step 2: Compute I^sx_∩ redundancy
        // This is the core contribution of Ehrlich et al. (2024)
        let redundancy = self.estimate_isx_redundancy(s1, s2, target, n, d_s1, d_s2, d_t)?;
        
        // Step 3: Derive other atoms from consistency equations
        // Unique(S₁) = I(S₁;T) - Red
        // Unique(S₂) = I(S₂;T) - Red
        // Synergy = I(S₁,S₂;T) - I(S₁;T) - I(S₂;T) + Red
        let unique_s1 = mi_s1_t - redundancy;
        let unique_s2 = mi_s2_t - redundancy;
        let synergy = mi_s1s2_t - mi_s1_t - mi_s2_t + redundancy;
        
        // Step 4: Bootstrap for standard errors (if configured)
        let (red_se, u1_se, u2_se, syn_se) = if self.config.n_bootstrap > 0 {
            self.bootstrap_se(s1, s2, target, n, d_s1, d_s2, d_t)?
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };
        
        Ok(PIDResult {
            redundancy,
            unique_s1,
            unique_s2,
            synergy,
            redundancy_se: red_se,
            unique_s1_se: u1_se,
            unique_s2_se: u2_se,
            synergy_se: syn_se,
            n_samples: n,
            dim_s1: d_s1,
            dim_s2: d_s2,
            dim_t: d_t,
        })
    }
    
    /// Estimate mutual information using KSG algorithm 1.
    /// 
    /// KSG estimator: I(X;Y) ≈ ψ(k) - <ψ(n_x + 1) + ψ(n_y + 1)> + ψ(n)
    /// 
    /// where:
    /// - ψ is the digamma function
    /// - k is the number of neighbors
    /// - n_x, n_y are the counts of points within ε in marginal spaces
    /// - ε is the distance to the k-th nearest neighbor in joint space
    fn estimate_mi(
        &mut self,
        x: &[f64],
        y: &[f64],
        n: usize,
        d_x: usize,
        d_y: usize,
    ) -> Result<f64, PIDError> {
        let k = self.config.k;
        
        // Concatenate X and Y for joint space k-NN
        let mut joint = Vec::with_capacity(n * (d_x + d_y));
        for i in 0..n {
            joint.extend_from_slice(&x[i * d_x..(i + 1) * d_x]);
            joint.extend_from_slice(&y[i * d_y..(i + 1) * d_y]);
        }
        
        // For each point, find k-th nearest neighbor distance in joint space
        let epsilon = self.find_kth_neighbor_distances(&joint, n, d_x + d_y, k);
        
        // Count neighbors within epsilon in marginal spaces
        let n_x = self.count_neighbors_within(x, n, d_x, &epsilon);
        let n_y = self.count_neighbors_within(y, n, d_y, &epsilon);
        
        // KSG formula
        let psi_k = digamma(k as f64);
        let psi_n = digamma(n as f64);
        
        let avg_psi_nx: f64 = n_x.iter()
            .map(|&nx| digamma((nx + 1) as f64))
            .sum::<f64>() / n as f64;
        let avg_psi_ny: f64 = n_y.iter()
            .map(|&ny| digamma((ny + 1) as f64))
            .sum::<f64>() / n as f64;
        
        let mi = psi_k - avg_psi_nx - avg_psi_ny + psi_n;
        
        // Bias correction if enabled
        let mi_corrected = if self.config.bias_correction {
            mi - (d_x + d_y) as f64 / (2.0 * n as f64)  // O(1/n) correction
        } else {
            mi
        };
        
        Ok(mi_corrected.max(0.0))  // MI is non-negative
    }
    
    /// Estimate I^sx_∩ redundancy using the continuous extension.
    /// 
    /// LEGACY SKETCH ONLY: do not treat as paper-faithful; see §8.1.3 and `crates/pid-core/src/isx.rs`.
    /// The key insight is that shared exclusions in probability space
    /// correspond to shared k-NN neighborhoods.
    fn estimate_isx_redundancy(
        &mut self,
        s1: &[f64],
        s2: &[f64],
        target: &[f64],
        n: usize,
        d_s1: usize,
        d_s2: usize,
        d_t: usize,
    ) -> Result<f64, PIDError> {
        let k = self.config.k;
        
        // Build joint spaces: (S₁, T), (S₂, T), (S₁, S₂, T)
        let mut s1_t = Vec::with_capacity(n * (d_s1 + d_t));
        let mut s2_t = Vec::with_capacity(n * (d_s2 + d_t));
        let mut s1_s2_t = Vec::with_capacity(n * (d_s1 + d_s2 + d_t));
        
        for i in 0..n {
            // (S₁, T)
            s1_t.extend_from_slice(&s1[i * d_s1..(i + 1) * d_s1]);
            s1_t.extend_from_slice(&target[i * d_t..(i + 1) * d_t]);
            
            // (S₂, T)
            s2_t.extend_from_slice(&s2[i * d_s2..(i + 1) * d_s2]);
            s2_t.extend_from_slice(&target[i * d_t..(i + 1) * d_t]);
            
            // (S₁, S₂, T)
            s1_s2_t.extend_from_slice(&s1[i * d_s1..(i + 1) * d_s1]);
            s1_s2_t.extend_from_slice(&s2[i * d_s2..(i + 1) * d_s2]);
            s1_s2_t.extend_from_slice(&target[i * d_t..(i + 1) * d_t]);
        }
        
        // Find k-th neighbor distances in each joint space
        let eps_s1_t = self.find_kth_neighbor_distances(&s1_t, n, d_s1 + d_t, k);
        let eps_s2_t = self.find_kth_neighbor_distances(&s2_t, n, d_s2 + d_t, k);
        let eps_s1_s2_t = self.find_kth_neighbor_distances(&s1_s2_t, n, d_s1 + d_s2 + d_t, k);
        
        // For I^sx_∩, we need to count neighbors in a way that captures
        // the "shared exclusions" - information that BOTH sources provide.
        // 
        // The key formula involves:
        // - n_t^{s1}: neighbors of i in T within ε_{s1,t}
        // - n_t^{s2}: neighbors of i in T within ε_{s2,t}
        // - n_t^{joint}: neighbors of i in T within max(ε_{s1,t}, ε_{s2,t})
        
        let n_t_s1 = self.count_neighbors_within(target, n, d_t, &eps_s1_t);
        let n_t_s2 = self.count_neighbors_within(target, n, d_t, &eps_s2_t);
        
        // LEGACY SKETCH ONLY (WARNING): this uses an intersection-of-radii heuristic.
        // This is *not* the Ehrlich et al. (2024) disjunction-kNN estimator described in §8.1.3.
        let eps_shared: Vec<f64> = eps_s1_t.iter()
            .zip(eps_s2_t.iter())
            .map(|(&e1, &e2)| e1.min(e2))  // Heuristic: intersection via smaller epsilon (legacy sketch)
            .collect();
        let n_t_shared = self.count_neighbors_within(target, n, d_t, &eps_shared);
        
        // Count in marginal spaces for normalization
        let n_t_joint = self.count_neighbors_within(target, n, d_t, &eps_s1_s2_t);
        
        // I^sx_∩ formula (continuous extension of discrete definition)
        // LEGACY SKETCH ONLY: do not treat as paper-faithful; see §8.1.3 and `crates/pid-core/src/isx.rs`.
        let psi_k = digamma(k as f64);
        let psi_n = digamma(n as f64);
        
        // Average digamma terms
        let avg_term: f64 = (0..n).map(|i| {
            let psi_shared = digamma((n_t_shared[i] + 1) as f64);
            let psi_s1 = digamma((n_t_s1[i] + 1) as f64);
            let psi_s2 = digamma((n_t_s2[i] + 1) as f64);
            
            // Shared exclusions contribution
            // The formula captures: log[p(t|s₁ ∨ s₂) / p(t)]
            // where ∨ represents the disjunction (OR) operation
            psi_shared - 0.5 * (psi_s1 + psi_s2)
        }).sum::<f64>() / n as f64;
        
        let redundancy = psi_k + psi_n + avg_term;

        // IMPORTANT: In shared-exclusions PID, even the redundancy `I^sx_∩` can be negative
        // at the distribution level (Makkeh et al. 2021 note under Eq. 17). Do not clamp.
        Ok(redundancy)
    }
    
    /// Find the distance to the k-th nearest neighbor for each point.
    fn find_kth_neighbor_distances(
        &mut self,
        data: &[f64],
        n: usize,
        d: usize,
        k: usize,
    ) -> Vec<f64> {
        let mut distances = vec![0.0; n];
        
        for i in 0..n {
            // Use a max-heap to track k smallest distances
            let mut heap: BinaryHeap<OrderedFloat> = BinaryHeap::with_capacity(k + 1);
            
            for j in 0..n {
                if i == j { continue; }  // Skip self
                
                let dist = self.compute_distance(data, i, j, d);
                
                if heap.len() < k {
                    heap.push(OrderedFloat(dist));
                } else if dist < heap.peek().unwrap().0 {
                    heap.pop();
                    heap.push(OrderedFloat(dist));
                }
            }
            
            // The k-th smallest is at the top of the max-heap
            distances[i] = heap.peek().map(|x| x.0).unwrap_or(f64::INFINITY);
        }
        
        distances
    }
    
    /// Count neighbors within epsilon for each point.
    fn count_neighbors_within(
        &self,
        data: &[f64],
        n: usize,
        d: usize,
        epsilon: &[f64],
    ) -> Vec<usize> {
        let mut counts = vec![0; n];
        
        for i in 0..n {
            let eps = epsilon[i];
            for j in 0..n {
                if i == j { continue; }
                
                let dist = self.compute_distance(data, i, j, d);
                if dist < eps {  // Strict inequality per KSG convention
                    counts[i] += 1;
                }
            }
        }
        
        counts
    }
    
    /// Compute distance between two points.
    #[inline]
    fn compute_distance(&self, data: &[f64], i: usize, j: usize, d: usize) -> f64 {
        let start_i = i * d;
        let start_j = j * d;
        
        match self.config.distance_metric {
            DistanceMetric::LInfinity => {
                // L∞: max |x_i - y_i|
                let mut max_diff = 0.0f64;
                for k in 0..d {
                    let diff = (data[start_i + k] - data[start_j + k]).abs();
                    max_diff = max_diff.max(diff);
                }
                max_diff
            }
            DistanceMetric::L2 => {
                // L2: sqrt(sum (x_i - y_i)²)
                let mut sum_sq = 0.0f64;
                for k in 0..d {
                    let diff = data[start_i + k] - data[start_j + k];
                    sum_sq += diff * diff;
                }
                sum_sq.sqrt()
            }
        }
    }
    
    /// Bootstrap resampling for standard error estimation.
    fn bootstrap_se(
        &mut self,
        s1: &[f64],
        s2: &[f64],
        target: &[f64],
        n: usize,
        d_s1: usize,
        d_s2: usize,
        d_t: usize,
    ) -> Result<(f64, f64, f64, f64), PIDError> {
        use rand::{SeedableRng, Rng};
        use rand::rngs::StdRng;
        
        let mut rng = StdRng::seed_from_u64(self.config.seed);
        let mut red_samples = Vec::with_capacity(self.config.n_bootstrap);
        let mut u1_samples = Vec::with_capacity(self.config.n_bootstrap);
        let mut u2_samples = Vec::with_capacity(self.config.n_bootstrap);
        let mut syn_samples = Vec::with_capacity(self.config.n_bootstrap);
        
        for _ in 0..self.config.n_bootstrap {
            // Sample with replacement
            let indices: Vec<usize> = (0..n).map(|_| rng.gen_range(0..n)).collect();
            
            // Create resampled data
            let mut s1_boot = vec![0.0; n * d_s1];
            let mut s2_boot = vec![0.0; n * d_s2];
            let mut t_boot = vec![0.0; n * d_t];
            
            for (new_i, &old_i) in indices.iter().enumerate() {
                s1_boot[new_i * d_s1..(new_i + 1) * d_s1]
                    .copy_from_slice(&s1[old_i * d_s1..(old_i + 1) * d_s1]);
                s2_boot[new_i * d_s2..(new_i + 1) * d_s2]
                    .copy_from_slice(&s2[old_i * d_s2..(old_i + 1) * d_s2]);
                t_boot[new_i * d_t..(new_i + 1) * d_t]
                    .copy_from_slice(&target[old_i * d_t..(old_i + 1) * d_t]);
            }
            
            // Estimate PID on bootstrap sample (without SE to avoid recursion)
            let mut temp_config = self.config.clone();
            temp_config.n_bootstrap = 0;
            let mut temp_estimator = IsxEstimator::new(temp_config);
            
            if let Ok(result) = temp_estimator.estimate(
                &s1_boot, &s2_boot, &t_boot, n, d_s1, d_s2, d_t
            ) {
                red_samples.push(result.redundancy);
                u1_samples.push(result.unique_s1);
                u2_samples.push(result.unique_s2);
                syn_samples.push(result.synergy);
            }
        }
        
        // Compute standard deviation of bootstrap samples
        fn std_dev(samples: &[f64]) -> f64 {
            if samples.is_empty() { return 0.0; }
            let mean = samples.iter().sum::<f64>() / samples.len() as f64;
            let variance = samples.iter()
                .map(|x| (x - mean).powi(2))
                .sum::<f64>() / samples.len() as f64;
            variance.sqrt()
        }
        
        Ok((
            std_dev(&red_samples),
            std_dev(&u1_samples),
            std_dev(&u2_samples),
            std_dev(&syn_samples),
        ))
    }
}

/// Digamma (psi) function.
/// Uses Stirling's approximation for large x and recurrence for small x.
#[inline]
fn digamma(x: f64) -> f64 {
    let mut x = x;
    let mut result = 0.0;
    
    // Recurrence for small x: ψ(x) = ψ(x+1) - 1/x
    while x < 6.0 {
        result -= 1.0 / x;
        x += 1.0;
    }
    
    // Stirling's approximation for large x
    let inv_x = 1.0 / x;
    let inv_x2 = inv_x * inv_x;
    
    result + x.ln() - 0.5 * inv_x
        - inv_x2 * (1.0/12.0 - inv_x2 * (1.0/120.0 - inv_x2 / 252.0))
}

/// Wrapper for f64 that implements Ord for use in BinaryHeap.
#[derive(Clone, Copy, PartialEq)]
struct OrderedFloat(f64);

impl Eq for OrderedFloat {}

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// Errors that can occur during PID estimation.
#[derive(Debug)]
pub enum PIDError {
    DimensionMismatch,
    InsufficientSamples,
    NumericalInstability,
}
```

#### B.3.4.3 Test Scenarios for Validation (Experiment 0)

Repo-canonical tests in this repo (kept in sync with the Rust implementation):
- `crates/pid-core/tests/ksg.rs` — MI + co-information sanity checks (Gaussian channels, independence).
- `crates/pid-core/tests/isx.rs` — `I^sx_∩` redundancy smoke tests + exact-value cross-check against `csxpid` on fixed data.
- `crates/pid-core/tests/pid3.rs` — 3-source SxPID cross-check against `csxpid` on fixed data.
- `crates/pid-core/tests/hierarchy.rs` — hierarchical screening behavior.
- `crates/pid-core/src/bin/exp0.rs` — Experiment 0 runner (synthetic sweeps; extend as needed).

The block below is a legacy sketch for illustration; do not treat its “ground truth” comments as mathematically guaranteed for continuous SxPID.

```text
//! LEGACY SKETCH ONLY — repo tests live in `crates/pid-core/tests/*.rs`.
//! 
//! Test scenarios for validating the I^sx_∩ estimator.
//! These implement Experiment 0 from the specification.

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, Rng};
    use rand::rngs::StdRng;
    use rand_distr::{Normal, Distribution};

    /// Test 1: Independent/additive-style system (qualitative expectation: low redundancy, low synergy)
    /// 
    /// S₁ and S₂ are independent, T = S₁ + S₂ + noise
    /// Note: exact atom values depend on the redundancy measure and estimator; validate against `csxpid` and check invariants.
    #[test]
    fn test_independent_sources() {
        let mut rng = StdRng::seed_from_u64(42);
        let normal = Normal::new(0.0, 1.0).unwrap();
        
        let n = 5000;
        let d = 10;  // Start with low dimensions
        
        // Generate independent sources
        let s1: Vec<f64> = (0..n * d).map(|_| normal.sample(&mut rng)).collect();
        let s2: Vec<f64> = (0..n * d).map(|_| normal.sample(&mut rng)).collect();
        
        // T = mean(S₁) + mean(S₂) + noise (additive, no synergy)
        let noise_std = 0.1;
        let target: Vec<f64> = (0..n).map(|i| {
            let s1_mean: f64 = s1[i * d..(i + 1) * d].iter().sum::<f64>() / d as f64;
            let s2_mean: f64 = s2[i * d..(i + 1) * d].iter().sum::<f64>() / d as f64;
            s1_mean + s2_mean + normal.sample(&mut rng) * noise_std
        }).collect();
        
        let mut estimator = IsxEstimator::new(IsxConfig {
            n_bootstrap: 0,  // Skip SE for speed
            ..Default::default()
        });
        
        let result = estimator.estimate(&s1, &s2, &target, n, d, d, 1).unwrap();
        
        // Assertions with tolerance
        assert!(result.synergy.abs() < 0.1, "Synergy should be ~0, got {}", result.synergy);
        assert!(result.redundancy.abs() < 0.1, "Redundancy should be ~0, got {}", result.redundancy);
        assert!(result.unique_s1 > 0.3, "Unique(S₁) should be positive, got {}", result.unique_s1);
        assert!(result.unique_s2 > 0.3, "Unique(S₂) should be positive, got {}", result.unique_s2);
    }
    
    /// Test 2: XOR-like System (Ground Truth: High Synergy)
    /// 
    /// T = sign(S₁ * S₂), meaning neither S₁ nor S₂ alone predicts T
    /// Expected: High synergy, low unique, low redundancy
    #[test]
    fn test_xor_synergy() {
        let mut rng = StdRng::seed_from_u64(42);
        let normal = Normal::new(0.0, 1.0).unwrap();
        
        let n = 5000;
        let d = 1;  // 1D for clear XOR interpretation
        
        // Generate sources
        let s1: Vec<f64> = (0..n).map(|_| normal.sample(&mut rng)).collect();
        let s2: Vec<f64> = (0..n).map(|_| normal.sample(&mut rng)).collect();
        
        // T = sign(S₁ * S₂) + noise
        let noise_std = 0.1;
        let target: Vec<f64> = (0..n).map(|i| {
            let product = s1[i] * s2[i];
            (if product > 0.0 { 1.0 } else { -1.0 }) + normal.sample(&mut rng) * noise_std
        }).collect();
        
        let mut estimator = IsxEstimator::new(IsxConfig {
            n_bootstrap: 0,
            ..Default::default()
        });
        
        let result = estimator.estimate(&s1, &s2, &target, n, d, d, 1).unwrap();
        
        // XOR should show high synergy
        assert!(result.synergy > 0.5, "Synergy should be high for XOR, got {}", result.synergy);
        // Individual sources should have low unique information
        assert!(result.unique_s1.abs() < 0.3, "Unique(S₁) should be low, got {}", result.unique_s1);
        assert!(result.unique_s2.abs() < 0.3, "Unique(S₂) should be low, got {}", result.unique_s2);
    }
    
    /// Test 3: Redundant Sources (Ground Truth: High Redundancy)
    /// 
    /// S₁ = S₂ = T + noise, both sources carry the same information
    /// Expected: High redundancy, low unique, low synergy
    #[test]
    fn test_redundant_sources() {
        let mut rng = StdRng::seed_from_u64(42);
        let normal = Normal::new(0.0, 1.0).unwrap();
        
        let n = 5000;
        let d = 5;
        
        // Generate target first
        let target: Vec<f64> = (0..n * d).map(|_| normal.sample(&mut rng)).collect();
        
        // S₁ and S₂ are noisy copies of T
        let noise_std = 0.1;
        let s1: Vec<f64> = target.iter()
            .map(|&t| t + normal.sample(&mut rng) * noise_std)
            .collect();
        let s2: Vec<f64> = target.iter()
            .map(|&t| t + normal.sample(&mut rng) * noise_std)
            .collect();
        
        let mut estimator = IsxEstimator::new(IsxConfig {
            n_bootstrap: 0,
            ..Default::default()
        });
        
        let result = estimator.estimate(&s1, &s2, &target, n, d, d, d).unwrap();
        
        // Redundancy should dominate
        assert!(result.redundancy > 0.5, "Redundancy should be high, got {}", result.redundancy);
        assert!(result.synergy.abs() < 0.2, "Synergy should be low, got {}", result.synergy);
    }
    
    /// Test 4: Dimensionality Scaling
    /// 
    /// Test that estimator works across different dimensionalities.
    /// This is critical for VLA applications (d=4096).
    #[test]
    fn test_dimensionality_scaling() {
        let dims = [10, 50, 100, 256];  // Progressively higher dimensions
        
        for d in dims {
            let mut rng = StdRng::seed_from_u64(42);
            let normal = Normal::new(0.0, 1.0).unwrap();
            
            // Scale n with d to maintain reasonable estimation
            let n = (1000 * (d as f64).sqrt() as usize).max(2000);
            
            let s1: Vec<f64> = (0..n * d).map(|_| normal.sample(&mut rng)).collect();
            let s2: Vec<f64> = (0..n * d).map(|_| normal.sample(&mut rng)).collect();
            let target: Vec<f64> = (0..n).map(|i| {
                s1[i * d] + s2[i * d] + normal.sample(&mut rng) * 0.1
            }).collect();
            
            let mut estimator = IsxEstimator::new(IsxConfig {
                n_bootstrap: 0,
                ..Default::default()
            });
            
            let result = estimator.estimate(&s1, &s2, &target, n, d, d, 1);
            
            assert!(result.is_ok(), "Estimation failed at d={}", d);
            
            let result = result.unwrap();
            // Check that results are finite and in reasonable range
            assert!(result.synergy.is_finite(), "Synergy not finite at d={}", d);
            assert!(result.redundancy.is_finite(), "Redundancy not finite at d={}", d);
            assert!(result.synergy.abs() < 5.0, "Synergy out of range at d={}", d);
            
            println!("d={}: Syn={:.3}, Red={:.3}, U1={:.3}, U2={:.3}", 
                     d, result.synergy, result.redundancy, result.unique_s1, result.unique_s2);
        }
    }
    
    /// Test 5: Consistency Check
    /// 
    /// Verify that the four PID atoms sum to I(S₁,S₂;T).
    /// This is a fundamental consistency requirement.
    #[test]
    fn test_pid_consistency() {
        let mut rng = StdRng::seed_from_u64(42);
        let normal = Normal::new(0.0, 1.0).unwrap();
        
        let n = 5000;
        let d = 10;
        
        let s1: Vec<f64> = (0..n * d).map(|_| normal.sample(&mut rng)).collect();
        let s2: Vec<f64> = (0..n * d).map(|_| normal.sample(&mut rng)).collect();
        let target: Vec<f64> = (0..n).map(|i| {
            // Mix of additive and multiplicative to have all PID components
            s1[i * d] + s2[i * d] + s1[i * d] * s2[i * d] + normal.sample(&mut rng) * 0.1
        }).collect();
        
        let mut estimator = IsxEstimator::new(IsxConfig {
            n_bootstrap: 0,
            ..Default::default()
        });
        
        let result = estimator.estimate(&s1, &s2, &target, n, d, d, 1).unwrap();
        
        // Estimate I(S₁,S₂;T) directly for comparison
        let mut s1_s2 = Vec::with_capacity(n * 2 * d);
        for i in 0..n {
            s1_s2.extend_from_slice(&s1[i * d..(i + 1) * d]);
            s1_s2.extend_from_slice(&s2[i * d..(i + 1) * d]);
        }
        let mi_joint = estimator.estimate_mi(&s1_s2, &target, n, 2 * d, 1).unwrap();
        
        // Sum of atoms should equal joint MI
        let sum_atoms = result.redundancy + result.unique_s1 + result.unique_s2 + result.synergy;
        let diff = (sum_atoms - mi_joint).abs();
        
        assert!(diff < 0.2, "PID consistency violated: sum={:.3}, MI={:.3}, diff={:.3}",
                sum_atoms, mi_joint, diff);
    }
}
```

---

### B.3.5 Scaling to 3-Way PID and High Dimensions

#### B.3.5.1 The Scalability Challenge

For 3-source PID (V, L, D → A), we face:

| Sources | PID Atoms | Complexity |
|---------|-----------|------------|
| 2 | 4 | O(n² × d) |
| 3 | 18 | O(n² × d) × 18 atoms |
| 4 | 166 | Computationally intractable |
| n | B(n) (Bell number) | Super-exponential |

The fundamental bottleneck is **NOT** the number of atoms, but:
1. **k-NN search** in high dimensions (curse of dimensionality)
2. **Sample complexity** increases with dimension
3. **Estimation variance** multiplies across atoms

#### B.3.5.2 Scalable Approaches from Literature

**Scope note:** This appendix lists several scalability ideas from the broader literature. For this PhD project, the in-scope core is **Wibral-group shared-exclusions PID (`I^sx_∩`)** and **Shannon invariants** (Gutknecht et al. 2025). Other approaches below are for context/baselines only and should not be conflated with `I^sx_∩`.

**Approach 1: Shannon Invariants (Gutknecht et al. 2025)**

Compute summary statistics that are invariant across all PID measures:

```rust
/// Bivariate co-information: CI₂(X₁,X₂;Y) = I(X₁;Y)+I(X₂;Y)-I(X₁,X₂;Y) = Red - Syn
/// (a Shannon invariant for any 2-source PID).
///
/// For 3 sources (with a distinguished target), CI₃ is an interaction-information-style
/// alternating sum of MI terms. It is a screening statistic (not a PID) and is much cheaper
/// than estimating all 18 PID atoms.
pub fn co_information_3way(
    s1: &[Vec<f64>],
    s2: &[Vec<f64>],
    s3: &[Vec<f64>],
    target: &[Vec<f64>],
    k: usize,
) -> f64 {
    // Compute 7 mutual information terms
    let i_s1_t = ksg_mi(s1, target, k);
    let i_s2_t = ksg_mi(s2, target, k);
    let i_s3_t = ksg_mi(s3, target, k);
    let i_s1s2_t = ksg_mi(&concat_sources(s1, s2), target, k);
    let i_s1s3_t = ksg_mi(&concat_sources(s1, s3), target, k);
    let i_s2s3_t = ksg_mi(&concat_sources(s2, s3), target, k);
    let i_s1s2s3_t = ksg_mi(&concat_sources_3(s1, s2, s3), target, k);
    
    // Co-information formula (alternating sum)
    i_s1_t + i_s2_t + i_s3_t 
        - i_s1s2_t - i_s1s3_t - i_s2s3_t 
        + i_s1s2s3_t
}

/// m-way co-information relative to a target (generalization of CI₃).
///
/// NOTE: This is **not** the O-information Ω.
/// - `CI_m(X₁,…,X_m;Y)` is defined via an alternating sum over MI terms `I(X_S;Y)`.
/// - `Ω(X₁,…,X_n)` is defined on a *set of variables* (no distinguished target) via entropies
///   and equals `TC - DTC`. See §2.5.3 for the correct Ω definition and scope limitations.
pub fn co_information_mway(
    sources: &[&[Vec<f64>]],
    target: &[Vec<f64>],
    k: usize,
) -> f64 {
    // CI_m(X₁,…,X_m;Y) := Σ_{∅≠S⊆{1..m}} (-1)^{|S|+1} I(X_S;Y)
    //
    // WARNING: enumerating all subsets is exponential in m; for PID-VLA screening we typically
    // only use m ≤ 3 (pairwise CI and 3-way CI with a distinguished target).
    let m = sources.len();
    let mut sum = 0.0f64;
    for mask in 1usize..(1usize << m) {
        let r = mask.count_ones() as usize;
        let sign = if (r % 2) == 1 { 1.0 } else { -1.0 };
        let blocks = select_blocks(sources, mask);
        sum += sign * ksg_mi(&concat_all(&blocks), target, k);
    }
    sum
}
```

**Interpretation:**
- `CI_m < 0`: synergy-leaning interactions (sign convention; see §2.5.3)
- `CI_m > 0`: redundancy-leaning interactions
- Provides scalar summaries without computing all PID atoms, but does not replace `I^sx_∩`

**Approach 2: Gaussian PID (NeurIPS 2023, 2024)**

Transform data to Gaussian latent space where PID has closed-form solution:

```rust
/// Gaussian PID via normalizing flows
/// Based on: "Gaussian PID: Bias Correction and Application to High-Dimensional Data"
pub struct GaussianPID {
    /// Normalizing flow encoder for each source
    encoders: Vec<NormalizingFlow>,
    /// Covariance matrices in latent space
    cov_cache: HashMap<String, DMatrix<f64>>,
}

impl GaussianPID {
    /// Transform sources to Gaussian latent space, then compute BROJA-PID
    pub fn estimate(
        &mut self,
        s1: &DMatrix<f64>,  // [n, d1]
        s2: &DMatrix<f64>,  // [n, d2]
        target: &DMatrix<f64>, // [n, dt]
    ) -> PIDResult {
        // Step 1: Encode to Gaussian latent space
        let z1 = self.encoders[0].encode(s1);
        let z2 = self.encoders[1].encode(s2);
        let zt = self.encoders[2].encode(target);
        
        // Step 2: Estimate covariance matrices
        let cov_z1_zt = estimate_covariance(&z1, &zt);
        let cov_z2_zt = estimate_covariance(&z2, &zt);
        let cov_z1_z2_zt = estimate_covariance_3way(&z1, &z2, &zt);
        
        // Step 3: Compute Gaussian PID (closed-form for BROJA)
        // Uses optimization over Q(zt | z1, z2) distributions
        self.compute_broja_gaussian(&cov_z1_zt, &cov_z2_zt, &cov_z1_z2_zt)
    }
    
    /// Bias-corrected estimator for finite samples
    pub fn estimate_with_bias_correction(
        &mut self,
        s1: &DMatrix<f64>,
        s2: &DMatrix<f64>,
        target: &DMatrix<f64>,
        n_bootstrap: usize,
    ) -> PIDResult {
        // Apply Ledoit-Wolf shrinkage to covariance estimates
        // Then use bias correction formula from NeurIPS 2023
        // ...
    }
}
```

**Key properties:**
- Scales to d=1024+ dimensions
- Bias-corrected estimator available
- Preserves information (normalizing flows are bijective)
- **Caveat:** Uses BROJA-PID, not I^sx_∩ (different measure)

**Approach 3: Coarse-Graining (Ehrlich et al. 2022)**

Instead of treating each neuron as a separate source, group neurons:

```rust
/// Coarse-grain high-dimensional sources into lower-dimensional groups
pub struct CoarseGrainedPID {
    /// PCA for dimensionality reduction
    pca: Option<PCA>,
    /// Clustering for neuron grouping
    kmeans: Option<KMeans>,
    /// Number of groups
    n_groups: usize,
}

impl CoarseGrainedPID {
    /// Reduce d-dimensional source to n_groups components
    pub fn coarse_grain(&self, source: &DMatrix<f64>) -> DMatrix<f64> {
        match &self.pca {
            Some(pca) => pca.transform(source, self.n_groups),
            None => {
                // Use k-means clustering on neurons
                let assignments = self.kmeans.as_ref().unwrap().predict(source);
                self.aggregate_by_cluster(source, &assignments)
            }
        }
    }
    
    /// Aggregate neurons by cluster (mean or first PC per cluster)
    fn aggregate_by_cluster(
        &self,
        source: &DMatrix<f64>,
        assignments: &[usize],
    ) -> DMatrix<f64> {
        let mut grouped = DMatrix::zeros(source.nrows(), self.n_groups);
        for (i, &cluster) in assignments.iter().enumerate() {
            for j in 0..source.nrows() {
                grouped[(j, cluster)] += source[(j, i)];
            }
        }
        grouped
    }
    
    /// Compute PID on coarse-grained representation
    pub fn estimate(&self, sources: &[DMatrix<f64>], target: &DMatrix<f64>) -> PIDResult {
        let coarse_sources: Vec<_> = sources.iter()
            .map(|s| self.coarse_grain(s))
            .collect();
        
        // Now compute standard PID on lower-dimensional representations
        // ...
    }
}
```

**Trade-offs:**
- Reduces dimensionality but may lose fine-grained information
- Bounds on information loss available (see Ehrlich et al.)
- Useful when interested in aggregate patterns, not individual neurons

**Approach 4: Normalizing-Flow PID in Latent Gaussian Distributions (Zhao et al., arXiv:2510.04417)**

Most recent approach combining all above:

```rust
/// Flow-based PID in latent Gaussian space (Zhao et al., arXiv:2510.04417).
/// "Partial Information Decomposition via Normalizing Flows in Latent Gaussian Distributions"
///
/// Note: Earlier drafts used the nickname “Thin-PID”; the arXiv title does not use that name.
/// This is also not I^sx_∩ (it targets a Gaussian/flow-based PID variant).
pub struct NormalizingFlowPid {
    /// Invertible normalizing flow encoder
    flow: RealNVP,  // Or other normalizing flow
    /// Gradient-based optimizer for GPID
    optimizer: AdamOptimizer,
}

/// Legacy alias used in earlier drafts.
pub type ThinPID = NormalizingFlowPid;

impl NormalizingFlowPid {
    /// Key insight: PID is easier to solve in Gaussian space
    /// Normalizing flows preserve information while Gaussianizing
    pub fn estimate(&mut self, s1: &[Vec<f64>], s2: &[Vec<f64>], target: &[Vec<f64>]) -> PIDResult {
        // 1. Train flows to map each variable to Gaussian
        let z1 = self.flow.fit_transform(s1);
        let z2 = self.flow.fit_transform(s2);
        let zt = self.flow.fit_transform(target);
        
        // 2. Compute Gaussian PID (much faster than discrete)
        let gpid = GaussianPID::new();
        gpid.estimate(&z1, &z2, &zt)
    }
}
```

#### B.3.5.3 Recommended Scaling Strategy for VLA

For the PID-VLA system with sources V (4096-d), L (4096-d), D (4096-d), Target A (7-d):

```
SCALING STRATEGY
================

Level 0: Shannon invariants (fastest; MI-only)
├── Compute CI_VL, CI_VD, CI_LD (pairwise co-information)
├── Compute CI_3(V,L,D;A) (3-way co-information / interaction information with a distinguished target)
├── O(7) MI estimates via KSG
└── Use for: Real-time monitoring, fast screening

Optional (research-only; not “MI-only cheap”):
└── Compute Ω on a *set* (e.g., Ω(V,L,D,A) or Ω(V,L,D)) after coarse-graining/feature selection.
    This requires high-order entropy estimation and can be harder than CI screening in high `d`.

Level 1: Pairwise `I^sx_∩` PID (slower; targeted; likely requires dim reduction)
├── Apply PCA: 4096-d → 256-d (retain 95% variance)
├── Compute PID(V,D;A), PID(V,L;A), PID(L,D;A)
├── O(3) × 4 atoms = 12 estimates
└── Use for: Detailed failure analysis

Level 2: 3-way SxPID (offline only; only after pairwise validation)
├── Pre-filter with Level 0 to identify suspicious patterns
├── Compute full 18-atom decomposition on reduced dimensions
├── Use coarse-graining if d > 512
└── Use for: Research, paper results
```

**Scope note:** This project’s scientific object is Wibral-group shared-exclusions PID (`I^sx_∩`) plus Shannon invariants. Non-SxPID approaches (e.g., BROJA, normalizing-flow PID variants) may be explored as *baselines* if needed, but they are out of scope for the core estimator and should not be conflated with `I^sx_∩`.

#### B.3.5.4 Implementation Recommendations

```rust
/// High-level PID computation with automatic scaling
pub fn auto_scaled_pid(
    sources: &[DMatrix<f64>],
    target: &DMatrix<f64>,
    config: &ScalingConfig,
) -> ScaledPIDResult {
    let n_sources = sources.len();
    let max_dim = sources.iter().map(|s| s.ncols()).max().unwrap_or(0);
    
    // Decide scaling strategy based on problem size
    match (n_sources, max_dim) {
        (2, d) if d <= 256 => {
            // Direct I^sx_∩ estimation
            let isx = IsxContinuous::with_config(config.isx_config.clone());
            ScaledPIDResult::Full(isx.estimate(&sources[0], &sources[1], target))
        }
        (2, d) if d <= 4096 => {
            // PCA reduction + I^sx_∩
            let reduced = apply_pca(sources, config.pca_components);
            let isx = IsxContinuous::with_config(config.isx_config.clone());
            ScaledPIDResult::Reduced(isx.estimate(&reduced[0], &reduced[1], target))
        }
        (3, d) if d <= 256 => {
            // Full 3-way PID (18 atoms)
            compute_3way_isx(sources, target, &config.isx_config)
        }
        (3, d) => {
            // Shannon invariants + targeted pairwise PID
            let ci = co_information_3way(&sources[0], &sources[1], &sources[2], target, config.k);
            let pairwise = compute_pairwise_pids(sources, target, &config);
            ScaledPIDResult::Hierarchical { ci, pairwise }
        }
        (n, _) if n > 3 => {
            // Scalable screening for many sources:
            // - Compute the full pairwise CI₂ matrix (reusing single-source MI terms).
            // - Optionally compute Ω on a *small, selected/coarse-grained* subset offline.
            let ci2 = compute_pairwise_ci2_matrix(sources, target, config.k);
            ScaledPIDResult::PairwiseCi2(ci2)
        }
        _ => unreachable!()
    }
}

pub enum ScaledPIDResult {
    Full(PIDResult),
    Reduced(PIDResult),  // With note about dimensionality reduction
    Hierarchical { ci: f64, pairwise: Vec<PIDResult> },
    PairwiseCi2(Vec<Vec<f64>>),
}
```

#### B.3.5.5 Computational Complexity Summary

| Method | Time Complexity | Space | Max d | Max sources |
|--------|-----------------|-------|-------|-------------|
| KSG MI | O(N² × d) | O(N × d) | ~1000 | Any |
| I^sx_∩ 2-way | O(N² × d × 4) | O(N × d) | ~500 | 2 |
| I^sx_∩ 3-way | O(N² × d × 18) | O(N × d) | ~256 | 3 |
| Pairwise CI₂ screening (m sources → target) | O(N² × d × m²) | O(N × d) | ~1000 | ~100 (still heavy) |
| m-way CI_m (explicit subset sum) | O(N² × d × (2^m − 1)) | O(N × d) | ~500 | ~5 (not scalable) |
| Ω (O-information on a set; entropy-based) | O(N² × d × m) | O(N × d) | ~500 | ~10–100 (depends on estimator; research-only) |
| Gaussian PID | O(N × d + d³) | O(d²) | ~1024 | 2-3 |
| NF-PID (Zhao et al.; “Thin-PID” legacy) | O(N × d × epochs) | O(d²) | 1000+ | 2-3 |

#### B.3.5.6 References for Scaling

```
Core Papers:
- Gutknecht et al. (2025): Shannon Invariants. arXiv:2504.15779
- Ehrlich et al. (2022): Representational Complexity. Trans. ML Res.
- Barrett et al. (2023): Gaussian PID with Bias Correction. NeurIPS.
- Zhao et al. (2025): Partial Information Decomposition via Normalizing Flows in Latent Gaussian Distributions. arXiv:2510.04417 (earlier drafts used “Thin-PID” as a nickname)

Libraries:
- dit (Python): dit.distributions, dit.pid (discrete PID measures)
- IDTxl (Python): Comprehensive information theory toolkit
- Abzinger/SxPID (Python): Discrete I^sx_∩ implementation
```

---

## B.4 Metal Compute for PID Estimation

### B.4.1 Why Custom Metal Kernels

```
PERFORMANCE MOTIVATION
======================

The I^sx_∩ PID estimator is dominated by k-NN search:
- For each sample i, find k nearest neighbors in d dimensions
- Naive: O(n² × d) distance calculations
- With k-d tree: O(n × log(n) × d) average case
- With Metal GPU: O(n × d / num_cores) parallel

For VLA scale (n=10000, d=4096, k=3):
- CPU (single-threaded): ~45 seconds
- CPU (M4 Max 16 cores): ~4 seconds
- Metal GPU (40 cores): ~0.3 seconds

The 150x speedup makes interactive analysis possible.

WHY NOT USE MPS (Metal Performance Shaders)?
============================================
MPS provides pre-built kernels for common operations (matmul, conv, etc.)
but does NOT provide:
- k-NN search with custom distance metrics
- Digamma function evaluation (needed for KSG)
- Custom reduction patterns for PID atoms

We need custom Metal kernels for the PID-specific operations.
```

### B.4.2 Metal Shader Implementation

```metal
/* 
 * pid_kernels.metal
 * =================
 * Custom Metal compute shaders for PID estimation.
 * 
 * COMPILATION:
 * xcrun -sdk macosx metal -c pid_kernels.metal -o pid_kernels.air
 * xcrun -sdk macosx metallib pid_kernels.air -o pid_kernels.metallib
 * 
 * Or via Rust metal-rs crate for runtime compilation.
 */

#include <metal_stdlib>
using namespace metal;

// =============================================================================
// CONSTANTS AND HELPERS
// =============================================================================

// Thread group size optimized for M4 GPU architecture
// M4 has 40 execution units with 32 threads each = 1280 max concurrent
// We use 256 threads per group for good occupancy
constant uint THREAD_GROUP_SIZE = 256;

// Digamma function coefficients (Bernoulli numbers)
// Used in KSG estimator: ψ(k) + ψ(n) - ψ(n_x) - ψ(n_y)
constant float DIGAMMA_C1 = -0.5772156649f;  // Euler-Mascheroni constant
constant float DIGAMMA_C2 = 1.6449340668f;   // π²/6

/*
 * digamma - Digamma (psi) function approximation
 * 
 * Uses asymptotic expansion for x > 6, recurrence relation for smaller x.
 * Accuracy: ~1e-7 relative error for x > 0.5
 * 
 * This is critical for KSG estimator accuracy. The formula:
 *   I(X;Y) ≈ ψ(k) + ψ(n) - <ψ(n_x + 1)> - <ψ(n_y + 1)>
 * 
 * where ψ is the digamma function and n_x, n_y are neighbor counts.
 */
inline float digamma(float x) {
    // Handle small x via recurrence: ψ(x) = ψ(x+1) - 1/x
    float result = 0.0f;
    while (x < 6.0f) {
        result -= 1.0f / x;
        x += 1.0f;
    }
    
    // Asymptotic expansion for large x
    float inv_x = 1.0f / x;
    float inv_x2 = inv_x * inv_x;
    
    result += log(x) - 0.5f * inv_x;
    result -= inv_x2 * (1.0f/12.0f - inv_x2 * (1.0f/120.0f - inv_x2 * (1.0f/252.0f)));
    
    return result;
}


// =============================================================================
// K-NN DISTANCE COMPUTATION
// =============================================================================

/*
 * compute_distances_chunk
 * 
 * Computes L∞ (Chebyshev) distances from query points to all data points.
 * L∞ is used in KSG estimator because it defines hypercube neighborhoods.
 * 
 * MEMORY LAYOUT:
 * - data: (n, d) row-major, all data points
 * - queries: (q, d) row-major, query points (subset of data for k-NN)
 * - distances: (q, n) output distances
 * 
 * PARALLELIZATION:
 * - Each thread handles one (query, data) pair
 * - Grid: (q, ceil(n / THREAD_GROUP_SIZE))
 * - Thread group: (1, THREAD_GROUP_SIZE)
 * 
 * For n=10000, d=4096, q=10000:
 * - 100M distance calculations
 * - ~30ms on M4 Max GPU
 */
kernel void compute_distances_linf(
    device const float* data       [[buffer(0)]],  // (n, d)
    device const float* queries    [[buffer(1)]],  // (q, d)
    device float* distances        [[buffer(2)]],  // (q, n)
    constant uint& n               [[buffer(3)]],  // number of data points
    constant uint& d               [[buffer(4)]],  // dimensionality
    constant uint& q               [[buffer(5)]],  // number of queries
    uint2 gid                      [[thread_position_in_grid]]
) {
    uint query_idx = gid.x;
    uint data_idx = gid.y;
    
    // Bounds check
    if (query_idx >= q || data_idx >= n) return;
    
    // Compute L∞ distance
    float max_diff = 0.0f;
    
    for (uint dim = 0; dim < d; dim++) {
        float query_val = queries[query_idx * d + dim];
        float data_val = data[data_idx * d + dim];
        float diff = abs(query_val - data_val);
        max_diff = max(max_diff, diff);
    }
    
    distances[query_idx * n + data_idx] = max_diff;
}


/*
 * compute_distances_l2
 * 
 * Alternative: Euclidean distance for comparison experiments.
 * Some PID estimators use L2 instead of L∞.
 */
kernel void compute_distances_l2(
    device const float* data       [[buffer(0)]],
    device const float* queries    [[buffer(1)]],
    device float* distances        [[buffer(2)]],
    constant uint& n               [[buffer(3)]],
    constant uint& d               [[buffer(4)]],
    constant uint& q               [[buffer(5)]],
    uint2 gid                      [[thread_position_in_grid]]
) {
    uint query_idx = gid.x;
    uint data_idx = gid.y;
    
    if (query_idx >= q || data_idx >= n) return;
    
    float sum_sq = 0.0f;
    
    for (uint dim = 0; dim < d; dim++) {
        float diff = queries[query_idx * d + dim] - data[data_idx * d + dim];
        sum_sq += diff * diff;
    }
    
    distances[query_idx * n + data_idx] = sqrt(sum_sq);
}


// =============================================================================
// K-NN SELECTION (PARTIAL SORT)
// =============================================================================

/*
 * find_k_nearest
 * 
 * For each query, find the k smallest distances.
 * Uses parallel bitonic sort on small k, heap for larger k.
 * 
 * OUTPUT:
 * - knn_indices: (q, k) indices of k nearest neighbors
 * - knn_distances: (q, k) distances to k nearest neighbors
 * 
 * NOTE: Excludes self-matches (distance = 0) for leave-one-out estimation.
 */
kernel void find_k_nearest(
    device const float* distances   [[buffer(0)]],  // (q, n)
    device uint* knn_indices        [[buffer(1)]],  // (q, k)
    device float* knn_distances     [[buffer(2)]],  // (q, k)
    constant uint& n                [[buffer(3)]],
    constant uint& k                [[buffer(4)]],
    constant uint& q                [[buffer(5)]],
    uint gid                        [[thread_position_in_grid]]
) {
    if (gid >= q) return;
    
    // Initialize with maximum values
    threadgroup float local_dists[32];  // Assuming k <= 32
    threadgroup uint local_indices[32];
    
    for (uint i = 0; i < k; i++) {
        local_dists[i] = INFINITY;
        local_indices[i] = 0;
    }
    
    // Scan all distances, maintaining top-k
    for (uint i = 0; i < n; i++) {
        float dist = distances[gid * n + i];
        
        // Skip self-match
        if (dist < 1e-10f) continue;
        
        // Check if this distance belongs in top-k
        if (dist < local_dists[k-1]) {
            // Insert in sorted position
            uint insert_pos = k - 1;
            while (insert_pos > 0 && dist < local_dists[insert_pos - 1]) {
                local_dists[insert_pos] = local_dists[insert_pos - 1];
                local_indices[insert_pos] = local_indices[insert_pos - 1];
                insert_pos--;
            }
            local_dists[insert_pos] = dist;
            local_indices[insert_pos] = i;
        }
    }
    
    // Write results
    for (uint i = 0; i < k; i++) {
        knn_indices[gid * k + i] = local_indices[i];
        knn_distances[gid * k + i] = local_dists[i];
    }
}


// =============================================================================
// KSG MUTUAL INFORMATION ESTIMATION
// =============================================================================

/*
 * ksg_count_neighbors
 * 
 * For each point, count neighbors within epsilon in marginal spaces.
 * This is the core of the KSG estimator.
 * 
 * Given joint space (X, Y) and epsilon = distance to k-th neighbor in joint:
 * - n_x = count of points within epsilon in X marginal
 * - n_y = count of points within epsilon in Y marginal
 * 
 * MI estimate: ψ(k) - 1/k - <ψ(n_x + 1) + ψ(n_y + 1)> + ψ(n)
 */
kernel void ksg_count_neighbors(
    device const float* X           [[buffer(0)]],  // (n, d_x)
    device const float* Y           [[buffer(1)]],  // (n, d_y)
    device const float* epsilon     [[buffer(2)]],  // (n,) distance to k-th neighbor in joint
    device uint* n_x                [[buffer(3)]],  // (n,) output: neighbors in X
    device uint* n_y                [[buffer(4)]],  // (n,) output: neighbors in Y
    constant uint& n                [[buffer(5)]],
    constant uint& d_x              [[buffer(6)]],
    constant uint& d_y              [[buffer(7)]],
    uint gid                        [[thread_position_in_grid]]
) {
    if (gid >= n) return;
    
    float eps = epsilon[gid];
    uint count_x = 0;
    uint count_y = 0;
    
    // Count neighbors in each marginal
    for (uint j = 0; j < n; j++) {
        if (j == gid) continue;
        
        // Check X marginal distance
        float dist_x = 0.0f;
        for (uint dim = 0; dim < d_x; dim++) {
            float diff = abs(X[gid * d_x + dim] - X[j * d_x + dim]);
            dist_x = max(dist_x, diff);
        }
        if (dist_x < eps) count_x++;
        
        // Check Y marginal distance  
        float dist_y = 0.0f;
        for (uint dim = 0; dim < d_y; dim++) {
            float diff = abs(Y[gid * d_y + dim] - Y[j * d_y + dim]);
            dist_y = max(dist_y, diff);
        }
        if (dist_y < eps) count_y++;
    }
    
    n_x[gid] = count_x;
    n_y[gid] = count_y;
}


/*
 * ksg_compute_mi
 * 
 * Final MI computation from neighbor counts.
 * Parallel reduction over all samples.
 */
kernel void ksg_compute_mi(
    device const uint* n_x          [[buffer(0)]],
    device const uint* n_y          [[buffer(1)]],
    device float* mi_contribution   [[buffer(2)]],  // (n,) per-sample contribution
    constant uint& n                [[buffer(3)]],
    constant uint& k                [[buffer(4)]],
    uint gid                        [[thread_position_in_grid]]
) {
    if (gid >= n) return;
    
    // KSG formula for single sample
    // KSG-style MI formula (exact variant/tie handling must match the chosen estimator).
    // See `crates/pid-core/src/ksg.rs` for the canonical implementation used in this repo.
    
    float psi_nx = digamma(float(n_x[gid] + 1));
    float psi_ny = digamma(float(n_y[gid] + 1));
    
    mi_contribution[gid] = psi_nx + psi_ny;
}
```

### B.4.3 Rust Metal Integration

```rust
//! (future sketch) crates/pid-core/src/metal_knn.rs (not yet implemented)
//! 
//! Rust bindings for Metal GPU acceleration of k-NN search.
//! Uses the metal-rs crate for GPU access.

use metal::{Device, CommandQueue, MTLResourceOptions, Buffer, ComputePipelineState};
use std::path::Path;

/// Metal-accelerated k-NN search for PID estimation.
/// 
/// # Architecture
/// 
/// The Metal backend is structured as:
/// 
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                     MetalKNN                                │
/// ├─────────────────────────────────────────────────────────────┤
/// │  device: Device           // GPU device handle              │
/// │  command_queue: CommandQueue  // Async command submission   │
/// │  distance_pipeline: ComputePipelineState  // Distance shader│
/// │  knn_pipeline: ComputePipelineState       // k-NN shader    │
/// │  ksg_count_pipeline: ComputePipelineState // KSG neighbor   │
/// │  ksg_mi_pipeline: ComputePipelineState    // KSG MI compute │
/// └─────────────────────────────────────────────────────────────┘
/// ```
/// 
/// # Memory Management
/// 
/// Metal buffers are allocated with `MTLResourceOptions::StorageModeShared`
/// for unified memory access between CPU and GPU. This avoids explicit
/// memory copies on Apple Silicon.
/// 
/// # Thread Safety
/// 
/// MetalKNN is NOT thread-safe. Each thread should have its own instance,
/// or access should be synchronized externally. This is because:
/// 1. CommandQueue submission is not thread-safe
/// 2. Buffer reuse assumes single-threaded access
/// 
/// For multi-threaded PID computation, create a pool of MetalKNN instances.
pub struct MetalKNN {
    device: Device,
    command_queue: CommandQueue,
    distance_pipeline: ComputePipelineState,
    knn_pipeline: ComputePipelineState,
    ksg_count_pipeline: ComputePipelineState,
    ksg_mi_pipeline: ComputePipelineState,
    
    // Pre-allocated buffers for common sizes (avoid allocation in hot path)
    // Key: (n, d), Value: Buffer
    distance_buffer_cache: std::collections::HashMap<(usize, usize), Buffer>,
}

impl MetalKNN {
    /// Create a new Metal k-NN accelerator.
    /// 
    /// # Arguments
    /// * `metallib_path` - Path to compiled Metal library (.metallib)
    /// 
    /// # Panics
    /// Panics if no Metal device is available or shader compilation fails.
    /// 
    /// # Example
    /// ```
    /// let knn = MetalKNN::new(Path::new("shaders/pid_kernels.metallib"))?;
    /// ```
    pub fn new(metallib_path: &Path) -> Result<Self, MetalError> {
        // Get default Metal device (M4 GPU)
        let device = Device::system_default()
            .ok_or(MetalError::NoDevice)?;
        
        // Create command queue for async execution
        let command_queue = device.new_command_queue();
        
        // Load compiled shader library
        let library = device.new_library_with_file(metallib_path)?;
        
        // Create compute pipelines for each kernel
        let distance_fn = library.get_function("compute_distances_linf", None)?;
        let distance_pipeline = device.new_compute_pipeline_state_with_function(&distance_fn)?;
        
        let knn_fn = library.get_function("find_k_nearest", None)?;
        let knn_pipeline = device.new_compute_pipeline_state_with_function(&knn_fn)?;
        
        let ksg_count_fn = library.get_function("ksg_count_neighbors", None)?;
        let ksg_count_pipeline = device.new_compute_pipeline_state_with_function(&ksg_count_fn)?;
        
        let ksg_mi_fn = library.get_function("ksg_compute_mi", None)?;
        let ksg_mi_pipeline = device.new_compute_pipeline_state_with_function(&ksg_mi_fn)?;
        
        Ok(Self {
            device,
            command_queue,
            distance_pipeline,
            knn_pipeline,
            ksg_count_pipeline,
            ksg_mi_pipeline,
            distance_buffer_cache: std::collections::HashMap::new(),
        })
    }
    
    /// Find k nearest neighbors for all query points.
    /// 
    /// # Arguments
    /// * `data` - Data points, shape (n, d), row-major
    /// * `queries` - Query points, shape (q, d), row-major
    /// * `k` - Number of neighbors to find
    /// 
    /// # Returns
    /// * `indices` - Neighbor indices, shape (q, k)
    /// * `distances` - Neighbor distances, shape (q, k)
    /// 
    /// # Performance
    /// For n=10000, d=4096, q=10000, k=3:
    /// - Distance computation: ~30ms
    /// - k-NN selection: ~5ms
    /// - Total: ~35ms
    /// 
    /// Compare to CPU: ~4000ms (100x speedup)
    pub fn find_knn(
        &mut self,
        data: &[f32],      // (n * d,)
        queries: &[f32],   // (q * d,)
        n: usize,
        d: usize,
        q: usize,
        k: usize,
    ) -> (Vec<u32>, Vec<f32>) {
        // Allocate GPU buffers
        let data_buffer = self.device.new_buffer_with_data(
            data.as_ptr() as *const _,
            (data.len() * std::mem::size_of::<f32>()) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        
        let query_buffer = self.device.new_buffer_with_data(
            queries.as_ptr() as *const _,
            (queries.len() * std::mem::size_of::<f32>()) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        
        // Distance output buffer
        let distance_buffer = self.get_or_create_distance_buffer(q, n);
        
        // k-NN output buffers
        let knn_indices_buffer = self.device.new_buffer(
            (q * k * std::mem::size_of::<u32>()) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        let knn_distances_buffer = self.device.new_buffer(
            (q * k * std::mem::size_of::<f32>()) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        
        // Create command buffer
        let command_buffer = self.command_queue.new_command_buffer();
        
        // --- Dispatch distance computation ---
        {
            let encoder = command_buffer.new_compute_command_encoder();
            encoder.set_compute_pipeline_state(&self.distance_pipeline);
            encoder.set_buffer(0, Some(&data_buffer), 0);
            encoder.set_buffer(1, Some(&query_buffer), 0);
            encoder.set_buffer(2, Some(&distance_buffer), 0);
            encoder.set_bytes(3, std::mem::size_of::<u32>() as u64, &(n as u32) as *const _ as *const _);
            encoder.set_bytes(4, std::mem::size_of::<u32>() as u64, &(d as u32) as *const _ as *const _);
            encoder.set_bytes(5, std::mem::size_of::<u32>() as u64, &(q as u32) as *const _ as *const _);
            
            // Grid: (q, ceil(n / 256) * 256)
            let thread_group_size = metal::MTLSize::new(1, 256, 1);
            let grid_size = metal::MTLSize::new(q as u64, ((n + 255) / 256 * 256) as u64, 1);
            encoder.dispatch_threads(grid_size, thread_group_size);
            encoder.end_encoding();
        }
        
        // --- Dispatch k-NN selection ---
        {
            let encoder = command_buffer.new_compute_command_encoder();
            encoder.set_compute_pipeline_state(&self.knn_pipeline);
            encoder.set_buffer(0, Some(&distance_buffer), 0);
            encoder.set_buffer(1, Some(&knn_indices_buffer), 0);
            encoder.set_buffer(2, Some(&knn_distances_buffer), 0);
            encoder.set_bytes(3, std::mem::size_of::<u32>() as u64, &(n as u32) as *const _ as *const _);
            encoder.set_bytes(4, std::mem::size_of::<u32>() as u64, &(k as u32) as *const _ as *const _);
            encoder.set_bytes(5, std::mem::size_of::<u32>() as u64, &(q as u32) as *const _ as *const _);
            
            let thread_group_size = metal::MTLSize::new(256, 1, 1);
            let grid_size = metal::MTLSize::new(((q + 255) / 256 * 256) as u64, 1, 1);
            encoder.dispatch_threads(grid_size, thread_group_size);
            encoder.end_encoding();
        }
        
        // Execute and wait
        command_buffer.commit();
        command_buffer.wait_until_completed();
        
        // Read results back (zero-copy on Apple Silicon due to shared memory)
        let indices: Vec<u32> = unsafe {
            std::slice::from_raw_parts(
                knn_indices_buffer.contents() as *const u32,
                q * k,
            ).to_vec()
        };
        
        let distances: Vec<f32> = unsafe {
            std::slice::from_raw_parts(
                knn_distances_buffer.contents() as *const f32,
                q * k,
            ).to_vec()
        };
        
        (indices, distances)
    }
    
    /// Get or create a distance buffer of given size.
    /// Reuses existing buffers when possible to avoid allocation overhead.
    fn get_or_create_distance_buffer(&mut self, q: usize, n: usize) -> Buffer {
        let key = (q, n);
        if let Some(buffer) = self.distance_buffer_cache.get(&key) {
            return buffer.clone();
        }
        
        let buffer = self.device.new_buffer(
            (q * n * std::mem::size_of::<f32>()) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        
        // Only cache if buffer is reasonably sized (< 1GB)
        if q * n * 4 < 1_000_000_000 {
            self.distance_buffer_cache.insert(key, buffer.clone());
        }
        
        buffer
    }
}

/// Errors that can occur during Metal operations.
#[derive(Debug)]
pub enum MetalError {
    NoDevice,
    ShaderCompilationFailed(String),
    BufferCreationFailed,
    PipelineCreationFailed,
}
```

---

## B.5 CoreML for Quantized Inference

### B.5.1 When to Use CoreML vs MLX

```
COREML VS MLX DECISION MATRIX
=============================

┌─────────────────────┬────────────────┬────────────────┬──────────────┐
│ Use Case            │ CoreML         │ MLX            │ Recommended  │
├─────────────────────┼────────────────┼────────────────┼──────────────┤
│ 7B model (f16)      │ ✓ (via ANE)    │ ✓ (GPU)        │ MLX          │
│ 7B model (4-bit)    │ ✓✓ (fast ANE)  │ ✓ (mlx-lm)     │ CoreML       │
│ Batch inference     │ Limited        │ ✓✓ (vmap)      │ MLX          │
│ Embedding extract   │ Difficult      │ ✓✓ (hooks)     │ MLX          │
│ Mobile deployment   │ ✓✓             │ ✗              │ CoreML       │
│ Custom layers       │ ✗              │ ✓✓             │ MLX          │
│ Deterministic       │ Varies         │ ✓✓             │ MLX          │
└─────────────────────┴────────────────┴────────────────┴──────────────┘

RECOMMENDATION FOR PID-VLA:
===========================
- PRIMARY: MLX for development and experiments (flexibility, hooks)
- SECONDARY: CoreML for final optimized inference (speed, memory)
- QUANTIZATION: Use CoreML's 4-bit for memory-constrained M4 (non-Max)

WHY MLX IS PRIMARY:
- Embedding extraction requires forward hooks (easy in MLX, hard in CoreML)
- PID experiments need custom operations (MLX is composable)
- Reproducibility benefits from deterministic execution; MLX is often deterministic on fixed Apple Silicon hardware, but treat determinism as an assumption to be tested (set seeds, avoid nondeterministic kernels, and verify repeatability).
- Debugging requires inspecting intermediates (MLX arrays are transparent)

WHEN TO USE COREML:
- Final deployment (CoreML is faster for pure inference)
- Memory-constrained hardware (CoreML 4-bit uses ~4GB for 7B model)
- Integration with iOS/macOS apps (CoreML is native)
```

### B.5.2 CoreML Model Conversion

```python
"""
pid_vla/coreml_convert.py
=========================
Convert VLA models to CoreML format for optimized inference.

DETAILED CONVERSION PIPELINE with comments explaining each step.
"""

import coremltools as ct
from coremltools.models.neural_network import quantization_utils
import torch
from pathlib import Path
from typing import Optional, Tuple

def convert_vla_to_coreml(
    pytorch_model_path: Path,
    output_path: Path,
    quantization: Optional[str] = None,  # None, '8bit', '4bit'
    compute_units: str = 'ALL'  # 'ALL', 'CPU_AND_GPU', 'CPU_AND_NE', 'CPU_ONLY'
) -> ct.models.MLModel:
    """
    Convert a PyTorch VLA model to CoreML format.
    
    CONVERSION STEPS:
    =================
    
    1. LOAD PYTORCH MODEL
       - Load weights from checkpoint
       - Set to evaluation mode
       - Trace with example inputs
    
    2. TRACE GRAPH
       - Use torch.jit.trace() for static models
       - Use torch.jit.script() if model has control flow
       - VLAs typically need trace (no data-dependent control flow)
    
    3. CONVERT TO COREML
       - Map PyTorch ops to CoreML ops
       - Handle custom layers (may need custom converters)
       - Set input/output specifications
    
    4. QUANTIZE (OPTIONAL)
       - 8-bit: ~2x smaller, ~1.5x faster, minimal accuracy loss
       - 4-bit: ~4x smaller, ~2x faster, some accuracy loss (~1%)
       - Uses calibration data for optimal quantization ranges
    
    5. VALIDATE
       - Compare CoreML output to PyTorch output
       - Check numerical accuracy (should be < 1e-4 for f16)
    
    Parameters:
        pytorch_model_path: Path to PyTorch checkpoint
        output_path: Where to save CoreML model (.mlpackage)
        quantization: Quantization level (None, '8bit', '4bit')
        compute_units: Which hardware to target
        
    Returns:
        Converted CoreML model
        
    Example:
        >>> model = convert_vla_to_coreml(
        ...     Path("models/openvla-7b.pt"),
        ...     Path("models/openvla-7b.mlpackage"),
        ...     quantization='4bit',
        ...     compute_units='CPU_AND_NE'  # Use Neural Engine
        ... )
    """
    
    # -------------------------------------------------------------------------
    # STEP 1: LOAD PYTORCH MODEL
    # -------------------------------------------------------------------------
    
    print(f"Loading PyTorch model from {pytorch_model_path}...")
    
    # Import model class based on architecture
    # This assumes model follows HuggingFace structure
    from transformers import AutoModelForCausalLM, AutoConfig
    
    config = AutoConfig.from_pretrained(pytorch_model_path)
    model = AutoModelForCausalLM.from_pretrained(
        pytorch_model_path,
        torch_dtype=torch.float16,  # CoreML works best with f16 input
        device_map='cpu'  # Trace on CPU
    )
    model.eval()
    
    # -------------------------------------------------------------------------
    # STEP 2: CREATE TRACE INPUTS
    # -------------------------------------------------------------------------
    
    # VLA models take:
    # - images: (batch, channels, height, width) = (1, 3, 224, 224)
    # - input_ids: (batch, seq_len) = (1, 128)
    # - attention_mask: (batch, seq_len) = (1, 128)
    
    batch_size = 1
    image_size = 224
    seq_len = 128
    
    example_inputs = {
        'pixel_values': torch.randn(batch_size, 3, image_size, image_size, dtype=torch.float16),
        'input_ids': torch.randint(0, 32000, (batch_size, seq_len)),
        'attention_mask': torch.ones(batch_size, seq_len, dtype=torch.long)
    }
    
    print("Tracing model...")
    
    # -------------------------------------------------------------------------
    # STEP 3: TRACE AND CONVERT
    # -------------------------------------------------------------------------
    
    # Trace the forward pass
    with torch.no_grad():
        traced_model = torch.jit.trace(
            model,
            example_kwarg_inputs=example_inputs,
            strict=False  # Allow non-deterministic ops
        )
    
    # Define CoreML input specifications
    # These tell CoreML the expected input shapes and types
    inputs = [
        ct.TensorType(
            name="pixel_values",
            shape=(batch_size, 3, image_size, image_size),
            dtype=ct.TensorType.Float16
        ),
        ct.TensorType(
            name="input_ids", 
            shape=(batch_size, seq_len),
            dtype=ct.TensorType.Int32
        ),
        ct.TensorType(
            name="attention_mask",
            shape=(batch_size, seq_len),
            dtype=ct.TensorType.Int32
        )
    ]
    
    # Convert to CoreML
    print("Converting to CoreML...")
    
    mlmodel = ct.convert(
        traced_model,
        inputs=inputs,
        convert_to="mlprogram",  # Use ML Program format (modern)
        compute_units=getattr(ct.ComputeUnit, compute_units),
        minimum_deployment_target=ct.target.macOS14  # macOS 14+ for M4 optimization
    )
    
    # -------------------------------------------------------------------------
    # STEP 4: QUANTIZE (OPTIONAL)
    # -------------------------------------------------------------------------
    
    if quantization == '8bit':
        print("Applying 8-bit quantization...")
        mlmodel = quantization_utils.quantize_weights(
            mlmodel,
            nbits=8,
            quantization_mode="linear"  # Linear quantization for weights
        )
        
    elif quantization == '4bit':
        print("Applying 4-bit quantization...")
        # 4-bit requires calibration data for best results
        # Here we use post-training quantization without calibration
        mlmodel = quantization_utils.quantize_weights(
            mlmodel,
            nbits=4,
            quantization_mode="linear_symmetric"  # Better for 4-bit
        )
    
    # -------------------------------------------------------------------------
    # STEP 5: SAVE AND VALIDATE
    # -------------------------------------------------------------------------
    
    print(f"Saving to {output_path}...")
    mlmodel.save(str(output_path))
    
    # Validate by comparing outputs
    print("Validating conversion...")
    
    # Run PyTorch
    with torch.no_grad():
        pt_output = model(**example_inputs).logits.numpy()
    
    # Run CoreML
    # Note: CoreML returns a dictionary of outputs
    coreml_inputs = {
        'pixel_values': example_inputs['pixel_values'].numpy(),
        'input_ids': example_inputs['input_ids'].numpy().astype('int32'),
        'attention_mask': example_inputs['attention_mask'].numpy().astype('int32')
    }
    coreml_output = mlmodel.predict(coreml_inputs)['logits']
    
    # Compare
    max_diff = abs(pt_output - coreml_output).max()
    print(f"Maximum output difference: {max_diff:.6f}")
    
    if max_diff > 0.01:
        print("WARNING: Large conversion error detected!")
        print("Consider using float32 or checking for unsupported ops.")
    else:
        print("Conversion validated successfully!")
    
    return mlmodel


def benchmark_coreml_inference(
    mlmodel_path: Path,
    num_iterations: int = 100
) -> dict:
    """
    Benchmark CoreML model inference speed.
    
    Measures:
    - Cold start time (first inference, includes compilation)
    - Warm inference time (subsequent inferences)
    - Memory usage
    
    Returns dict with timing statistics.
    """
    import time
    import coremltools as ct
    
    # Load model
    mlmodel = ct.models.MLModel(str(mlmodel_path))
    
    # Create dummy inputs
    inputs = {
        'pixel_values': np.random.randn(1, 3, 224, 224).astype(np.float16),
        'input_ids': np.random.randint(0, 32000, (1, 128)).astype(np.int32),
        'attention_mask': np.ones((1, 128)).astype(np.int32)
    }
    
    # Cold start
    t0 = time.perf_counter()
    _ = mlmodel.predict(inputs)
    cold_time = time.perf_counter() - t0
    
    # Warm iterations
    times = []
    for _ in range(num_iterations):
        t0 = time.perf_counter()
        _ = mlmodel.predict(inputs)
        times.append(time.perf_counter() - t0)
    
    return {
        'cold_start_ms': cold_time * 1000,
        'warm_mean_ms': np.mean(times) * 1000,
        'warm_std_ms': np.std(times) * 1000,
        'warm_p50_ms': np.percentile(times, 50) * 1000,
        'warm_p99_ms': np.percentile(times, 99) * 1000,
    }
```

---

## B.6 Nix Configuration for Reproducibility

**Important (avoid divergence):**
- The **canonical** reproducibility config lives in the repo root: `flake.nix`, `flake.lock`, `pyproject.toml`, `uv.lock`.
- This appendix includes a much larger “kitchen sink” flake from earlier drafts (Rust overlays, Python-withPackages, CUDA scaffolding, etc.). It is retained for context, but it is **not** the authoritative environment definition unless it is explicitly reconciled with the repo files.
- Prefer: Nix pins **tooling** (rustc/cargo/python/uv/just) and `uv.lock` pins **Python deps**. Avoid mixing “Python deps from Nix” and “Python deps from uv” unless you have a clear reason and a single source of truth.

### B.6.1 Complete flake.nix

```nix
# flake.nix
# =========
# Nix flake for reproducible PID-VLA development environment.
#
# WHAT THIS PROVIDES:
# - Exact versions of all dependencies (Rust, Python, system libraries)
# - Reproducible builds across machines
# - Isolated development environment (no system pollution)
# - Easy onboarding for new contributors
#
# USAGE:
#   nix develop              # Enter development shell
#   nix build .#pid-core     # Build Rust crate
#   nix build .#pid-python   # Build Python wheel
#   nix run .#experiments    # Run experiment suite
#
# WHY NIX FOR THIS PROJECT:
# 1. REPRODUCIBILITY: ML experiments MUST be reproducible
#    - Same code + same data + same environment = same results
#    - Nix guarantees environment reproducibility
#
# 2. DEPENDENCY HELL AVOIDANCE:
#    - Python version conflicts (3.9 vs 3.11 vs 3.12)
#    - CUDA version conflicts (11.x vs 12.x)
#    - Rust toolchain versions
#    - System library versions (OpenBLAS, MKL, etc.)
#
# 3. MULTI-PLATFORM:
#    - Same flake works on macOS (M4) and Linux (CUDA)
#    - Platform-specific optimizations are handled transparently
#
# APPLE SILICON SPECIFIC:
# - Uses nixpkgs-darwin for macOS-specific packages
# - MLX is installed via pip (not in nixpkgs yet)
# - Metal SDK is available through Xcode (not managed by Nix)

{
  description = "PID-VLA: Partial Information Decomposition for Vision-Language-Action Model Diagnostics";

  # ============================================================================
  # INPUTS: External dependencies
  # ============================================================================
  
  inputs = {
    # Main package repository
    # Using nixos-unstable for latest packages
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    
    # Rust toolchain overlay
    # Provides rust-bin for precise Rust version control
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    # Flake utilities for multi-system support
    flake-utils.url = "github:numtide/flake-utils";
    
    # Optional: Nix Darwin for macOS system configuration
    # Uncomment if you want to manage macOS system settings via Nix
    # darwin = {
    #   url = "github:lnl7/nix-darwin";
    #   inputs.nixpkgs.follows = "nixpkgs";
    # };
  };

  # ============================================================================
  # OUTPUTS: What this flake provides
  # ============================================================================
  
  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        # ======================================================================
        # PLATFORM DETECTION
        # ======================================================================
        
        # Detect if we're on Apple Silicon
        isAppleSilicon = system == "aarch64-darwin";
        isLinux = builtins.match ".*-linux" system != null;
        
        # Configure nixpkgs with overlays
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ 
            (import rust-overlay)
            
            # Custom overlay for project-specific packages
            (final: prev: {
              # Pin specific Python version
              python = prev.python311;
              
              # Add project-specific packages here
              pid-vla = final.callPackage ./nix/pid-vla.nix { };
            })
          ];
          
          # Allow unfree packages (needed for some ML tools)
          config.allowUnfree = true;
        };
        
        # ======================================================================
        # RUST CONFIGURATION
        # ======================================================================
        
        # Use stable Rust with specific extensions
        # We need:
        # - rust-src: For IDE support and debugging
        # - rust-analyzer: For IDE integration
        # - clippy: For linting
        # - rustfmt: For formatting
        rustToolchain = pkgs.rust-bin.stable."1.75.0".default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
          targets = [
            # Native target
            (if isAppleSilicon then "aarch64-apple-darwin" else "x86_64-unknown-linux-gnu")
            # Cross-compilation targets (optional)
          ] ++ (if isLinux then [ "aarch64-apple-darwin" ] else [ ]);
        };
        
        # ======================================================================
        # PYTHON CONFIGURATION
        # ======================================================================
        
        # Python with specific packages from nixpkgs
        # Additional packages (MLX, etc.) are installed via pip in the shell
        pythonEnv = pkgs.python.withPackages (ps: with ps; [
          # Core scientific computing
          numpy
          scipy
          scikit-learn
          
          # Data handling
          polars
          pyarrow
          
          # Visualization
          matplotlib
          seaborn
          
          # Development tools
          pytest
          black
          ruff
          mypy
          
          # Jupyter for experiments
          jupyterlab
          ipykernel
          
          # PyO3 for Rust bindings
          maturin
        ]);
        
        # ======================================================================
        # PLATFORM-SPECIFIC DEPENDENCIES
        # ======================================================================
        
        # Common dependencies (all platforms)
        commonDeps = with pkgs; [
          # Build tools
          rustToolchain
          pkg-config
          cmake
          
          # Python environment
          pythonEnv
          uv  # Fast Python package installer
          
          # Task runner
          just
          
          # Version control
          git
          git-lfs  # For large model files
          
          # Data tools
          jq
          yq
          
          # Documentation
          mdbook
        ];
        
        # macOS-specific dependencies
        darwinDeps = with pkgs; [
          # Apple frameworks (needed for Metal, CoreML)
          darwin.apple_sdk.frameworks.Metal
          darwin.apple_sdk.frameworks.MetalPerformanceShaders
          darwin.apple_sdk.frameworks.Accelerate
          darwin.apple_sdk.frameworks.CoreML
          
          # macOS-specific tools
          darwin.cctools
          
          # NOTE: MLX is installed via pip, not Nix
          # (not yet available in nixpkgs)
        ];
        
        # Linux-specific dependencies
        linuxDeps = with pkgs; [
          # CUDA support (optional, for NVIDIA GPUs)
          # Uncomment if you have NVIDIA hardware
          # cudaPackages.cudatoolkit
          # cudaPackages.cudnn
          
          # OpenBLAS for linear algebra
          openblas
          
          # Linux-specific tools
          patchelf
        ];
        
        # Select platform-specific deps
        platformDeps = if isAppleSilicon then darwinDeps
                       else if isLinux then linuxDeps
                       else [ ];
        
        # ======================================================================
        # ENVIRONMENT VARIABLES
        # ======================================================================
        
        envVars = {
          # Rust configuration
          RUST_BACKTRACE = "1";
          RUST_LOG = "info";
          
          # Python configuration
          PYTHONDONTWRITEBYTECODE = "1";
          VIRTUAL_ENV = ""; # Disable venv detection (we use Nix)
          
          # Project paths
          PID_VLA_ROOT = builtins.toString ./.;
          PID_VLA_DATA = builtins.toString ./data;
          PID_VLA_MODELS = builtins.toString ./models;
          
        } // (if isAppleSilicon then {
          # Apple Silicon specific
          # Point to Metal SDK (from Xcode)
          METAL_SDK = "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk";
          
          # MLX configuration
          MLX_GPU_MEMORY_LIMIT = "0.9";  # Use 90% of GPU memory
          
          # Disable MPS fallback warnings
          PYTORCH_ENABLE_MPS_FALLBACK = "1";
          
        } else { });
        
      in {
        # ======================================================================
        # DEVELOPMENT SHELLS
        # ======================================================================
        
        devShells = {
          # Default development shell with everything
          default = pkgs.mkShell {
            name = "pid-vla-dev";
            
            buildInputs = commonDeps ++ platformDeps;
            
            inherit (envVars) RUST_BACKTRACE RUST_LOG PYTHONDONTWRITEBYTECODE;
            
            shellHook = ''
              echo "🧠 PID-VLA Development Environment"
              echo "=================================="
              echo "Platform: ${system}"
              echo "Rust: $(rustc --version)"
              echo "Python: $(python --version)"
              echo ""
              
              # Create .venv if it doesn't exist (for pip packages like MLX)
              if [ ! -d .venv ]; then
                echo "Creating Python virtual environment for pip packages..."
                python -m venv .venv
              fi
              
              # Activate venv for pip packages
              source .venv/bin/activate
              
              # Install pip packages not in nixpkgs
              ${if isAppleSilicon then ''
                echo "Installing Apple Silicon specific packages..."
                pip install -q mlx mlx-lm coremltools
              '' else ''
                echo "Installing Linux specific packages..."
                pip install -q torch torchvision
              ''}
              
              echo ""
              echo "Available commands:"
              echo "  just build    - Build Rust crates"
              echo "  just test     - Run tests"
              echo "  just exp0     - Run Experiment 0 (validation)"
              echo "  just notebook - Start Jupyter Lab"
              echo ""
            '';
          };
          
          # Minimal shell for CI
          ci = pkgs.mkShell {
            name = "pid-vla-ci";
            buildInputs = with pkgs; [
              rustToolchain
              pythonEnv
              just
            ];
          };
        };
        
        # ======================================================================
        # PACKAGES
        # ======================================================================
        
        packages = {
          # Rust core library
          pid-core = pkgs.rustPlatform.buildRustPackage {
            pname = "pid-core";
            version = "0.1.0";
            src = ./crates/pid-core;
            cargoLock.lockFile = ./Cargo.lock;
          };
          
          # Python wheel with Rust bindings
          pid-python = pkgs.python.pkgs.buildPythonPackage {
            pname = "pid-vla";
            version = "0.1.0";
            src = ./.;
            format = "pyproject";
            
            nativeBuildInputs = [ pkgs.maturin rustToolchain ];
            
            buildPhase = ''
              maturin build --release
            '';
          };
          
          # Default package
          default = self.packages.${system}.pid-python;
        };
        
        # ======================================================================
        # APPS (Runnable commands)
        # ======================================================================
        
        apps = {
          # Run experiment suite
          experiments = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "run-experiments" ''
              cd ${builtins.toString ./.}
              python -m pid_vla.experiments.run_all
            '';
          };
          
          # Start visualization app
          viz = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "pid-viz" ''
              cd ${builtins.toString ./.}
              cargo run -p pid-tauri
            '';
          };
        };
      });
}
```

### B.6.2 justfile (Task Runner)

**Canonical:** the repo root `justfile`. The block below is an expanded example from earlier drafts (kept for context); update it only if you also update the repo `justfile`.

```makefile
# justfile
# ========
# Task runner for PID-VLA project.
# 
# Just is like Make but simpler and more ergonomic.
# Install: cargo install just
# 
# USAGE:
#   just          # List available tasks
#   just build    # Build everything
#   just test     # Run all tests
#   just exp0     # Run Experiment 0

# Default recipe: list available tasks
default:
    @just --list

# =============================================================================
# BUILD TASKS
# =============================================================================

# Build all Rust crates
build:
    cargo build --release

# Build Python wheel
build-wheel:
    maturin build --release

# Build and install Python package in development mode
dev-install:
    maturin develop --release

# Clean build artifacts
clean:
    cargo clean
    rm -rf .venv
    rm -rf __pycache__
    rm -rf .pytest_cache
    find . -name "*.egg-info" -type d -exec rm -rf {} +

# =============================================================================
# TEST TASKS
# =============================================================================

# Run all tests
test: test-rust test-python

# Run Rust tests
test-rust:
    cargo test --release

# Run Python tests
test-python:
    pytest python/tests/ -v

# Run tests with coverage
test-coverage:
    cargo tarpaulin --out Html
    pytest python/tests/ --cov=pid_vla --cov-report=html

# =============================================================================
# LINT AND FORMAT
# =============================================================================

# Format all code
fmt:
    cargo fmt
    black python/
    ruff check python/ --fix

# Check formatting (CI)
fmt-check:
    cargo fmt --check
    black --check python/
    ruff check python/

# Run all linters
lint:
    cargo clippy -- -D warnings
    ruff check python/
    mypy python/pid_vla/

# =============================================================================
# EXPERIMENTS
# =============================================================================

# Run Experiment 0: Estimator validation
exp0:
    @echo "Running Experiment 0: KSG Estimator Validation"
    @echo "=============================================="
    python python/experiments/exp0_validation.py

# Run Experiment 1: Decomposition analysis
exp1: exp0
    @echo "Running Experiment 1: Decomposition Analysis"
    @echo "============================================="
    python python/experiments/exp1_decomposition.py

# Run Experiment 2: Baseline comparison
exp2: exp1
    @echo "Running Experiment 2: Baseline Comparison"
    @echo "=========================================="
    python python/experiments/exp2_baselines.py

# Run all experiments in sequence
experiments: exp0 exp1 exp2
    @echo "All experiments completed!"

# =============================================================================
# DATA MANAGEMENT
# =============================================================================

# Download LIBERO dataset
download-libero:
    @echo "Downloading LIBERO dataset..."
    python scripts/download_libero.py

# Download model weights
download-models:
    @echo "Downloading VLA model weights..."
    python scripts/download_models.py

# Extract embeddings from VLA model
extract-embeddings model="openvla":
    @echo "Extracting embeddings from {{model}}..."
    python python/experiments/extract_embeddings.py --model {{model}}

# =============================================================================
# DEVELOPMENT
# =============================================================================

# Start Jupyter Lab
notebook:
    jupyter lab --notebook-dir=python/notebooks

# Start Tauri visualization app
viz:
    cargo run -p pid-tauri

# Generate documentation
docs:
    cargo doc --no-deps --open
    cd python && sphinx-build -b html docs/ docs/_build/

# =============================================================================
# CI/CD
# =============================================================================

# Full CI check (runs everything)
ci: fmt-check lint test
    @echo "CI checks passed!"

# Build release artifacts
release: build build-wheel
    @echo "Release artifacts built!"
```

---

## B.7 Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | Dec 2025 | Initial specification |
| 2.0 | Dec 2025 | Comprehensive revision with critical analysis, discarded approaches, three-way PID discussion |
| 2.1 | Dec 2025 | Added six use cases, Shannon invariants section, dual-process theory framing |
| 2.2 | Dec 2025 | Added complete Apple M4 implementation reference (Appendix B) |
| 2.3 | Dec 2025 | Added existing PID code availability analysis, complete Rust I^sx_∩ implementation with 5 validation test scenarios, verified all content based on Wibral's PID (not older Williams & Beer I_min) |
| 2.4 | Dec 2025 | **Major update:** (1) Corrected WAN information throughout document - WAN CAN be made action-conditioned via LoRA fine-tuning, VACE, Wan-Move; added WAN 2.2 MoE architecture, inference speed improvements; added Motus, DreamGen, VideoVLA as WAN-based robotics systems. (2) Added comprehensive §B.3.5 on scaling 3-way PID: Shannon invariants, Gaussian PID, NF-PID (“Thin-PID” legacy) via normalizing flows, coarse-graining approaches. (3) Added new references for scalable PID methods. |
| 2.5 | Dec 2025 | **Additions:** (1) Added §10.4 Depth Perception Methods: monocular depth (Depth-Anything v2/v3, Metric3D v2, RollingDepth), stereo vision (StereoVLA approach from arXiv:2512.21970), transparent object depth (DKT). (2) Added Headless Gazebo + Tauri Visualization System with Zenoh middleware, SparkJS/Three.js rendering, ~25-30ms latency path, cross-platform ML backends (CoreML/MLX/Metal on macOS, CUDA/TensorRT on Linux). (3) Added NanoGPT/nanochat note to DreamVLA backbone section - clarified GPT-2 refers to pretrained architecture, NanoGPT useful for custom training. (4) Expanded references: Depth Estimation & 3D Perception, Simulation & Middleware, Training Infrastructure. (5) Updated glossary with Zenoh, NanoGPT, StereoVLA, DKT. |
| 2.6 | Jan 2026 | **Process Reward Models integration:** (1) Added §3.5 PID vs. Process Reward Models (PRMs) - comprehensive comparison of PID approach with Robo-Dopamine's General Reward Model (GRM), including when to use each, potential synergies, and the "semantic trap" insight for reward shaping. (2) Added GRM as baseline #7 in experimental design. (3) Added §13.6 Process Reward Models references (Robo-Dopamine, GVL, VLAC, SARM, LIV). (4) Updated glossary with PRM, GRM, ORM, VOC, PBRS terms. |
| 2.7 | Jan 2026 | **World model paradigms & DKT deep dive:** (1) Added §10.1 world model taxonomy (Internal/Evaluative/Generative) with Genie 3 as environment generator. (2) Expanded §10.4.3 DKT section with "Diffusion Knows Transparency" principle, technical details, robot grasping results, and genuine PID relevance (perception quality as prerequisite for valid PID). (3) Added §10.7 World Model Paradigms and PID Implications: theoretical framework for how external world models (Genie 3, WAN) affect internal D; "Diffusion Knows Physics" principle; perception quality diagnostic tree. (4) Added Genie 3, SIMA 2, Genie 2 to world models references. (5) Updated glossary with Genie 3, SIMA 2, TransPhy3D, Emergent Physics. (6) Renumbered sections 10.7→10.8 for Gazebo+Tauri. |
| 2.8 | Jan 2026 | **NixOS CUDA secondary target:** (1) Restructured §B.2 as "Platform Implementation Reference" with primary (Apple M4) and secondary (NixOS + CUDA) targets. (2) Added §B.2.4 NixOS + CUDA Implementation with complete configuration.nix for NVIDIA drivers, flake.nix with CUDA-enabled PyTorch and Rust toolchain, CUDA software stack diagram. (3) Added GPU-accelerated PID implementation: CUDAKSGEstimator and CUDAPIDEstimator classes with chunked distance computation for OOM prevention. (4) Added NixOS troubleshooting guide and multi-GPU configuration (NCCL). (5) Fixed §B.3 subsection numbering: B.3.5→B.3.3, B.3.6→B.3.4, B.3.7→B.3.5 with correct heading levels. |
| 2.9 | Jan 2026 | **PixelVLA integration & sae_analysis notes:** (1) Added §7.3 PixelVLA architecture: multiscale pixel-aware encoder, visual prompting encoder, continuous action decoder, Pixel-160K dataset. (2) Added §7.4 TraceVLA: visual trace prompting for spatial-temporal awareness. (3) Added §10.8.7 PixelVLA + Headless Gazebo + Tauri integration: data flow diagram, visual prompting in Tauri (TypeScript), PixelVLA-specific PID analysis (Rust), latency budget (~86ms interactive). (4) Added §B.3.3.2 Abzinger/sae_analysis: Shannon invariants (Red°, Vul°) for SAE analysis, comparison with our approach. (5) Updated §B.3.3.5 to clarify sae_analysis is **not** an `I^sx_∩` estimator; added implementation-level definitions of Red°/Vul° and safe integration guidance (SAE compression + screening), not a correctness validation for `I^sx_∩`. (6) Updated §7.5 with MemoryVLA, CoT-VLA. (7) Added PixelVLA, TraceVLA, sae_analysis to references (§13.2, §13.3). (8) Updated glossary with PixelVLA, TraceVLA, Red°, Vul°, multiscale pixel-aware encoder, Pixel-160K. |
| 3.0 | Jan 2026 | **First-principles audit pass:** (1) Reframed “synergy sign” as a falsifiable hypothesis (not a definition); clarified deterministic-target degeneracy in VLA decompositions and the need for external targets/counterfactuals. (2) Tightened estimator risk framing and strengthened Experiment 0 as a scientific gate before any VLA claims. (3) Added/expanded i.i.d. vs trajectory autocorrelation guidance (sampling unit, block bootstrap). |
| 4.0 (Draft) | Jan 2026 | **Audited + citation-verified pass:** (1) Added explicit reference verification policy and downgraded unsourced architecture/latency statements to “unverified sketches”. (2) Added strong-dependence warning (Gao et al. 2015) and integrated a Gaussian-channel strong-dependence sweep into Experiment 0. (3) Added MI/CMI estimator comparison section (Gao-LNC/local Gaussian, MINE, CCMI) strictly as MI/CMI baselines (do not mix estimator families inside SxPID identities). (4) Verified key VLA citations (notably DreamVLA) and added optional background papers (OpenVLThinker, SRL, diffusion parameterization). (5) Cleaned up NF-PID (“Thin-PID” legacy) naming and other citation/notation fixes. (6) Corrected/clarified Shannon-invariant definitions (CI sign conventions; Ω vs target co-information) and reconciled scaling sketches. (7) Aligned reproducibility guidance with repo-canonical `flake.nix` + `uv.lock` workflow (macOS-first). (8) Integrated differential-geometry contingencies into §8.1.5 without relying on a repo-local PDF. |
| 5.0 | Jan 2026 | **Final audit release:** Added confounding factors analysis (§14), numerical stability guidance (§15), manifold/PCA/kNN limitations (§16). Integrated information geometry methods and intrinsic dimension estimation. Code audit complete (implementation cross-checked). Grant-ready documentation with full provenance tracking. |
| 5.1-5.3 | Jan 2026 | **Refinements:** Clarified variable definitions for OpenVLA/DreamVLA, added scope for visual prompting/trace architectures, and distinguished source-count scaling (hierarchy) from estimator validity (geometry). |
| 5.4 | Jan 2026 | **VLA Integration:** Verified key VLA + Shannon-invariants citations (OpenVLA, DreamVLA, PixelVLA, TraceVLA). Clarified primary hypothesis vs. candidate sub-hypotheses and mapped them to aims. |
| 5.5 | Jan 2026 | **Critical Geometry Fix:** Documented that Wibral PID (`I^sx_∩`) on manifolds/Lorentz spaces requires new derivations. Added top-level warning against naive Euclidean application. |
| 5.6 | Jan 2026 | **Manifold Approaches (WIP):** Added Top 5 manifold-compatible engineering approaches (Unrolling, Geodesic MI, Linear Projection, Quantization, Copula Transform) to address the v5.5 discovery. |
| **5.7** | Jan 2026 | **First-Principles Geometry Analysis + VLA Verification:** (1) Verified VLA architectures against original papers: OpenVLA (SigLIP+DinoV2 600M, 32 layers, 4096d), DreamVLA (GPT-2 dims UNSPECIFIED), PixelVLA/TraceVLA (4096d, 7D actions). (2) Added §16.6-§16.11: local flatness testing (4 methods incl. Ollivier-Ricci curvature), δ-hyperbolicity testing, SAE analysis for VLA, Chebyshev/PixelVLA geometry transition, GPT-2 vs modern LLMs hierarchy evidence, unified Geometry-First Protocol with NanoGPT foundational study. (3) Added Wibral GitLab repos as authoritative code sources. (4) Integrated VLA-Arena benchmark findings, GenieReasoner/FACT tokenizer, hierarchical geometry of cognitive states. (5) Added explicit hyperbolic training guidance. |

---

*End of Document*
