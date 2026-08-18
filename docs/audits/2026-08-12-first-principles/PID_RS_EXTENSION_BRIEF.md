# `pid-rs` extension brief for Prisoma

Review completed: 2026-08-14. Literature cutoff: 2026-08-13.

Prisoma pin: `796c11e70f009634b853dc4ada6f565563d82f51`.

Public `pid-rs` head observed during this review:
`bc3aa80fb6025e709c2906a08bce25a4fac40578`.

Latest reviewed estimator-code anchor: `cb3f58f0b190454cb3f1090de8798261ec78f194`.
The later `7473e62..bc3aa80` interval repairs custody and assurance surfaces. It does not change
crate or Cargo inputs and grants no new estimator or scientific credit.

This brief supports upstream design work. It does not authorize a Prisoma pin change or replace the
sole copy-and-send body in [`PID_RS_HANDOFF.md`](PID_RS_HANDOFF.md), lines 424–529.
It does not claim that a proposed method is implemented, validated, or fit for a hypothesis. The
governing Prisoma method and publication rules are in
[`PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md`](../../../PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md).

## Decision

Do not ask `pid-rs` for a generic “Wibral PID.” No such single scientific object exists.

Ask for a small foundation bundle first. Separate contract repair from new scientific input:

1. represent each paper-defined functional and output coordinate separately from its estimator,
   evaluator, transform, and downstream composition;
2. expose a bounded sparse empirical-count-law API for two-source categorical MGW shared
   exclusions;
3. record the relation computed from caller-declared transform-fit and evaluation row identities,
   or record a separate caller assertion;
4. add checked resource-estimate composition for multi-stage callbacks;
5. add an exact fixed-law fixture for the MGW averaged source-law invariance; and
6. separate nominal Monte Carlo p-values from surrogate tail scores at the Rust type boundary.

Bridge the count-law type to the existing exact certifier next. Then add a distinct analytic
rational-law input. Add a declared binary64 finite-law MGW evaluator only after those two paths
agree on bounded fixtures. These are the highest-value next scientific extensions. They make
finite laws inspectable without row expansion or target sampling.

Add generic group-aware schedule infrastructure next. Prisoma can map groups to episodes. Do not
present whole-group replacement bootstrap as a continuous kNN solution: repeated groups duplicate
their numeric coordinates, and occurrence IDs do not remove exact ties. The first continuous route
must use a separately justified without-replacement group subsampling diagnostic, per-group
statistic, or future weighted/cluster-aware estimator. Improve the continuous Ehrlich contract
after that. Keep an infomorphic objective record separate from the PID functional and estimator.
Preserve the BROJA/\(\sim\)-PID, the distinct Gaussian-restricted \(\sim_G\)-PID, the deficiency
\(\delta\)-PID, the Gaussian-channel-restricted \(\delta_G\) quantity, the convex-surrogate
\(\widehat{\delta}_G\)-PID, the \(\delta^\lambda\) family, the information-deficiency I-PID, and the
distinct Lyu–Clark–Raviv Gaussian information hierarchy as research registry nodes; this request
does not make them new runtime modes. The \(\sim\)-PID is the BROJA-PID, but \(\sim_G\) is a
different restricted functional whose equality to BROJA on Gaussian input laws is conjectured.

Do not make BROJA, Williams–Beer `I_min`, a mixed-law proposal, or a failed continuous estimate a
fallback for Prisoma. Do not prioritize GPU or MPS work for the small categorical path.

These requests do not block Prisoma W1 or W2. Those primary world-model claims are prediction and
complete-policy experiments, not PID claims. PID remains a conditional H3 diagnostic. It becomes
interpretable only after its population, measure, estimator, and application gates pass. Upstream
work can improve the estimator gate. It cannot clear the other three gates for Prisoma.

One independent `pid-runlog` request is also useful. Schema 2 has no neutral typed event for a
decision commitment or execution receipt. Prisoma currently carries those two records in
`label_observed` compatibility envelopes. They are not scientific labels. Add a versioned,
content-bound decision-record event before Prisoma migrates. Keep the actual restored-fork outcome
in `label_observed`. This schema request is orthogonal to every PID item below.

## Provenance and estimand graph

Do not emit a `wibral_lineage` result identity. The author name is useful for literature search,
but it does not identify a scientific object. Represent the ecosystem as a provenance and
estimand graph. Functional nodes define named quantities and lattice coordinates. Estimator edges
map observations to an estimate of one named coordinate. Evaluator edges map a declared law to
that coordinate's value. Preprocessing and objective-composition nodes remain separate. Every
result must bind the applicable node and edge identifiers.

The identifiers below are proposed stable semantic IDs, not names assigned by the cited authors.
Keep current `pid-rs` IDs canonical for their existing estimator, pipeline, and validation rows.
They are not aliases for a paper-defined functional. Add typed `defines_quantity`,
`targets_functional`, `evaluates_functional`, `implements_route`, `recovers_on_domain`,
`motivated_by`, `validated_by`, and `composes_quantities` edges. Bind complete author lists,
references, versions, and software revisions in metadata rather than in stable IDs. Reserve
`alias_of` for definitionally identical identities.

The current method-catalog schema cannot represent this whole graph. It has no functional,
declared-law evaluator, reference, theorem, or objective-composition node kinds. Use a separate
scientific-object registry or a method-catalog v2. A minimal Wave 0 can add functional and
quantity-coordinate identities plus typed target edges without blocking later graph work.

| Graph node or edge | Proposed identity | What it is | What it is not | Prisoma use |
|---|---|---|---|---|
| Wibral–Priesemann–Kay–Lizier–Phillips neural goal functions | `reference.neural-goal-coordinates.2017` | A coordinate language for comparing and composing PID terms into neural goals | A PID functional, estimator, or implemented arbitrary learning rule | Design context only |
| Gutknecht–Wibral–Makkeh parthood and logic | `semantics.pid-information-parthood.2021` | A semantic foundation for PID atoms, lattices, and information parthood | A finite-sample estimator | Semantic authority |
| Makkeh–Gutknecht–Wibral categorical shared exclusions | `functional.shared-exclusions.mgw-categorical` | A pointwise functional on categorical probability laws, differentiable on its declared fixed-support domain | Ehrlich continuous PID, a quantizer, or an infomorphic objective | Active low-dimensional categorical diagnostic |
| MGW cumulative lattice quantity | `quantity.shared-exclusions.mgw-categorical.cumulative` | One cumulative shared-exclusion value at an exact antichain coordinate | Its Möbius-inverted atom or a generic redundancy scalar | Required result coordinate |
| MGW Möbius atom quantity | `quantity.shared-exclusions.mgw-categorical.mobius-atom` | One atom obtained by Möbius inversion on the declared lattice | A cumulative value or another lattice coordinate | Required result coordinate |
| Empirical-PMF categorical route | current `shared-exclusions.categorical` targeting `functional.shared-exclusions.mgw-categorical` | A plug-in estimator of the categorical MGW functional when counts represent sampled observations | A second PID definition, a declared-law evaluator, or a population-law certificate | Active descriptive estimator route |
| Schick-Poland et al. measure-theoretic shared exclusions | `functional.shared-exclusions.measure-theoretic-2021` | An arXiv-preprint construction for broad random-variable domains | The later practical Ehrlich formula or its kNN estimator | Literature and theorem boundary only |
| Ehrlich et al. purely continuous analytic shared exclusions | `functional.shared-exclusions.ehrlich-continuous` | A continuous counterpart inspired by categorical shared exclusions, with its own analytic estimand and assumptions | Binned MGW, an unrelated method, or a proved categorical-to-continuous equivalence | Default-off research functional |
| Ehrlich source-disjunction kNN route | current `shared-exclusions.continuous-report` targeting `functional.shared-exclusions.ehrlich-continuous` | A finite-sample estimator that targets the Ehrlich continuous functional | The functional itself or a categorical estimator | Default-off, application-blocked estimator route |
| Williams–Beer categorical redundancy | `functional.pid.williams-beer-imin` | A separate PID comparator based on `I_min` | MGW shared exclusions or a fallback after continuous abstention | Inactive comparator only |
| Lyu–Clark–Raviv conditional-independence hierarchy | `functional.information-hierarchy.lyu-conditional-independence` | Two-source redundancy plus per-source unique information, order-\(K\) synergistic effects, narrow synergy, and total synergistic effect; it deliberately defines no redundancy for \(N\geq3\) | A complete higher-source antichain PID or atoms from another functional | Preserved research identity only |
| Lyu Gaussian covariance-law evaluator | `evaluator.information-hierarchy.lyu-gaussian-covariance-law` | Log-determinant evaluation of the named hierarchy quantities on a positive-definite jointly Gaussian law | A sample estimate or distribution-free formula | Future law-specific sensitivity route |
| Lyu sample-covariance plug-in route | `route.information-hierarchy.lyu-sample-covariance-plugin` | A sample estimator for the named Gaussian hierarchy quantities | The population evaluator, a complete \(N\geq3\) PID, or shared-exclusions validation | Preserved research route only |
| BROJA/\(\sim\)-PID | `functional.pid.broja-bivariate` | The Bertschinger et al. optimization-based bivariate PID; \(\sim\)-PID is another name for this same functional | Deficiency \(\delta\), `I_min`, MMI, shared exclusions, \(\delta^\lambda\), or I-PID | Preserved comparator only |
| Gaussian-restricted \(\sim_G\)-PID | `functional.pid.sim-g-gaussian-restricted` | The BROJA/\(\sim\) optimization with candidate couplings restricted to jointly Gaussian laws; it bounds BROJA in general | BROJA/\(\sim\) itself or an evaluator route; equality for Gaussian input \(P\) is conjectured | Preserved research functional only |
| \(\sim_G\) Gaussian covariance-law evaluator | `evaluator.pid.sim-g-gaussian-covariance-law` | Declared-law optimization targeting the \(\sim_G\)-PID | An evaluator for unrestricted BROJA/\(\sim\), a sample estimate, or proof of the equality conjecture | Future law-specific research route |
| \(\sim_G\) sample plug-in route | `route.pid.sim-g-gaussian-sample-plugin` | Sample-covariance estimation of \(\sim_G\) without the later finite-sample correction | The declared-law evaluator, BROJA/\(\sim\), a Gaussianity test, or the bias-corrected route | Preserved research route only |
| \(\sim_G\) bias-corrected sample route | `route.pid.sim-g-gaussian-sample-bias-corrected-2023` | The paper's distinct finite-sample bias-corrected estimation route targeting \(\sim_G\) | A new functional, exact evaluator, unrestricted BROJA/\(\sim\), or permission to analyze atomic data | Preserved research route only |
| Bivariate deficiency PID | `functional.pid.deficiency-delta-bivariate` | The distinct deficiency-based \(\delta\)-PID | BROJA/\(\sim\), \(\sim_G\), MMI, shared exclusions, or I-PID | Preserved comparator only |
| Gaussian-channel-restricted deficiency | `functional.pid.deficiency-delta-g-gaussian-channel-restricted` | \(\delta_G\), obtained by restricting the degrading channel to the linear additive Gaussian class; it upper-bounds unrestricted deficiency | The unrestricted \(\delta\)-PID or the convex surrogate | Preserved research functional only |
| Convex Gaussian deficiency surrogate | `functional.pid.deficiency-delta-g-hat-convex` | The paper-defined \(\widehat{\delta}_G\)-PID; a further surrogate with proved bounds and extremal agreements relative to \(\delta\) | \(\delta\), \(\delta_G\), or numerical solver error | Preserved research proxy functional only |
| Convex-surrogate law evaluator | `evaluator.pid.deficiency-delta-g-hat-convex-law` | Declared-Gaussian-covariance evaluation route for the \(\widehat{\delta}_G\) convex program | An exact evaluator of \(\delta\) or \(\delta_G\), a sample estimator, or a certificate | Preserved research route only |
| Lagrangian deficiency family | `functional.pid.deficiency-lagrangian-delta-lambda-bivariate` | A parameterized \(\delta^\lambda\)-PID family for \(\lambda\geq0\); the paper states deficiency and BROJA endpoint connections, but its displayed raw objective tends to zero at small \(\lambda\) when exact copying is feasible, so a normalized or lexicographic limit theorem is still required | One parameter-free functional, an unqualified `recovers_on_domain` edge, or an alias relation among parameter instances | Preserved research family only; \(\lambda=0\) is a degenerate endpoint |
| Information-deficiency I-PID | `functional.pid.information-deficiency-i-bivariate` | A distinct bivariate PID designed to capture unique information through an auxiliary random variable; proved Blackwellian for jointly Gaussian laws, with the general claim left conjectural | BROJA/\(\sim\), \(\delta\), \(\delta^\lambda\), or a Gaussian sample route | Preserved research identity only |
| Bivariate infomorphic objective | `composition.infomorphic.bivariate-2025` | A paper-defined parametric family of weighted compositions of named categorical MGW atoms and residual entropy; one coefficient vector identifies one objective instance | The empirical law construction, gradient route, a new PID definition, or an estimator guarantee | Future typed composition only |
| Bivariate infomorphic empirical conditional-gradient route | `evaluation.infomorphic.bivariate-conditional-gradient-2025` | An empirical binned-source/model-conditional law construction and partial derivative with the histogram and bin map held fixed | The full derivative of MGW over an induced joint law or a consistency theorem | Future typed evaluation edge only |
| Trivariate infomorphic objectives | `composition.infomorphic.trivariate-2025` | A distinct parametric family that composes named PID atoms into local neural objectives; the atom map and coefficient vector identify an instance | The bivariate PNAS object, a PID definition, or an estimator guarantee | Future typed composition only |
| Trivariate infomorphic empirical conditional-gradient route | `evaluation.infomorphic.trivariate-conditional-gradient-2025` | An empirical-source/model-conditional training route with declared binning and stopped-gradient semantics | The objective itself or a full-law gradient theorem | Future typed evaluation edge only |
| Matthias et al. PID inconsistency result | `theorem.pid-axiom-inconsistency.2025-v1` | An arXiv-v1 theorem about one unavoidable axiom trade-off | Evidence that PID is useless, that MGW is validated, or that a particular negative atom is necessary | Required interpretation warning |

The categorical MGW paper establishes fixed-support differentiability with respect to a
categorical PMF and a target chain rule. It does not promise nonnegative net atoms. The 2025
arXiv-v1 preprint gives a three-source counterexample and proves that no single PID family with an
associated redundancy measure can satisfy local positivity, target chain rule, and re-encoding
invariance for all source counts. It does not prove nonexistence at every fixed source count. It
does not validate MGW or prove that any particular negative MGW atom is necessary or correct. Its
counterexample is discrete; the paper leaves a continuous-only restriction open.

The categorical MGW and Ehrlich objects are related, not interchangeable. Ehrlich et al. present a
purely continuous analytic counterpart inspired by categorical shared exclusions. They also state that their
practical analytic formulation does not retain every property of the earlier measure-theoretic
proposal. Shared motivation and notation do not supply a general output-identification theorem.

Gaussianity likewise does not choose a PID. The \(\sim\)-PID is the BROJA-PID, but \(\sim_G\) is
a distinct functional obtained by restricting its coupling optimization to jointly Gaussian
\(Q\). It upper- or lower-bounds the corresponding BROJA atoms in general; equality for Gaussian
input \(P\) is conjectured. Likewise, unrestricted deficiency \(\delta\), Gaussian-channel-
restricted \(\delta_G\), and the further convex-surrogate \(\widehat{\delta}_G\)-PID are distinct
objects related by restrictions, bounds, and proved extremal agreements. The Lyu–Clark–Raviv
hierarchy uses yet another definition. The same 2023
unique-information paper also defines the parameterized \(\delta^\lambda\) family and a distinct
I-PID; it proves the I-PID Blackwellian for jointly Gaussian laws but leaves the general claim
conjectural. For \(N\geq3\), the Lyu hierarchy does not assign redundancy and therefore must not be
serialized as a complete PID lattice. Its declared-covariance evaluator and sample-covariance
plug-in estimator are different graph edges. An empirical covariance matrix, marginal normality
check, ridge, or bias correction does not prove the required joint law or make atomic and dependent
robotics data admissible.

Do not encode “bias corrected” as a maturity or correctness claim. Later component-specific bias
work reports unequal bias across PID atoms and describes its own corrections as heuristic despite
large-sample analysis. The 2023 \(\sim_G\) correction is therefore one named estimator route that
still needs matched-regime bias, variance, coverage, and failure evaluation and does not become an
unrestricted BROJA estimator without the missing equality theorem.

Functional identity alone is insufficient. Metadata must also bind `quantity_id`, the exact
antichain or lattice coordinate, cumulative-versus-Möbius construction, pointwise-versus-averaged
scope, averaging law, and `net | informative | misinformative` component. A cumulative value is
not its atom. A net atom is not either nonnegative component. Metadata must also bind
`input_law_kind` and units. Use one tagged route: `SampleEstimator { estimator_id }` or
`DeclaredLawEvaluator { evaluator_id }`. Derive evaluation kind from that variant. Add an optional
typed preprocessing identity and `composition_id` when applicable. This prevents contradictory
estimator/evaluator metadata. A finite-sample estimator is an edge to its named functional. It is
not a free-standing peer functional. Reserve “older comparator” for the Williams–Beer `I_min`
object, and name that object explicitly whenever it appears.

The same arithmetic can have two scientific roles. Counts from sampled rows define a plug-in
estimate of an unknown population law. Integer ratios supplied as an analytic law define a direct
evaluation of that specified law. The input's nominal type must select the role. A result must not
infer the role from values or from the fact that both inputs reduce to rational masses.

### Which objects Prisoma actually needs

Prisoma does not need one implementation for every paper in the graph. It needs each paper at the
layer where that paper has authority:

- use categorical MGW as the active low-dimensional shared-exclusions functional;
- keep the Ehrlich continuous functional and its kNN estimator as a separate default-off research
  path with closed application gates;
- use the parthood paper and the inconsistency result to state atom semantics and unavoidable
  axiom trade-offs;
- use the Schick-Poland measure-theoretic work to bound theorem and domain claims, not as an alias
  for the practical Ehrlich estimator;
- use the 2017 goal-coordinate paper as design context only; and
- represent the bivariate and trivariate infomorphic papers as separate downstream objective
  compositions.

Thus the active estimator request is narrow. The bibliographic and metadata contract is broader.
Prisoma must preserve both without pretending that every related paper defines another runtime PID
mode.

## What Prisoma needs

Prisoma has two different needs.

First, it needs a low-overhead categorical reference path. That path should evaluate two-source
MGW shared exclusions on explicit finite laws. It should keep empirical counts, specified
rational masses, and declared binary64 masses distinct without duplicating rows.

Second, it needs an honest continuous research path. That path must bind the complete tuple-level
population assertion, the source gauge, preprocessing, estimator settings, and failure state. It
must not infer joint regularity from continuous marginal axes.

Neither path passes Prisoma's application gate. Upstream estimator work can improve estimator
evidence. It cannot establish that a VLDA tensor is a valid source, that an action target is not
injected, or that an observed association is causal.

## Observed upstream boundary

The requests above are not renames for current public APIs. At observed head `bc3aa80`:

- the method catalog has strong origin and constraint fields, but it does not expose separate
  paper-defined functional nodes for the empirical categorical and continuous estimator rows;
- the stable categorical entry points accept equal-weight rows;
- the sparse empirical PMF remains an internal implementation type;
- exact-count certification is a bounded validation surface, not a stable sparse-law evaluator;
- `ResourceEstimate` exposes zero, triangular, and contiguous constructors, but no checked public
  composition method;
- quantizer reports bind training-input and transform-input hashes, but no row-set relation;
- row resampling types independent rows or one weakly stationary series, but no group plan;
- permutation reports record calibrated-p-value versus approximate-surrogate semantics, but both
  values still occupy the same optional binary64 field shape rather than distinct nominal types;
- the continuous report binds source-gauge prose, but one support contract carries a broad
  population assertion; and
- no declared binary64 finite-law MGW or infomorphic-objective API was found.

The recovery head has terminal full CI run
[`31773937366`](https://github.com/sepahead/pid-rs/actions/runs/31773937366) with 45 successful
jobs. Its CodeQL run
[`31773937102`](https://github.com/sepahead/pid-rs/actions/runs/31773937102) has four successful
jobs. These receipts repair the earlier exact-head hosted failures. They do not establish
downstream compatibility, estimator validity, or a Prisoma application gate.

The neutral tail name is therefore an ergonomic follow-up. It is not a request to replace the
existing typed calibration field. The other items close distinct input, provenance, resource, or
dependence gaps.

## Recommended Rust shape

These names are a design sketch, not a required public API. The type separations are the
requirement.

```rust
struct CategoricalCountLaw2Ref<'a> {
    source_1_states: DiscreteMatRef<'a>,
    source_2_states: DiscreteMatRef<'a>,
    target_states: DiscreteMatRef<'a>,
    positive_counts: &'a [u64],
}

struct SpecifiedRationalLaw2<S1, S2, T> {
    alphabets: DeclaredAlphabets2<S1, S2, T>,
    positive_cells: Vec<RationalNumeratorCell2<S1, S2, T>>,
    common_denominator: PositiveInteger,
}

enum SourceMarginalOrigin {
    SpecifiedFiniteLaw,
    EmpiricalBatchHistogram,
}

enum ConditionalOrigin {
    SpecifiedConditional,
    ModelConditional,
}

struct FiniteLawConstructionProvenance {
    source_marginal_origin: SourceMarginalOrigin,
    conditional_origin: Option<ConditionalOrigin>,
    law_construction: LawConstructionId,
}

struct DeclaredMassTable2<S1, S2, T> {
    alphabets: DeclaredAlphabets2<S1, S2, T>,
    positive_cells: Vec<MassCell2<S1, S2, T>>,
    provenance: FiniteLawConstructionProvenance,
}

struct NormalizedFiniteLaw2<S1, S2, T> {
    canonical_cells: Vec<NormalizedMassCell2<S1, S2, T>>,
    normalization: NormalizationReceipt,
    identity: LawIdentity,
}

enum ComputedRowIdentityRelation {
    SameOrderedUniqueSequence,
    SameUniqueIdentitySet,
    PartialOverlap { shared_identities: usize },
    Disjoint,
}

struct ComputedRowRelationReceipt {
    relation: ComputedRowIdentityRelation,
    fit_set_digest: [u8; 32],
    evaluation_set_digest: [u8; 32],
    fit_cardinality: usize,
    evaluation_cardinality: usize,
    sampling_unit_kind: SamplingUnitKind,
    algorithm_revision: RowRelationAlgorithmRevision,
}

enum FitEvaluationRelation {
    ComputedFromDeclaredIdentities(ComputedRowRelationReceipt),
    CallerAssertedDisjoint,
    Unknown,
}

struct MonteCarloPermutationPValue {
    value: f64,
    family: PermutationFamily,
    hypothesis: HypothesisIdentity,
    null_receipt: PermutationNullReceipt,
}
struct SurrogateTailFraction(f64);
```

Use a borrowed, validated count-law view in `pid-core`. Keep fixed-width counts there. Bridge its
bounded intersection to the standalone arbitrary-precision certifier through a versioned lossless
schema adapter, not one shared Rust type. Keep empirical counts and specified rational masses as
different public nominal types. Require specified numerators to sum exactly to the denominator.
Only empirical input may emit sample-count, occupancy, or coverage diagnostics. Scaling counts
preserves the normalized law and MGW values. It changes empirical-sample identity, count-based
diagnostics, and possibly admission.

Build a declared-mass table first. The evaluator must reject a non-unit total. If convenience
normalization is needed, expose a separately named transform that returns the normalized type and
a receipt. The evaluator must accept only that normalized type. The receipt must bind the raw
input bits, canonical order, summation algorithm, normalization factor, and output identity.

Record source-marginal and conditional provenance separately. An infomorphic law can combine an
empirical batch histogram with a model conditional. Labeling that whole law only “specified” or
“model-induced” would erase its hybrid construction.

If declared alphabets are retained, use separate positive-law and declared-state-space identities.
The current certifier binds only positive support. Extend its schema before claiming that it binds
unused zero-probability states. Otherwise, defer declared-alphabet metadata from the first bridge.

Compute row relations jointly from caller-declared stable row identities. This proves the relation
among supplied identities, not that they truthfully identify physical sampling units. Retain exact
per-row identities or their digests while computing disjointness. A whole-set digest alone cannot
prove overlap. The first API must reject duplicate identities. If multiplicity is later required,
add an explicitly occurrence-aware multiset contract; do not call it an unordered set. A caller
assertion must never inhabit a computed variant. Keep row and group identities separate: row IDs
are unique, while episode or cluster IDs may repeat across rows and define the split/uncertainty
hierarchy.

Give nominal tail types private checked constructors. Return them only from a transform whose
typed null supports that calibration. Bind every p-value to family, hypothesis, null, and algorithm
identity. Add typed BH and BY entry points. Retain the current numeric functions for compatibility,
but document that they do not carry provenance. Consider deprecation only through the 1.0 policy.

The first resource API can stay small:

```rust
impl ResourceEstimate {
    fn from_components(
        estimated_bytes: u128,
        pairwise_distances: u128,
        operations_hint: u128,
    ) -> Self;
    fn checked_component_sum(
        operation: &'static str,
        estimates: impl IntoIterator<Item = Self>,
    ) -> PidResult<Self>;
}
```

The constructor is required because external crates cannot construct a `#[non_exhaustive]`
literal. Use component-wise checked sums. Summed `estimated_bytes` is a conservative co-resident
memory upper bound when components may coexist. Summed `pairwise_distances` and
`operations_hint` are additive work charges, not simultaneous resources. The callback declaration
must charge retained output and transient scratch separately. Do not claim a universal sequential
peak until those resources have different types.

## Twelve candidate approaches

The following options cover the useful design space. The order is not a priority order.

### O1 — sparse empirical-count-law MGW API

Expose the categorical MGW plug-in estimator through a borrowed, validated sparse view of joint
states and positive fixed-width empirical counts. Do not expand a count of `n` into `n` repeated
rows. Keep a specified rational law in a distinct nominal type even when it uses the same integer
ratios.

Pros:

- It exposes the scientific input object directly.
- It lowers memory and runtime for repeated sampled observations.
- It gives a lossless schema adapter to the exact-count certifier a natural boundary.
- Existing row APIs can lower to the same internal law.

Cons:

- Canonical duplicate handling and state ordering become public contracts.
- Count overflow and total-count limits need typed failures.
- An empirical count sample remains a plug-in input, not a population-validity result.

### O2 — declared binary64 finite-law MGW evaluator

Accept a canonical sparse categorical law with declared positive binary64 masses. Evaluate the
paper-defined MGW functional directly on that law.

Require orthogonal source-marginal, conditional, and construction provenance. Do not accept
survey, importance, frequency, or reliability weights in the first API. Those weights add a
sampling-design estimand that this evaluator cannot infer.

Pros:

- It covers model-induced laws such as `p(r,c) p_theta(y|r,c)` without sampling `Y`.
- It is the smallest useful runtime evaluator for bounded infomorphic-law checks.
- It keeps the functional separate from a finite-sample estimator.

Cons:

- Binary64 normalization, zero cells, duplicate states, and support changes need exact policies.
- A value evaluator is not an automatic-differentiation training engine or an empirical
  weighted-sample estimator.
- Soft sigmoid masses are not certified by an integer-count proof.

### O3 — exact-count refinement bridge and analytic-rational extension

First, connect bounded positive empirical counts to the existing two-source interval and
exact-product assurance route. Then extend that same engine with a separate specified-rational-law
schema whose positive numerators sum to its declared common denominator. Do not create a second
exact engine. Do not encode a specified probability law as fake empirical counts.

Pros:

- It avoids row expansion for exact rational fixtures.
- It supplies an independent oracle for the binary64 declared-law evaluator.
- It can enclose logarithmic values and certify sign or zero decisions within its exact scope.

Cons:

- Denominator and bit-length growth need strict limits.
- Logarithms of rational masses are generally not rational. The route certifies outward intervals
  and bounded rational-product sign decisions, not “exact rational PID values.”
- It does not certify irrational intended probabilities or arbitrary binary64 neural
  probabilities.
- It remains a conditional per-law certificate, not a population statement.

### O4 — fit/evaluation row-relation receipts

Add a generic relation receipt. Integrate it first through additive equal-width-quantizer entry
points that accept row identities. Keep old entry points and emit `Unknown`. Distinguish computed
ordered equality, unordered equality, overlap, disjointness, caller assertion, and unknown.

Pros:

- It closes a present provenance ambiguity.
- It makes same-row descriptive analysis honest.
- It supports frozen-transform and held-out contracts without inferring them from hashes.

Cons:

- Matrix hashes alone cannot prove row-set disjointness.
- Computed relations require caller-declared stable row identities or full identity lists.
- Reports and Python bindings need an explicit additive schema and migration policy.

### O5 — checked multi-stage resource composition

Add a public component constructor and checked composition for `ResourceEstimate`. Start with a
checked component-wise aggregate. Treat bytes as a conservative co-resident bound and the two work
fields as additive charges. Add a more precise phase plan only when retained outputs and transient
scratch are typed separately.

Pros:

- It lets a callback declare preprocessing plus PID work without unchecked arithmetic.
- It enables nested preprocessing inside resampling with fail-closed preflight.
- It removes repeated private composition code.

Cons:

- A naive sequential maximum can undercount outputs retained across phases.
- A conservative sum can reject work that would fit in memory.
- An opaque callback can still misstate what it executes.

### O6 — group-aware resample plans

Add a generic schedule-first API that preserves group or cluster boundaries by construction. Keep
“episode” semantics in Prisoma. Keep independent-cluster, one-series block, and two-level designs
as different schemes. Give repeated sampled groups distinct occurrence identities. Treat callback
admissibility as a separate contract.

Pros:

- It maps cleanly to robotics rollouts and repeated restored states.
- It prevents accidental concatenation of independent episodes into one stationary series.
- One schedule can be shared across paired statistics and content-bound by hash.

Cons:

- Equal-group and equal-row targets are different estimands.
- A two-level episode-plus-block scheme needs stronger assumptions.
- Few episodes cannot support credible group-level uncertainty.
- Whole-group replacement duplicates numeric rows. The pinned continuous KSG/Ehrlich route rejects
  the resulting exact ties; occurrence IDs repair provenance, not coordinates.
- Without-replacement group subsampling targets an m-of-G diagnostic and does not automatically
  calibrate a confidence interval for the full-group estimator.

### O7 — nominal row-transform tail API

Add `row_transform_tail_fraction_*` or an equivalent neutral name. Return distinct nominal types
for a family-, hypothesis-, and null-bound Monte Carlo p-value and an approximate surrogate tail.
Keep the current numeric APIs for compatibility and mark their missing provenance explicitly.

Pros:

- It matches the typed distinction between a Monte Carlo p-value and a surrogate score.
- It can make BH and BY adjustments accept only the p-value type.
- It is small and backward compatible.

Cons:

- It does not improve estimator validity.
- Names and types cannot repair an invalid null scheme.

### O8 — fixed-source-law invariance fixture

Commit a small two-source specified-rational law family. Freeze one alphabet, event map, and
lattice. Define each full joint law separately. Independently marginalize every condition and
require exact equality of `P(S1,S2)` before testing the informative-component invariance.
Treat the relationship as paper-derived and the fixture as project-defined validation unless an
exact primary-source theorem locator is pinned.

Pros:

- It tests a defining MGW structure with exact expected relations.
- It is cheap, deterministic, and easy to review.
- It separates averaged informative and misinformative components from signed net atoms.

Cons:

- It validates one functional property and one implementation path only.
- An empirical-count representation is a distinct law kind and needs its own fixture identity.
- “Misinformative” remains a formal negative-surprisal component, not error or harm.

### O9 — tuple-level Ehrlich assumption and gauge-sensitivity contract

Replace one blanket continuous support assertion with a typed PID2 population contract. Bind the
required marginal and joint regularity, finite-information assertions, relative source precision,
comparable scale, source gauges, scaling, and target preprocessing.

Pros:

- It blocks the false inference from marginal continuity to joint absolute continuity.
- It makes the Ehrlich estimand and its gauge visible.
- It improves fail-closed consumer integration.

Cons:

- The assertions remain caller declarations.
- Finite samples cannot prove absolute continuity or finite mutual information.
- This contract cannot clear estimator or application validity.

### O10 — typed infomorphic objective specification

Define one record for the objective composition and another for its empirical evaluation or
training edge. Bind exact functional and coordinate IDs, atom coefficients, residual entropy,
hybrid law construction, binning,
gradient-stop boundaries, fit relation, and numerical guards. Do not call either record a PID
measure.

Pros:

- It prevents a training objective from masquerading as a PID functional or estimator.
- It makes published infomorphic configurations reproducible.
- It creates a clean future boundary for value and gradient checks.

Cons:

- The record alone does not implement or validate learning.
- A differentiable engine requires more numerical and autodiff work.
- This may fit a separate crate better than `pid-core`.

### O11 — new mixed, conditional, or temporal PID functionals

Add a new method for mixed continuous-categorical laws, conditional PID, or time-series transfer.

Pros:

- These domains are relevant to robotics.
- A correct method could reduce destructive quantization.

Cons:

- These are new scientific objects with large theorem and validation burdens.
- Conditional dependence is not causal flow.
- They invite silent substitution for the active MGW or Ehrlich objects.

### O12 — GPU or MPS acceleration

Port selected continuous distance work or categorical lattice work to a device backend.

Pros:

- Large pairwise continuous workloads might run faster.
- A bounded MPS backend could use Apple unified memory.

Cons:

- Small categorical laws do not need a device.
- Device reductions can weaken determinism and numerical parity.
- Acceleration does not repair support, bias, or application validity.

## Twenty-lens decision review

Scores are ordinal decision aids, not measurements. `0` means blocked or harmful in the current
scope. `1` means weak. `2` means viable with conditions. `3` means strong.

The scientific lenses are:

1. `L1` scientific-object identity;
2. `L2` direct Prisoma hypothesis leverage;
3. `L3` closure of a confirmed gap;
4. `L4` theory readiness;
5. `L5` independent-oracle readiness;
6. `L6` finite-sample or inferential-validity gain;
7. `L7` leakage and selection defense;
8. `L8` dependent-row defense;
9. `L9` support-domain honesty; and
10. `L10` misuse resistance.

| Option | L1 | L2 | L3 | L4 | L5 | L6 | L7 | L8 | L9 | L10 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| O1 count law | 3 | 3 | 3 | 3 | 3 | 1 | 1 | 1 | 3 | 3 |
| O2 declared law | 3 | 3 | 3 | 3 | 2 | 0 | 1 | 1 | 3 | 2 |
| O3 rational bridge | 3 | 2 | 2 | 3 | 3 | 0 | 1 | 1 | 3 | 3 |
| O4 row relation | 3 | 3 | 3 | 3 | 3 | 2 | 3 | 1 | 2 | 3 |
| O5 resource composition | 3 | 3 | 3 | 3 | 3 | 1 | 3 | 2 | 2 | 3 |
| O6 episode plans | 3 | 3 | 3 | 2 | 2 | 1 | 1 | 3 | 2 | 3 |
| O7 nominal tail | 3 | 2 | 2 | 3 | 3 | 1 | 1 | 2 | 2 | 3 |
| O8 invariance fixture | 3 | 2 | 2 | 3 | 3 | 0 | 1 | 1 | 3 | 3 |
| O9 Ehrlich contract | 3 | 1 | 3 | 2 | 2 | 0 | 2 | 2 | 3 | 3 |
| O10 objective record | 2 | 2 | 2 | 2 | 2 | 0 | 2 | 1 | 2 | 2 |
| O11 new functionals | 1 | 2 | 1 | 0 | 1 | 1 | 1 | 2 | 1 | 1 |
| O12 device acceleration | 3 | 1 | 1 | 3 | 2 | 0 | 0 | 0 | 0 | 1 |

The engineering lenses are:

11. `L11` bounded-resource feasibility;
12. `L12` deterministic replay;
13. `L13` Rust API fit;
14. `L14` Python API fit;
15. `L15` backward compatibility;
16. `L16` provenance and serialization;
17. `L17` formal-assurance tractability;
18. `L18` low overhead on an M4 Max;
19. `L19` maintenance cost, where a high score means lower cost; and
20. `L20` adoption speed.

| Option | L11 | L12 | L13 | L14 | L15 | L16 | L17 | L18 | L19 | L20 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| O1 count law | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 2 | 2 |
| O2 declared law | 3 | 3 | 2 | 3 | 3 | 3 | 2 | 3 | 2 | 2 |
| O3 rational bridge | 2 | 3 | 2 | 2 | 3 | 3 | 3 | 2 | 2 | 2 |
| O4 row relation | 3 | 3 | 3 | 3 | 2 | 3 | 2 | 3 | 3 | 3 |
| O5 resource composition | 3 | 3 | 3 | 2 | 3 | 3 | 2 | 3 | 3 | 3 |
| O6 episode plans | 2 | 3 | 2 | 2 | 3 | 3 | 2 | 3 | 2 | 2 |
| O7 nominal tail | 3 | 3 | 3 | 2 | 3 | 3 | 1 | 3 | 3 | 3 |
| O8 invariance fixture | 3 | 3 | 2 | 1 | 3 | 3 | 3 | 3 | 3 | 3 |
| O9 Ehrlich contract | 2 | 3 | 2 | 2 | 2 | 3 | 1 | 2 | 1 | 1 |
| O10 objective record | 2 | 3 | 2 | 3 | 3 | 3 | 1 | 2 | 1 | 1 |
| O11 new functionals | 0 | 2 | 1 | 1 | 2 | 2 | 0 | 1 | 0 | 0 |
| O12 device acceleration | 1 | 1 | 1 | 1 | 2 | 2 | 1 | 2 | 0 | 0 |

No total score is used. A low `L1`, `L4`, or `L5` score is a stop condition. Runtime cannot offset
an unidentified functional or an unavailable validation oracle.

## Selected roadmap

### Wave 0 — repair meaning before adding scope

Keep the current scientific-object split. Add distinct paper-defined functional and output-
coordinate identities through a catalog-v2 schema or separate scientific-object registry. Keep
current method IDs canonical for their existing routes. Link them through the typed graph edges
defined above. Correct any metadata that calls categorical MGW and continuous Ehrlich one measure.
Replace “genuine PID” qualifiers with the exact functional name. They can wrongly imply that other
well-defined PID measures are illegitimate rather than different.
Make old fitted-transform entry points report unknown row relation unless computed evidence exists.

This wave also retains the exact method catalog, units, aggregation scope, and status boundaries.
It is prerequisite work, not a scientific extension.

### Wave 1 — foundation bundle

Implement O5, O4, and O7 as contract repair. Implement O1 and O8 as the first scientific input
extension. If only one scientific feature can land, choose O1. If only one low-risk engineering
change can land, choose O5.

O5 is the best immediate engineering change. Add a public component constructor and checked
component sum first. It must sum bytes, pairwise work, and operation hints with overflow checks.
Only the byte field is a conservative co-resident memory bound. The other fields are additive work
charges. Do not use a maximum for memory unless the API types retained outputs and transient
scratch.

The resampling callback must then bind one preprocessing disposition. At minimum, distinguish
`refit_inside_every_transform` from `held_fixed_conditional_analysis`. A successful refit claim
must include a receipt bound to that invocation's exact row-index or input digest. The engine can
check receipt consistency. It cannot prove that an opaque callback told the truth.

O4 is the best immediate provenance change. Start additively with the equal-width quantizer. A
report must default to `unknown`. It may report disjointness computed from declared identities only
after jointly checking retained per-row identities or digests. This does not verify that IDs denote
the true sampling units. Different domain-separated matrix hashes do not prove different rows. A
caller assertion must remain typed as an assertion. It must not create a held-out result type.

O7 is a small misuse-prevention change. The current weak-stationarity circular-shift route must not
inhabit the nominal p-value type. A future exact group-invariance randomization test would be a
different typed route. Add family-aware BH and BY entry points for nominal p-values. Compatibility
APIs can keep the current binary64 output during migration.

O1 is the best overall scientific foundation. Its canonical table mechanics should underlie the
existing row API, exact fixtures, and future declared-law paths. Its public empirical result must
remain distinct from specified-law results. O8 is its first nontrivial oracle.

### Wave 2 — finite-law reference path

Integrate the count half of O3 with the existing certifier. Add the distinct analytic-rational
schema next. Then implement O2 against bounded certificates from those paths.

The first Prisoma use should be one project-owned analytically specified contextual law. It should
not be a VLA embedding result. Preserve whether the same integer ratios represent empirical
counts or a specified rational law. Equal ratios define the same functional value but not the
same sampling diagnostics or scientific object.

The declared-law constructor must:

- reject NaN, infinity, and negative mass;
- define the treatment of negative zero and zero-mass cells;
- canonicalize or reject duplicate state rows;
- bind the exact input bits and canonical state order;
- reject non-unit input at the evaluator boundary;
- require any normalization through a separately named, receipt-bearing transform;
- use deterministic compensated accumulation;
- cap states, coordinates, nodes, bytes, and work; and
- identify the result as a direct finite-law functional evaluation.

It must not emit sample-size, singleton, low-count, or coverage diagnostics. Those diagnostics
belong only to an empirical-count law. A later design-weighted empirical route needs its own
sampling-design contract.

The rational path must not expand counts into rows. It must cap total count, denominator, state
count, and arithmetic bit length. Its certificate must state the exact coordinates that it covers.
Keep MPFR and arbitrary-precision dependencies outside default `pid-core`. It must not certify
arbitrary soft neural probability masses.

### Wave 3 — robotics dependence and continuous honesty

Implement O6, then O9.

For O6, separate these cases:

1. independent and exchangeable rows;
2. independent episodes resampled as whole clusters;
3. one weakly stationary ordered episode with blocks; and
4. independent episodes with a declared within-episode block design.

The API must not concatenate groups. It must record the declared statistical unit, row-weighting
rule, group count, group-size distribution, ordered schedule hash, realized row count, and sampled
group occurrences. Do not call any of these an effective sample size. The schedule must also bind
callback admissibility. Replacement sampling may serve categorical or other duplicate-tolerant
statistics. A continuous KSG/Ehrlich callback must abstain on duplicate-producing schedules. Start
continuous work with a separately justified without-replacement group subsampling diagnostic or
defer it until a weighted or cluster-aware estimator exists; do not claim bootstrap calibration.

For O9, use one typed contract for the complete PID2 tuple. The contract must enumerate every
marginal and joint law required by the three KSG MI terms and the Ehrlich redundancy term. It must
also bind relative precision, comparable scale, source gauges, and preprocessing. Sample
diagnostics can reject a declaration. They cannot prove it. Contract passage cannot clear the
estimator or application gates.

### Wave 4 — objective composition

Consider O10 only after O2 has a stable law identity. Put the first record in a schema, run-log
layer, or separate objective crate rather than stable `pid-core`.

Start O10 as an objective-composition record plus a separate empirical evaluation record. Do not
start with a training engine. Required fields include:

- `functional_id`, exact `quantity_id` and lattice coordinate for every term,
  `composition_id`, and one tagged `route`;
- `SampleEstimator { estimator_id }` or `DeclaredLawEvaluator { evaluator_id }`, from which the
  evaluation kind is derived;
- source and target roles;
- atom coefficients;
- residual-entropy coefficient;
- law-construction rule;
- binning and fit/evaluation relation;
- stopped-gradient boundaries;
- the conditional-path partial derivative with empirical source PMF, bin map, support, and state
  order held fixed;
- numerical guards;
- units;
- software identity; and
- failure or abstention behavior.

Any gradient API must freeze support and state ordering. Its finite-difference checks must perturb
only the model conditional. It must not claim a full-law derivative or estimator consistency.
Support-boundary behavior needs a separate contract.

## Options not selected

Defer O11. A mixed, conditional, or temporal method needs its own defining paper, method identity,
estimation theory, and validation plan. It must not enter as an “extension” of categorical MGW or
continuous Ehrlich by naming alone.

Defer O12. The categorical reference path is small and CPU-suitable. The current scientific risks
are support, finite-sample bias, fit leakage, dependence, and interpretation. Device acceleration
does not close those risks.

Do not add BROJA to Prisoma's active path. It answers a different measure-specific question. Do
not add `I_min` as a fallback. A cross-measure comparison can be a separate sensitivity study only
when its hypothesis, source roles, target, law, and interpretation are frozen in advance.

## Adjacent `pid-runlog` request

Add one neutral decision-record event in a new compatible schema revision. The minimum payload is:

- a nonempty `decision_id`;
- a nonempty domain-defined `stage` such as `forecast_commit` or `execution_receipt`;
- a versioned payload-schema identity;
- the canonical payload;
- a recomputed canonical payload hash;
- step and timestamp when applicable; and
- bounded metadata.

The generic run-log validator should check shape, limits, and the payload hash. It should not infer
domain-specific stage order. A consumer verifier should enforce forecast-before-oracle,
selection-before-execution, and receipt-before-label rules. Preserve schema-2 replay. Do not
reinterpret old `label_observed` records automatically. A migration must retain the old event bytes
or issue a new logical-trace identity under a declared conversion.

This avoids three problems in the current compatibility envelope. Forecasts stop inflating label
counts. Execution receipts stop masquerading as observed outcomes. Generic viewers can display a
decision record without treating it as ground truth. `artifact_logged` is not a substitute because
it requires an external artifact and carries no inline typed payload.

## Acceptance tests for upstream

The first three waves need the following minimum tests.

### Law identity

- Row input and the equivalent sparse count law return bit-identical numeric coordinates only on a
  versioned canonical-lowering intersection. Bind the total-count bound, fixed-width count bound,
  feature/platform identity, arithmetic contract, coordinate set, and exact-count binary64
  ceiling. Full reports retain different provenance.
- Permuting sparse cells does not change the canonical law or result.
- Duplicate cells either reject or merge under one documented deterministic rule.
- Zero, negative, and non-finite masses reject under typed policies.
- The direct evaluator rejects non-unit input. Only a named transform can normalize it.
- Count, specified-rational, and binary64 routes agree on exactly representable bounded fixtures.
- Interval certification applies only to inputs admitted by the rational certifier.
- Binary64 comparisons are bounded regressions, not certificates for the floating evaluator.
- Scaling all counts preserves normalized-law identity and functional values. It changes
  empirical-sample identity, sample size, count diagnostics, and possibly admission.
- A specified rational law never emits empirical occupancy or sampling diagnostics.
- A specified law can emit structural alphabet, positive-support, and mass-range diagnostics under
  names that do not imply sampling.

### MGW semantics

- All averaged atoms reconstruct the required mutual-information terms.
- Informative and misinformative components remain distinct from signed net atoms.
- Every full joint fixture independently marginalizes to the same exact specified-rational source
  law on one frozen alphabet, event map, source order, positive support, antichain lattice, and log
  base.
- That family has an identical averaged informative cumulative and atom vector in every condition.
- For that family, each change in net atom equals the negative change in its misinformative atom.
- A changed source marginal breaks the invariance fixture as expected.
- Misinformative components may remain equal in a special case. The fixture must include at least
  one condition where a misinformative component changes.

### Fit relation

- Identical ordered unique row identities produce computed ordered-sequence equality.
- A reordering produces computed unique-set equality, not ordered equality.
- Duplicate row identities reject in the first API. A future multiset route needs a new nominal
  relation and occurrence semantics.
- Repeated episode identities are accepted only in the separate group field and never satisfy or
  defeat a unique-row relation by themselves.
- Disjoint declared row identities produce a computed disjoint relation.
- Partial overlap records the overlap and cannot claim disjointness.
- Every computed relation binds fit/evaluation digests, cardinalities, sampling-unit kind, and
  algorithm revision.
- Equal matrix bytes with different IDs and different matrix bytes with equal IDs remain distinct
  test cases.
- Digest-only or missing identities remain caller-asserted or unknown.
- Domain-separated hashes of identical matrix bytes do not imply disjoint rows.

### Resources

- Every composition overflow fails before callback execution.
- Conservative composition never undercounts any component.
- The callback declaration separately charges retained output and transient scratch.
- `ZERO` is the identity. Order does not change a nonoverflowing conservative sum.
- Worker multiplicity and every component sum are exact.
- Cancellation returns no partial library result. It does not promise rollback of callback side
  effects.

### Dependence

- No schedule crosses a declared group boundary unless the selected scheme explicitly permits it.
- Paired statistics can reuse one exact schedule identity.
- Mixed group coverage and missing order fail closed.
- Equal-group and equal-row weighting produce different typed estimands.
- Repeated sampled groups receive distinct occurrence identities.
- A repeated-group categorical callback can pass only under its declared sampling estimand.
- A continuous KSG/Ehrlich callback rejects any schedule that duplicates numeric rows.
- Without-replacement group subsampling is labeled as its own m-of-G diagnostic and does not emit a
  bootstrap-calibrated confidence interval without a separate theorem.

### Tail calibration

- The current weak-stationarity circular-shift surrogate cannot construct a nominal p-value.
- A future circular-shift randomization p-value needs a distinct exact invariance/null contract.
- Full and block shuffles construct p-values only under the matching null declaration.
- Every p-value binds its predeclared family, hypothesis, null, null-invariance group or randomized-
  assignment mechanism, orbit conditioning, draw scheme/count/stream, tail rule, correction, and
  exact/conditional/Monte-Carlo status.
- Ordinary row permutation rejects under dependence unless the declared null supplies the required
  invariance. A block, group, or circular transformation cannot borrow validity from a differently
  defined null.
- BH and BY accept nominal p-values and reject surrogate-tail values by type. BH additionally
  requires a justified independence or applicable positive-dependence regime; BY does not repair an
  invalid p-value or a post-selected family.
- Compatibility wrappers are bit-identical to the new API for the same valid null.

### Continuous contract

- Continuous marginals with a singular deterministic joint do not satisfy a full joint contract.
- An unspecified joint assertion fails before estimator work.
- Gauge and preprocessing changes alter the result identity.
- Relative-precision or comparable-scale changes alter the result identity.
- Exact ties reject the observed sample without classifying the population law.

### Low-overhead benchmark

- An arm64 release benchmark records that sparse categorical memory and work scale with distinct
  support size, not expanded sample count.
- Common count scaling does not increase allocations or work on the sparse route.
- This is one bounded measurement, not a universal M4 performance promise.

### Process and publication packet

- Each wave has one versioned Markdown method-change packet. It names every functional, quantity,
  evaluator, estimator, transform, certifier, validation artifact, and objective that changed.
- The packet contains defining equations, paper theorem locators, project derivations, admitted law
  domains, source-count scope, failure behavior, nonclaims, migration impact, and rejected routes.
- Tests map to those exact claims. Passing tests are not evidence for a different functional,
  coordinate, route, source count, or application.
- Negative fixtures, superseded derivations, and failed gates remain addressable. A later route
  does not overwrite them.
- Prisoma records its consumer review in canonical Markdown and a deterministic PDF view. A build
  receipt binds source, renderer, PDF, exact command, extracted-text checks, page count, and visual
  review. The PDF is not an independent authority.

## Questions for the upstream agent

Please answer these before implementation:

1. Can the internal empirical PMF lower from one borrowed validated sparse count-law view without
   duplicating the categorical evaluator?
2. What versioned lossless schema adapter spans the bounded intersection of fixed-width
   `pid-core` counts and the arbitrary-precision certifier?
3. Can the direct evaluator reject non-unit input while a separate transform provides explicit,
   receipt-bound normalization?
4. Will the declared-mass API expose values only, or also derivatives on a fixed support?
5. How will zero-mass cells and support changes affect derivative identity?
6. Can row relations be computed jointly from caller-declared stable identities while retaining
   enough per-row evidence to establish overlap?
7. Which serialized fields must change before 1.0?
8. Can a conservative resource-composition primitive land before a precise phase planner?
9. Should group schedules live in `pid-core`, or should `pid-core` accept a validated external
   schedule plan?
10. Which exact marginal and joint support assertions does the continuous PID2 path require?
11. Does the infomorphic specification belong in `pid-core`, a new crate, or only a schema?
12. Which parts can ship stable, and which must remain behind an experimental feature?

## Companion implementation extract

This extract condenses implementation work selected by the authoritative handoff. It is a
companion, not a second copy-and-send request. Use it only to discuss implementation sequencing
after sending the handoff body. The broad comparator registry and adjacent `pid-runlog` proposal
remain separate and must be scheduled or declined independently.

**Title: Prisoma request for a typed finite-law and resampling foundation**

Prisoma remains pinned to `pid-rs@796c11e`. We are not requesting a pin update or a generic
“Wibral PID.” We need distinct catalog identities for categorical MGW shared exclusions, the
measure-theoretic Schick-Poland construction, the practical continuous Ehrlich formulation, each
finite-sample estimator, and each infomorphic objective composition. Not every catalog identity
needs a runtime mode.

This work is not on the W1 or W2 critical path. It supports a conditional H3 diagnostic only.
Estimator improvements cannot clear Prisoma's population, measure, or application gates.

Please model these objects as a provenance and estimand graph. Keep existing method IDs canonical
for the routes they already identify. Do not alias an estimator to a functional. Add stable
functional and quantity-coordinate identities through a scientific-object registry or method-
catalog v2. Link routes with typed target, evaluator, implementation, recovery-domain,
motivation, validation, and composition edges. Bind complete defining teams, exact references,
publication status, and software revisions in metadata. Separate cumulative lattice values,
Möbius-inverted atoms, net/informative/misinformative components, and pointwise/averaged results.
Ehrlich is a related continuous counterpart, but no general mapping theorem identifies its
outputs with categorical MGW outputs.

Do not add BROJA, Gaussian-restricted, deficiency, I-PID, Lyu-hierarchy, or other comparator
implementations for this request. Prisoma preserves those separately typed objects in its own
research registry; none is a fallback, alias, or new runtime mode here.

Our highest-value upstream package has six parts:

1. separate paper-defined functional and output-coordinate identities plus typed graph edges;
2. a borrowed, bounded sparse empirical-count-law API for two-source categorical MGW;
3. row-relation receipts computed from declared identities, starting additively at the quantizer;
4. a public resource constructor plus a checked all-components-execute sum with typed memory/work
   semantics;
5. an exact fixed-source-law MGW invariance fixture; and
6. nominal p-value and surrogate-tail Rust types that cannot enter the same FDR API.

**Separate maintenance annex; not part of the PID package:** consider a versioned `pid-runlog`
decision-record event. Prisoma currently transports
forecast commitments and execution receipts through `label_observed` because schema 2 has no
neutral inline record. They are not labels. The new event should bind a decision id, stage, payload
schema, canonical payload hash, payload, optional step/time, and bounded metadata. Keep restored-fork
outcomes as labels. Let consumer verifiers enforce stage order. Preserve schema-2 replay and do not
silently reinterpret historical logical traces.

First bridge the fixed-width empirical-count view to the arbitrary-precision exact-count certifier
through a versioned lossless schema adapter. Do not force them into one Rust type. Then add a
distinct specified-rational definition and certificate schema. Add a declared binary64 finite-law
MGW evaluator after those paths agree on bounded fixtures. This last path must evaluate soft finite
laws directly. It must not sample a target and relabel the resulting empirical law as the original
soft law. The exact-count certifier must not be presented as a certificate for arbitrary
sigmoid-derived masses.

The declared-law evaluator must reject non-unit input. Put convenience normalization behind a
separate named transform and receipt. Record empirical source-marginal and model-conditional
origins separately for hybrid infomorphic laws.

For robotics data, the next request is a generic group-aware schedule API. Prisoma will map groups
to episodes. It must keep independent clusters, one stationary series, and two-level designs
separate. It must never splice groups by default. It must also bind callback admissibility.
Whole-group replacement duplicates numeric coordinates; occurrence IDs do not make those rows
valid for continuous kNN. Start the continuous route with a separately justified without-
replacement group subsampling diagnostic, per-group statistic, or future weighted/cluster-aware
estimator. Do not claim bootstrap confidence-interval calibration from the schedule alone.

For continuous PID2, please replace one blanket support assertion with a tuple-level contract that
binds every required marginal and joint population assertion, finite-information assumption,
source gauge, and preprocessing identity. This remains caller-declared and does not validate a
Prisoma application.

Please keep BROJA and `I_min` out of any fallback path. Split an infomorphic objective composition
from its empirical conditional-gradient evaluation edge. Neither is a PID measure. Bind the exact
atom identities, coefficient vector, residual terms, sign convention, and optimization direction
for every objective instance. Device acceleration is lower priority than scientific identity,
bounded resources, row provenance, dependence, and validation.

Please accompany each accepted wave with a versioned Markdown method-change packet. It must map
equations, law domains, object and route IDs, tests, nonclaims, migrations, and negative evidence.
Prisoma will bind its consumer decision to that packet and publish a deterministic PDF view with a
source-to-render receipt. This process artifact is separate from estimator or scientific credit.

The full twenty-lens review, acceptance tests, and twelve design alternatives are in this file.

## Primary sources

- [Wibral et al., neural goal functions](https://pubmed.ncbi.nlm.nih.gov/26475739/)
- [Makkeh, Gutknecht, and Wibral, categorical shared exclusions](https://arxiv.org/abs/2002.03356)
- [Gutknecht, Wibral, and Makkeh, parthood and formal logic](https://arxiv.org/abs/2008.09535)
- [Schick-Poland et al., measure-theoretic discrete and continuous PID](https://arxiv.org/abs/2106.12393)
- [Ehrlich et al., continuous shared exclusions and estimation](https://arxiv.org/abs/2311.06373)
- [Venkatesh and Schamberg, deficiency PID for multivariate Gaussian laws](https://arxiv.org/abs/2105.00769)
- [Venkatesh, Gurushankar, and Schamberg, \(\delta^\lambda\), I-PID, and interpretations of \(\delta\) and BROJA/\(\sim\)](https://arxiv.org/abs/2302.11873)
- [Venkatesh et al., restricted-Gaussian \(\sim_G\) PID, estimation, and bias correction](https://arxiv.org/abs/2307.10515)
- [Koçillari et al., component-specific PID sampling bias and heuristic corrections](https://doi.org/10.1101/2024.06.04.597303)
- [Lyu, Clark, and Raviv, conditional-independence Gaussian hierarchy](https://arxiv.org/abs/2605.09919)
- [Makkeh et al., infomorphic learning framework](https://pmc.ncbi.nlm.nih.gov/articles/PMC11912414/)
- [Schneider et al., infomorphic objective (verified ICLR 2025 proceedings)](https://proceedings.iclr.cc/paper_files/paper/2025/hash/87d8ed41d250c401a68f05100e0a4ef0-Abstract-Conference.html)
- [Matthias et al., PID inconsistency results](https://arxiv.org/abs/2512.16662)
- [`pid-rs` retained Prisoma pin](https://github.com/sepahead/pid-rs/tree/796c11e70f009634b853dc4ada6f565563d82f51)
- [`pid-rs` observed public head](https://github.com/sepahead/pid-rs/tree/bc3aa80fb6025e709c2906a08bce25a4fac40578)
