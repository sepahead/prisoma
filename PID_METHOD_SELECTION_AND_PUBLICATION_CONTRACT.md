# PID method-selection, mathematics, and publication contract

- **Status:** current research-governance contract; not a preregistration or empirical result
- **Canonical parent:** [`grandplan.md`](grandplan.md), docset v13.0
- **Prisoma implementation pin:** `pid-rs@796c11e70f009634b853dc4ada6f565563d82f51`
- **Review date:** 2026-08-16
- **Literature cutoff:** 2026-08-13
- **Publication view:** [`output/pdf/PID_Method_Selection_and_Publication_Contract.pdf`](output/pdf/PID_Method_Selection_and_Publication_Contract.pdf)
- **Build and review receipt:** [`docs/audits/2026-08-15-pid-method-contract/PID_PUBLICATION_PROCESS_AND_PDF_RECEIPT.md`](docs/audits/2026-08-15-pid-method-contract/PID_PUBLICATION_PROCESS_AND_PDF_RECEIPT.md)

This contract governs PID method selection, result identity, interpretation, and publication in
Prisoma. It also governs PID-derived work shared with ecosystem projects such as Galadriel. The
canonical thesis claims and four scientific gates remain in [`grandplan.md`](grandplan.md). This
document makes the method mathematics and route boundaries explicit.

## 1. Decision

There is no generic “PID result” and no generic “Wibral PID.” A PID is measure-relative. Its
numerical value is not identified until the source and target variables, probability law,
functional, output quantity or lattice coordinate, source order, aggregation, component, and
units are fixed. A sample result also needs an estimator and sampling design. A declared-law
result needs an evaluator instead.

Prisoma must keep these nine kinds of object separate:

1. a paper-defined **functional**, including an exact instance of a parameterized functional
   family, on a declared class of probability laws;
2. a **quantity or coordinate** defined within that functional, such as a cumulative lattice value
   or one Möbius-inverted atom;
3. a **declared-law evaluator** that maps one specified law to that functional's named quantity;
4. a **sample estimator** that maps observations to an estimate of that named quantity;
5. a fitted or fixed **transform** that creates the variables supplied to a route;
6. an exact or interval **certifier** with a stated admitted domain;
7. a theorem, fixture, or other **validation artifact**;
8. an **objective composition** that combines named PID coordinates with coefficients or terms;
9. an application-level **interpretation or decision rule**.

One object can cite another. It cannot impersonate it. No scientific conclusion transfers between
two functionals, law classes, estimators, transforms, or source counts without an explicit mapping
theorem whose assumptions hold in the application.

Valid novel work must be preserved. A route that is not ready for an active Prisoma hypothesis
stays in the research registry with its exact definition, status, evidence, and blockers. It is not
silently deleted, relabeled as another PID, or used as a fallback.

## 2. Scientific-object registry

The IDs below are Prisoma semantic IDs. They are not names assigned by the cited authors. Existing
`pid-rs` method IDs remain canonical implementation-route IDs. They must not be demoted to aliases
for the new functional nodes.

| Object | Mathematical domain and role | Current Prisoma role | Prohibited equivalence |
|---|---|---|---|
| `functional.shared-exclusions.mgw-categorical` | Makkeh–Gutknecht–Wibral pointwise shared exclusions on categorical probability laws; averaging and Möbius inversion give measure-specific atoms | Active low-dimensional categorical functional | Continuous Ehrlich shared exclusions, `I_min`, BROJA, or an objective |
| `route.shared-exclusions.mgw-empirical-pmf` | Plug-in route from sampled categorical rows or positive empirical counts to the MGW functional on the induced empirical law | Active at the pinned row API; sparse count API requested upstream | A specified population law, a certificate of sampling validity, or a second functional |
| `functional.shared-exclusions.measure-theoretic-2021` | Schick-Poland et al. construction for broad measure-theoretic random-variable domains | Theorem and domain boundary | The practical Ehrlich formula or its kNN estimator |
| `functional.shared-exclusions.ehrlich-continuous` | Ehrlich et al. purely continuous analytic shared-exclusions formulation with its own law and gauge assumptions | Default-off research functional | Binned MGW or a proved categorical-to-continuous limit |
| `route.shared-exclusions.ehrlich-source-disjunction-knn` | Finite-sample nearest-neighbor route targeting the Ehrlich functional | Default-off and application-blocked | The functional itself, categorical MGW, or generic KSG validity |
| `functional.pid.williams-beer-imin` | Williams–Beer categorical redundancy functional | Preserved inactive comparator | Shared exclusions or a rescue after another route abstains |
| `functional.pid.broja-bivariate` | Bertschinger et al. optimization-based bivariate PID, also called the \(\sim\)-PID; union information and unique-information formulations identify the same decomposition | Preserved inactive comparator | Shared exclusions, `I_min`, deficiency PID, I-PID, or a multi-source PID |
| `functional.pid.sim-g-gaussian-restricted` | Venkatesh et al. \(\sim_G\)-PID: the BROJA/\(\sim\) optimization with the coupling \(Q_{MXY}\) restricted to jointly Gaussian laws | Preserved research functional | BROJA/\(\sim\) itself: \(\sim_G\) only bounds it in general, and equality for Gaussian input \(P\) is conjectured |
| `evaluator.pid.sim-g-gaussian-covariance-law` | Covariance-law optimization that evaluates the \(\sim_G\)-PID on an admitted jointly Gaussian law | Future declared-law research route | An evaluator for unrestricted BROJA/\(\sim\), a sample estimate, or proof of the equality conjecture |
| `route.pid.sim-g-gaussian-sample-plugin` | Sample-covariance plug-in estimator targeting \(\sim_G\) | Preserved research candidate for justified continuous Gaussian regimes | The declared-law evaluator, a Gaussianity test, BROJA/\(\sim\), or the bias-corrected route |
| `route.pid.sim-g-gaussian-sample-bias-corrected-2023` | Distinct finite-sample bias-corrected estimator targeting \(\sim_G\) | Preserved research candidate | A new functional, an exact evaluator, unrestricted BROJA/\(\sim\), or permission to analyze atomic spike counts |
| `functional.pid.deficiency-delta-bivariate` | Banerjee et al. deficiency-based bivariate \(\delta\)-PID | Preserved research comparator | BROJA/\(\sim\), \(\sim_G\), MMI, shared exclusions, or I-PID |
| `functional.pid.deficiency-delta-g-gaussian-channel-restricted` | Gaussian deficiency \(\delta_G\), which restricts the degrading channel in the deficiency optimization to linear additive Gaussian channels | Preserved research functional | The unrestricted \(\delta\)-PID; the restriction gives an upper bound on deficiency |
| `functional.pid.deficiency-delta-g-hat-convex` | The paper-defined \(\widehat{\delta}_G\)-PID obtained from a further convex surrogate; its atoms bound the unrestricted \(\delta\)-PID and agree in proved extremal cases | Preserved research proxy functional | \(\delta\), \(\delta_G\), or mere numerical solver error |
| `evaluator.pid.deficiency-delta-g-hat-convex-law` | Declared-Gaussian-covariance route for evaluating the \(\widehat{\delta}_G\) convex program | Preserved research evaluator route | An exact evaluator of \(\delta\) or \(\delta_G\), a sample estimator, or a certificate |
| `functional.pid.deficiency-lagrangian-delta-lambda-bivariate` | Venkatesh–Gurushankar–Schamberg parameterized \(\delta^\lambda\)-PID family for \(\lambda\geq0\); the paper states deficiency and BROJA endpoint connections, but the small-\(\lambda\) raw-value limit needs an explicit normalization or lexicographic theorem | Preserved research functional family; every \(\lambda>0\) identifies one instance and \(\lambda=0\) is a degenerate endpoint | One parameter-free functional, an unqualified `recovers_on_domain` edge, or permission to conflate parameter instances |
| `functional.pid.information-deficiency-i-bivariate` | Venkatesh–Gurushankar–Schamberg I-PID based on information deficiency, designed to capture unique information as an auxiliary random variable | Preserved research candidate | BROJA/\(\sim\), \(\delta\), \(\delta^\lambda\), or a Gaussian estimator |
| `functional.information-hierarchy.lyu-conditional-independence` | Conditional-independence quantities: two-source redundancy, per-source unique information, signed order-\(K\) synergistic effects, narrow synergy, and total synergistic effect; for \(N\geq3\) it deliberately defines no redundancy or complete PID lattice | Preserved research candidate | A complete multi-source antichain PID, a nonnegative atom system, or a shared-exclusions decomposition |
| `evaluator.information-hierarchy.lyu-gaussian-covariance-law` | Log-determinant evaluation of the named hierarchy quantities on positive-definite jointly Gaussian laws | Future declared-law sensitivity route | A sample estimate or a distribution-free formula |
| `route.information-hierarchy.lyu-sample-covariance-plugin` | Sample-covariance plug-in estimator for those named Gaussian hierarchy quantities | Preserved research candidate | The population evaluator, a full \(N\geq3\) PID, or a validation of another measure |
| `composition.infomorphic.bivariate-2025` | Paper-defined parametric family of coefficient-weighted compositions of named categorical PID atoms and residual entropy | Future typed composition; each coefficient vector is a distinct objective instance | A PID functional, estimator, declared-law evaluator, or gradient theorem |
| `composition.infomorphic.trivariate-2025` | Distinct paper-defined parametric family of trivariate local objective compositions | Future typed composition; each atom map and coefficient vector is bound | The bivariate family, a new PID definition, or a coefficient-free objective |
| `diagnostic.shannon-invariant.*` | Measure-independent algebraic summary of mutual-information terms | Screening only after every constituent MI route passes | A PID, causal evidence, or immunity to MI-estimator failure |

This table is scoped to the present Prisoma–`pid-rs` interaction and the reviewed high-value
research candidates. It is not an exhaustive list of PID or adjacent information-decomposition
research. Absence is not rejection. Any unlisted published or project-defined construction—including
alternative redundancy or unique-information families, partial-entropy or integrated-information
decompositions, and mixed-law, conditional, temporal, dynamic, manifold, neuromorphic, or
differentiable programs—enters `PID-M0 preserve` as an `unassessed_literature_candidate`. Preserve
its exact intake before deciding applicability. Do not relabel an adjacent decomposition as an
ordinary target PID, and do not alias an unassessed construction to a listed functional because its
atom names look familiar.

The functional ID does not identify one scalar output. Every PID result must also name:

- `quantity_id`, such as a cumulative shared-exclusion value or a Möbius-inverted atom;
- `lattice_coordinate`, including the exact antichain or node and ordered source roles;
- `component_kind = net | informative | misinformative` when the functional has that split; and
- `aggregation_scope = pointwise | averaged` with the averaging law when applicable.

A non-lattice information hierarchy must instead name its exact output family and index, such as
per-source unique information or \(SE_K\). It must not invent a lattice coordinate so that its result
resembles a full PID. An objective result must name the exact composition family, coefficient vector,
non-PID terms, optimization direction, and the identities of every input quantity.

A cumulative lattice value is not its Möbius-inverted atom. A net atom is not either of its
nonnegative informative or misinformative components. A pointwise realization is not its
probability-law average. Equal field names or familiar labels such as “redundancy” do not make
these coordinates interchangeable.

`project_defined` is an origin tag, not a generic scientific-object kind. A new functional receives
a functional ID; a new estimator receives a route ID; a new diagnostic or composition receives its
own kind-specific ID. A catch-all “extension” ID is forbidden because it would erase the distinction
this contract is intended to preserve.

### 2.1 Typed relations preserve both distinction and lineage

Distinct objects can still have precise relationships. The registry must use typed edges rather
than prose similarity or author-name lineage:

| Edge | Meaning | Required evidence |
|---|---|---|
| `defines_quantity` | A functional defines one named cumulative value, atom, or component coordinate | Defining equations and exact reference locator |
| `targets_functional` | A sample estimator targets one functional and named quantity | Estimator definition and admitted law or sampling domain |
| `evaluates_functional` | A declared-law evaluator computes one functional and named quantity | Evaluator contract and admitted law schema |
| `implements_route` | A software surface implements one estimator or evaluator route | Software revision and conformance evidence |
| `recovers_on_domain` | One construction provably agrees with another on a stated subdomain | Mapping theorem, assumptions, and exact domain |
| `motivated_by` | A later object adopts an idea without claiming equality | Exact reference and bounded relationship statement |
| `validated_by` | A theorem, oracle, counterexample, or fixture tests a named claim | Evidence identity and covered coordinates |
| `composes_quantities` | An objective or diagnostic combines named coordinates | Coefficients, other terms, and evaluation route |

Use `alias_of` only for definitionally identical identities. One route cannot alias several
functionals. Shared authors, notation, lineage, or limiting intuition do not create a
`recovers_on_domain` edge.

The 2017 neural-goal paper supplies a coordinate language for constructing and comparing local
information-theoretic goals. It does not define one universal PID functional. The parthood work
supplies semantic structure. It does not supply a finite-sample estimator. Infomorphic learning
composes PID coordinates into objectives. The objective is downstream of the PID definition.

The Ehrlich paper also draws a boundary inside the shared-exclusions lineage. Its practical
continuous formula is not invariant under arbitrary invertible re-encodings of individual source
variables without a prescribed preprocessing choice. The paper therefore does not identify that
formula as a direct concretization of the earlier measure-theoretic construction. Prisoma records
the two functionals as related and distinct. A shared name or motivation is not a recovery theorem.

The Matthias et al. arXiv-v1 result supplies a three-source counterexample. It rules out one PID
family that would satisfy local positivity, target chain rule, and re-encoding invariance for all
source counts. It does not show that no PID exists at every fixed source count. It does not
validate MGW, make all negative atoms necessary, or make PID useless. Its counterexample is
discrete; the paper explicitly leaves open whether a family restricted to continuous laws can
avoid the inconsistency.

### 2.2 Gaussianity is a law restriction, not a PID identity

"Gaussian PID" is not one method. The relevant research lines answer different questions:

- the bivariate \(\sim\)-PID is the BROJA-PID. The Gaussian paper then defines a distinct
  restricted functional, \(\sim_G\), by allowing only jointly Gaussian couplings \(Q\) in the
  BROJA optimization. Its unique terms upper-bound BROJA/\(\sim\), while its redundant and
  synergistic terms lower-bound them in general. Equality when the input law \(P\) is jointly
  Gaussian is a conjecture, not a mapping theorem. The covariance-law evaluator, sample-covariance
  plug-in estimator, and finite-sample bias-corrected estimator are three further route layers
  targeting \(\sim_G\), not unrestricted BROJA/\(\sim\);
- the bivariate deficiency \(\delta\)-PID is another functional. On Gaussian inputs, \(\delta_G\)
  restricts the degrading channel to the linear additive Gaussian class and therefore upper-bounds
  unrestricted deficiency. The further convex construction defines \(\widehat{\delta}_G\), an
  exact surrogate functional whose relationship to \(\delta\) is by proved bounds and extremal
  equalities. It is not merely numerical error in an exact \(\delta\) evaluator. Agreement among
  \(\delta\), \(\delta_G\), \(\widehat{\delta}_G\), \(\sim_G\), MMI, or another PID on a fixture
  does not make any pair aliases;
- the same 2023 unique-information paper separately defines a parameterized \(\delta^\lambda\)
  family and the I-PID. The I-PID is proved Blackwellian for jointly Gaussian laws, while its
  general Blackwellian property is stated as a conjecture. Neither object is the BROJA/\(\sim\)-PID
  merely because the paper analyzes their relationships; and
- the Lyu–Clark–Raviv construction gives a conditional-independence information hierarchy with
  covariance-law formulas and a plug-in estimator. For \(N\geq3\), the authors deliberately do not
  assign redundancy. Its unique-information and synergy-spectrum quantities therefore must not be
  padded with an invented redundancy, called a complete antichain decomposition, or pooled with
  atoms from another PID.

For every Gaussian route, bind the exact functional or hierarchy quantity, source count, covariance
domain, positive-definiteness or regularization conditions, analytic-law versus sample route, and all
estimator parameters. Ridge regularization and bias correction belong to route identity; neither
silently changes the population functional. Empirical bell-shaped marginals do not prove the required
joint Gaussian law. Atomic spike counts, quantized variables, singular manifolds, dependent episode
rows, and arbitrary VLA embeddings do not become eligible merely because a covariance matrix can be
computed. Such data require a matching law, estimator regime, and application gate or an explicit
multi-estimand sensitivity study.

The Lyu hierarchy needs one further route distinction. Its order-specific synergistic effects can
be negative, so their names do not make them nonnegative PID atoms. A fixed positive ridge makes a
regularized finite-sample route and does not retain the paper's exact blockwise affine invariance.
If a ridge sequence vanishes with sample size, the limit claim needs its own rate and conditioning
assumptions. Record the ridge rule, not only the realized covariance matrix.

Bias correction is also route-specific evidence, not a theorem that the corrected atoms are
unbiased. A later sampling study reports materially different bias across PID components and treats
its own corrections as heuristic despite large-sample analysis. The 2023 \(\sim_G\) correction
therefore remains a named research route requiring independent matched-regime bias, variance,
coverage, and failure studies. It is not the automatic preferred route and does not estimate
unrestricted BROJA/\(\sim\) unless the unproved Gaussian-optimum equality happens to hold.

### 2.3 Bivariate unique-information identities

For target \(M\) and sources \(X,Y\), define

\[
\Delta_P=\{Q_{MXY}:Q_{MX}=P_{MX},\;Q_{MY}=P_{MY}\}.
\]

The BROJA/\(\sim\)-PID fixes one unique-information coordinate by

\[
\widetilde{UI}(M:X\setminus Y)=\min_{Q\in\Delta_P} I_Q(M;X\mid Y).
\]

The three bivariate PID consistency equations then determine redundancy, the other unique term,
and synergy. “Union information” is an equivalent coordinate of this same decomposition, not a
second functional.

The Gaussian-restricted construction changes the feasible set:

\[
\widetilde{UI}_G(M:X\setminus Y)
=\min_{Q\in\Delta_P\cap\mathcal G} I_Q(M;X\mid Y),
\]

where \(\mathcal G\) is the admitted jointly Gaussian coupling class. Consequently
\(\widetilde{UI}_G\geq\widetilde{UI}\); the corresponding \(\sim_G\) redundant and synergistic
terms are lower bounds on the BROJA/\(\sim\) terms. The paper conjectures equality when the input
law \(P_{MXY}\) is jointly Gaussian. It does not prove it. Thus \(\sim_G\) is a separately named
functional related to BROJA/\(\sim\) by bounds and a conjecture, not a law evaluator already known
to return the BROJA/\(\sim\) atoms.

The deficiency \(\delta\)-PID instead measures the cost of approximating the \(M\to X\) channel
through \(Y\):

\[
\delta(M:X\setminus Y)
=\inf_{P_{X'\mid Y}}
\mathbb E_M\!\left[
D_{\mathrm{KL}}\!\left(P_{X\mid M}\Vert
P_{X'\mid Y}\circ P_{Y\mid M}\right)\right].
\]

Restricting \(P_{X'\mid Y}\) to linear additive Gaussian channels defines \(\delta_G\), so
\(\delta_G\geq\delta\). The paper then defines the further convex surrogate
\(\widehat{\delta}_G\), with
\(\widehat{\delta}_G\geq\delta_G\geq\delta\). After the paper's redundancy symmetrization, its
\(\widehat{\delta}_G\) unique atoms upper-bound, and its redundancy and synergy atoms lower-bound,
the unrestricted \(\delta\)-PID atoms; proved extremal cases agree. “Approximate” describes this
mathematical surrogate relationship. A numerical implementation can still have additional solver
error and needs a separate evaluator identity.

The 2023 \(\delta^\lambda\) family combines approximation divergence and conditional-information
leakage in a parameterized objective,

\[
\delta^\lambda(M:X\setminus Y)
=\inf_{P_{X'\mid MY}}\left\{
\mathbb{E}_{M}\!\left[D_{\mathrm{KL}}(P_{X\mid M}\Vert P_{X'\mid M})\right]
+\lambda I(M;X'\mid Y)\right\},
\]

with the induced-channel and symmetrization conventions fixed by the paper. The paper states that
the \(\lambda\to\infty\) and \(\lambda\to0\) regimes yield the deficiency and \(\sim\)-PIDs,
respectively. The displayed raw scalar requires more care: when an exact copy is feasible,
choosing \(X'=X\) gives

\[
0\leq \delta^\lambda(M:X\setminus Y)
\leq \lambda I(M;X\mid Y).
\]

Thus \(\delta^\lambda\to0\) as \(\lambda\downarrow0\) whenever the conditional information is
finite. The normalized value \(\delta^\lambda/\lambda\) or a lexicographically selected optimizer
can approach the
\(\sim\)-PID objective. Prisoma therefore records the small-\(\lambda\) relationship as a paper
claim needing a normalization, existence, and limit theorem before adding `recovers_on_domain`.
Each \(\lambda>0\) is part of functional-instance identity; \(\lambda=0\) is a degenerate endpoint,
not an alias for BROJA/\(\sim\).

The separate I-PID starts from information deficiency,

\[
\delta^I(M:X\setminus Y)
=\sup_{P_{T\mid M}}\bigl(I(T;X)-I(T;Y)\bigr),
\]

and symmetrizes this quantity into a bivariate PID. Its optimizer \(T\) is intended to represent
the part of \(M\) uniquely accessible from one source. The paper proves the resulting I-PID
Blackwellian for jointly Gaussian \(P_{MXY}\) and labels the unrestricted Blackwellian statement a
conjecture. That theorem is a property of I-PID; it does not turn I-PID into BROJA, \(\delta\), or a
Gaussian sample estimator.

## 3. Result identity is mandatory

Every serialized or published PID result, named information-hierarchy quantity, or PID-derived
objective result must bind this identity tuple:

```text
scientific_object_kind
scientific_object_id
functional_instance = Unparameterized
                    | Parameterized { exact_parameters, instance_id }
defining_references_and_versions
quantity = Cumulative { quantity_id, lattice_coordinate_or_antichain }
         | MobiusInvertedAtom { quantity_id, lattice_coordinate_or_antichain }
         | NamedFunctionalQuantity { quantity_id, output_indexing_structure }
         | ObjectiveValue { composition_family_id, objective_instance_id }
component_kind = net | informative | misinformative | not_applicable
source_variables_in_order
target_variable
source_count_and_lattice_or_output_structure
input_law_kind
declared_law_identity_or_sample_and_split_identity
route = sample_estimator | declared_law_evaluator
route_id_and_revision
route_parameters_and_configuration
transform_identity_and_fit_scope
fit_evaluation_row_relation
population_and_support_contract
gauge_metric_and_scale_when_applicable
zero_mass_null_event_and_boundary_conventions
information_unit_and_log_base
aggregation = Pointwise { realization_identity }
            | Averaged { averaging_law }
sign_convention
sampling_unit_dependence_and_resampling_design
uncertainty_procedure_and_multiplicity_family
software_and_feature_identity
validation_scope_and_status
application_gate_status
objective_composition = NotApplicable
                      | Composition { family_id, coefficient_vector,
                                      input_quantity_identities, non_pid_terms,
                                      optimization_direction }
```

`quantity` and `aggregation` are tagged choices. Do not serialize construction or averaging again
as free strings that can contradict the selected variant. A route may support several variants,
but one result instance names exactly one variant and coordinate.

`route` is a tagged choice. A result cannot claim both `sample_estimator` and
`declared_law_evaluator`. Evaluation kind must be derived from that variant, not repeated as a
possibly contradictory string.

The same normalized categorical PMF can arise from different scientific inputs. Sample counts
produce a plug-in estimate of an unknown population law. Exact rational masses can specify an
analytic law. The functional values can agree while sample identity, occupancy, uncertainty, and
claim scope remain different. Nominal types must preserve that distinction.

## 4. Mathematical non-transfer rules

### 4.1 PID equations do not choose a measure

For two sources,

\[
I(S_1,S_2;T)=R+U_1+U_2+S,
\]

\[
I(S_1;T)=R+U_1,\qquad I(S_2;T)=R+U_2.
\]

These equations do not identify \(R\). A redundancy principle chooses a PID measure. Two atom
vectors that satisfy the equations can still answer different scientific questions.

### 4.2 Quantization changes the estimand

A fitted quantizer maps numeric variables to new categorical variables. MGW on those variables is
not the Ehrlich continuous functional of the original numeric variables. Binning sensitivity is
therefore an estimand sensitivity, not only an estimator robustness check. No categorical-to-
continuous limit is claimed without a theorem.

### 4.3 Negative atoms and signed components are not semantic labels

MGW informative and misinformative components are formal surprisal components. “Misinformative”
does not mean wrong, deceptive, unsafe, or harmful. Net atoms can be negative. Never clamp them
unless the named functional itself defines the operation. Never translate a sign into an
application meaning without independent theory and intervention evidence.

### 4.4 Transform invariance is functional- and route-specific

Population mutual information can be invariant under suitable invertible transformations. A
finite-sample estimator need not be. The Ehrlich continuous construction also exposes source
gauge and relative-scale choices. Scaling, normalization, PCA, PLS, pooling, quantization, and
learned projection must appear in result identity. Stability under one transform is not a general
invariance theorem.

### 4.5 Exact finite-law evidence has a bounded scope

An integer-count certifier can prove facts for its admitted positive count laws and arithmetic
bounds. It does not certify arbitrary binary64 neural probabilities. A rational-law extension can
certify its own admitted schema. A binary64 evaluator remains a deterministic numerical evaluator,
not an exact certifier, even when it agrees on bounded fixtures.

A declared-law evaluator must reject non-unit mass. Convenience normalization is a separately
named transform with its own receipt because it changes the declared input. A soft specified law
must be evaluated directly; drawing target samples and evaluating the induced empirical law does
not evaluate the original soft law.

The planned fixed-source-law MGW fixture tests a paper-derived identity under one frozen alphabet,
event map, source order, antichain lattice, log base, positive source support, and exact source
marginal. The MGW informative cumulative term depends only on the matching source union-event
probability. Averaging therefore depends only on the exact source marginal. Möbius inversion on
the same lattice preserves equality of the informative atom vector. Under those conditions,
`delta(net_atom) = -delta(misinformative_atom)` for every coordinate. This statement does not
survive a changed source marginal, event map, lattice, support convention, or component identity.
Until an exact theorem locator is pinned, describe the fixture as project-defined validation of a
paper-derived algebraic identity. Do not promote the fixture itself to a paper-defined theorem.

### 4.6 Higher source count is a different mathematical regime

The source count and antichain lattice are part of the fully instantiated mathematical result and
admitted domain, even when one stable functional-family ID spans several source counts. A two-
source oracle does not validate three- or four-source atoms. Higher-order categorical MGW work
remains valuable, but it needs source-count-specific reconstruction, exact-law fixtures,
complexity bounds, and interpretation. A pairwise screen is not a higher-order PID.

### 4.7 Cross-PID studies are multi-estimand studies

Comparing different PID constructions can be scientifically valuable. It must be designed as a
multi-estimand study, not as a search for whichever measure gives the preferred answer. Freeze each
functional, coordinate, route, assumptions, hypothesis, and interpretation before outcomes are
inspected. Pass each route's own gates, report disagreement, and control any family of claims.

Never average atom vectors across functionals, vote them into a “consensus PID,” select a measure
after seeing its sign, or treat numerical agreement as a mapping theorem. Agreement is one observed
comparison result. Disagreement is not an implementation failure unless an applicable theorem or
oracle requires equality.

## 5. Applicability decision procedure

Choose the route from the scientific question and population law. Do not choose it from which API
returns a number.

| Declared problem | Admissible route | Current disposition |
|---|---|---|
| Exact finite categorical law | Direct MGW declared-law evaluation; rational or interval assurance when admitted | Upstream request; not in the Prisoma pin |
| Sampled categorical rows | MGW empirical-PMF plug-in estimator with sampling and occupancy diagnostics | Available at the pin |
| Sparse positive empirical counts | Bounded MGW count-law plug-in route sharing the row kernel | Highest-value upstream request |
| Numeric data deliberately mapped to categories | Fitted categorical estimand with transform and row-relation receipts | Available only as a descriptive warning route at the pin |
| Purely continuous, full-dimensional, finite-information tuple | Ehrlich functional through its named kNN estimator and gauge contract | Default-off; application gate blocked |
| Exactly specified jointly Gaussian two-source vector law | One predeclared \(\sim_G\), \(\delta_G\), or \(\widehat{\delta}_G\) functional plus its exact declared-law route; unrestricted BROJA/\(\sim\) or \(\delta\) may be claimed only with an applicable equality theorem | Preserved research candidates; no current Prisoma route |
| Sampled rows under a predeclared jointly Gaussian two-source model | One named \(\sim_G\) sample route, with plug-in versus bias-corrected identity fixed before outcomes; a sample-covariance route is not the population evaluator | Preserved research candidates; no current Prisoma route |
| Exactly specified or sampled jointly Gaussian \(N\)-source law | One named Lyu–Clark–Raviv hierarchy quantity through its law evaluator or plug-in estimator | Research candidate; for \(N\geq3\), not a complete PID |
| Deterministic continuous target path or singular joint law | No current continuous PID route | Abstain or define a different estimand |
| Mixed, atomic, quantized, singular, or unknown law | A separately defined matching functional and estimator | No automatic route; research definition required |
| Independent episodes or clusters | Group-preserving schedule appropriate to the statistic | Upstream schedule substrate requested |
| High-dimensional transformed VLA tensors | No active PID interpretation until matched-regime validation passes | Current NO-GO/blocked boundary |
| Local neural training goal | Named objective composition plus a separate evaluation/gradient route | Future work; never call the objective a PID measure |

The decision procedure is:

1. Freeze the scientific question, target population, source order, target, and timing.
2. State the law class. Include deterministic ancestry, mixed support, and dependence.
3. Choose one functional or named information hierarchy whose domain and axioms match the
   question.
4. Choose one evaluator or estimator that targets that object and exact quantity.
5. Bind every applicable lattice coordinate or hierarchy index, component, aggregation law,
   transform, fit relation, gauge, unit, and source-count output structure.
6. Pass the applicable population, measure, route, and application gates.
7. Compare against simpler non-PID quantities and strong prediction baselines.
8. Publish the complete denominator, abstentions, warnings, and negative controls.

If step 2 is unresolved, stop. A method name cannot repair an unidentified population law.

## 6. Four gates, with route-specific meaning

Prisoma's H3 sample analysis retains four independent gates:

1. **Population gate.** The quantity is defined, finite, and relevant for the declared population.
2. **Measure gate.** The functional or hierarchy's mathematical commitments fit the scientific
   question.
3. **Estimator gate.** The sample estimator recovers that named object and quantity in the frozen
   regime with adequate bias, failure detection, and uncertainty behavior.
4. **Application gate.** The real variables and sampling process lie inside the validated regime.

A declared-law evaluator has no finite-sample estimator claim. Replace gate 3 with an
**evaluator-correctness gate** that checks implementation, canonicalization, arithmetic, and an
independent oracle. This substitution does not clear the population or application gates. An exact
certifier supplies assurance only for the facts and inputs named by its certificate.

Passing one gate never passes another. KSG MI validation does not validate Ehrlich redundancy.
Categorical exact assurance does not validate continuous shared exclusions. A low-dimensional
fixture does not validate high-dimensional VLA embeddings. Stable output does not establish a
correct estimand.

## 7. Dependence, row identity, and uncertainty

### 7.1 Row relations

Matrix hashes do not prove that transform-fit rows and evaluation rows are disjoint. Compute row
relations from caller-declared stable sampling-unit identities. The initial contract must reject
duplicate identities. It can then distinguish:

- the same ordered unique sequence;
- the same unique identity set in another order;
- partial overlap with an exact shared count; and
- disjoint unique identity sets.

If repeated identities are scientifically meaningful, add a later multiset or occurrence-aware
contract. Do not call a repeated collection an unordered set. A caller assertion remains an
assertion. It must never inhabit a computed-proof variant.

A row identity and a group identity are different fields. Every observed row receives one unique
row identity; several rows may carry the same episode or cluster identity. If the independent
sampling unit spans multiple rows, record that hierarchy and use it for splitting and uncertainty.
Never substitute a repeated episode ID for the unique row ID merely to obtain a relation result.

### 7.2 Group schedules

A schedule API can prevent accidental splicing of independent episodes. It does not, by itself,
make every statistic valid. Sampling whole groups with replacement duplicates all numeric rows in
a selected episode. Distinct occurrence IDs preserve provenance but do not change the duplicated
coordinates. The pinned continuous KSG/Ehrlich route rejects exact ties.

The first continuous-compatible episode route should therefore be a prespecified without-
replacement group subsampling diagnostic, a per-group statistic, or a future weighted/cluster-
aware estimator. It must state its target and must not claim bootstrap confidence-interval
calibration. Replacement schedules may remain useful for categorical statistics and other
callbacks that admit duplicated rows. A continuous callback must abstain when a realized schedule
violates its sample contract.

### 7.3 Tail probabilities

A nominal Monte Carlo permutation p-value and a stationary-surrogate tail fraction are different
types. A nominal p-value must bind its hypothesis and predeclared family; the null-invariance group
or randomized-assignment mechanism that licenses the transformation; any orbit conditioning; the
draw scheme, count, and random-stream identity; the tail rule; and the finite-sample correction.
Exact enumeration, conditional randomization, and Monte Carlo approximation are different statuses.
A circular-shift score cannot acquire p-value status from the same arithmetic.

Only typed nominal p-values may enter BH or BY adjustment, and admission by type does not establish
the adjustment theorem's assumptions. BH requires a justified independence or applicable positive-
dependence regime (or a cited extension that covers the declared family). BY can address arbitrary
dependence among valid p-values under its own finite-family procedure; it does not repair an invalid
null, an invalid transformation group, or a post-selected family.

## 8. Current pin versus upstream roadmap

The following table prevents future work from being written as if it already ships in Prisoma.

| Capability | `pid-rs@796c11e` | Prisoma status |
|---|---|---|
| Row-based empirical categorical MGW | Present | Active descriptive route |
| Fitted equal-width categorical MGW | Present | Same-row warning route; no H3 evidence |
| Continuous Ehrlich kNN report route | Present behind experimental surface | Application-blocked |
| Williams–Beer `I_min` | Present | Preserved inactive comparator, never fallback |
| Separate functional/quantity registry and typed graph edges | Absent | Upstream Wave 0 request |
| Borrowed sparse empirical-count MGW2 API | Absent | Upstream priority |
| Specified rational-law evaluator/certificate schema | Absent | Later upstream priority |
| Declared binary64 finite-law evaluator | Absent | Only after count/rational agreement |
| Computed fit/evaluation row-relation receipt | Absent | Upstream priority |
| Public checked component sum with typed memory/work meaning | Absent | Upstream priority |
| Nominal p-value and surrogate-tail nominal types | Absent; calibration is recorded in one shared report/value shape | Upstream priority |
| Episode/group schedule contract | Absent | Experimental upstream request |
| Complete tuple-level Ehrlich support/gauge contract | Incomplete | Experimental upstream request |
| Neutral `pid-runlog` decision event | Absent in schema 2 | Orthogonal upstream request |
| Bivariate BROJA/\(\sim\), restricted \(\sim_G\), deficiency \(\delta\), restricted \(\delta_G\), and convex-surrogate \(\widehat{\delta}_G\) functionals and routes | Absent | Preserved as distinct research identities; no runtime request yet |
| Bivariate \(\delta^\lambda\) family and information-deficiency I-PID | Absent | Preserved research identities; no runtime request yet |
| Lyu–Clark–Raviv Gaussian hierarchy evaluator/plug-in routes | Absent | Preserved research identities; no runtime request yet |

The current world-model reference therefore continues to store forecast commitments and execution
receipts in strictly named schema-2 `label_observed` compatibility envelopes. They are not outcome
labels. Prisoma must migrate only after a new run-log schema is adopted and historical replay is
preserved.

## 9. Upstream priority order

The best sequence is:

1. add functional and quantity-coordinate identities plus typed graph edges while preserving
   current method IDs as canonical routes;
2. add `ResourceEstimate` construction and checked component sums with typed memory/work meaning;
3. add unique-row identity relations and typed nominal p-value/surrogate outputs;
4. refactor one private canonical weighted positive-law kernel;
5. expose a borrowed, bounded sparse empirical-count MGW2 view and a distinct empirical result;
6. add the fixed-source-law MGW fixture and bridge admitted counts to the existing exact certifier;
7. add a distinct specified-rational-law schema and certificate path;
8. add a binary64 declared-law evaluator only after the count and rational routes agree;
9. add group schedule infrastructure with explicit callback admissibility;
10. strengthen the continuous Ehrlich tuple-law and gauge contract; and
11. represent infomorphic objectives as downstream typed compositions outside estimator identity.

Do not add BROJA or `I_min` to Prisoma's active path. Do not prioritize GPU or MPS work before
identity, mathematics, assurance, and dependence are correct. Do not add mixed, conditional, or
temporal functionals by extending an existing name. Each requires its own definition and program.

The first resource composition is a checked component-wise aggregate, not one homogeneous physical
quantity. Its contract means every listed component executes and its memory may coexist. Summed
`estimated_bytes` is then a conservative co-resident memory upper bound, not an exact sequential
peak. Summed `pairwise_distances` and `operations_hint` are additive work charges, not simultaneous
resources. Alternatives and mutually exclusive branches need a different plan type. A later phase
plan may take memory maxima only after retained outputs and transient scratch have different types
and explicit lifetimes.

## 10. Preserve and promote novel PID research

Novel valid work is an asset for the thesis and future papers. Record its origin separately from
its implementation maturity, evidence level, and current disposition. These axes are orthogonal.
For example, a paper-defined functional can be application-blocked, while a project-defined
evaluator can have strong exact-oracle evidence.

| Origin | Meaning | Allowed claim |
|---|---|---|
| `paper_defined` | Exact quantity or estimator defined by a cited paper | The cited scope only |
| `paper_derived` | New implementation or composition derived from cited mathematics | Repository derivation, not new theory by default |
| `project_defined` | New diagnostic, contract, evaluator, estimator, or objective proposed here | Explicit project origin and unvalidated status |
| `external_reference_code` | Separately maintained implementation used for a named comparison | That bounded comparison only |
| `no_implementation` | Literature object or request with no repository route | Definition and gap only |

Record disposition on a second axis:

| Disposition | Meaning |
|---|---|
| `active_route` | Eligible for the named analysis after every gate passes |
| `preserved_comparator` | Retained for a named comparison; never an automatic fallback |
| `research_candidate` | Incomplete mathematical, software, or empirical program |
| `application_blocked` | May support methods work but not the current application claim |
| `negative_boundary` | Reproducible failure of a stated theorem, oracle, route, or application gate |
| `superseded_preserved` | Replaced by a correction while the original record remains immutable |

Every new PID-related object needs:

- a mathematical definition and law domain;
- exact output quantity, coordinate construction, component, and aggregation;
- source count, lattice, and source/target order;
- stated axioms, invariances, and known trade-offs;
- a proof or derivation ledger;
- a separate evaluator or estimator identity;
- an independent oracle or counterexample family;
- resource and determinism bounds;
- sampling, dependence, and uncertainty assumptions;
- typed failure and abstention behavior;
- comparison with the nearest existing functional without pooling atoms;
- an application hypothesis that simpler MI or prediction baselines cannot already answer; and
- a publication claim that does not exceed the evidence level.

Research routes worth preserving include higher-source categorical MGW, exact and interval finite-
law assurance, specified rational and binary64 law evaluation, dependence-aware schedules,
continuous tuple/gauge contracts, bivariate BROJA/\(\sim\), \(\sim_G\), \(\delta\), \(\delta_G\),
\(\widehat{\delta}_G\), \(\delta^\lambda\), and I-PID programs, the distinct
conditional-independence Gaussian hierarchy, and typed infomorphic compositions. Mixed-law,
conditional, temporal, manifold, neuromorphic, and new differentiable PID proposals may also be
valuable. They stay distinct research programs until their definitions, mapping theorems,
estimators, and oracles exist.

A failed application gate removes a route from an active application claim. It does not erase a
valid functional, implementation, theorem, negative result, or research artifact. If a derivation
is wrong, retain the superseded bytes and publish the correction. Do not overwrite the record.

## 11. PID publication evidence ladder

Use the strongest achieved level and name all missing higher levels.
The `PID-P*` labels below are local to this PID evidence ladder. They are not the repository's
priority labels and are not the ecosystem E0–E5 capability scale.

| Level | Evidence | Does not establish |
|---|---|---|
| `PID-P0 definition` | Exact object, law, equations, and provenance | Correct implementation |
| `PID-P1 derivation` | Reviewed proof or derivation obligations | Numerical correctness |
| `PID-P2 implementation` | Deterministic bounded code and conformance tests | Functional recovery |
| `PID-P3 oracle` | Independent exact, analytic, or cross-implementation fixtures | Planned-regime estimator validity |
| `PID-P4 route regime` | Estimator bias/coverage at matched scale and dependence, or evaluator error/conditioning across its admitted law domain; both include failure detection | Real-application validity |
| `PID-P5 application` | Frozen application gate and held-out incremental value over baselines | Causal mechanism or transport |
| `PID-P6 intervention/replication` | Target-engaging intervention and independent family/site replication | Universal validity |

Every PID paper packet must contain:

1. the complete result-identity tuple;
2. the defining equations and full-team primary references;
3. the population, measure, route, and application verdicts;
4. source/target ancestry and timing;
5. transform-fit and evaluation row receipts;
6. exact software, feature, data, split, and environment identity;
7. oracle, negative-control, and matched-regime results;
8. dependence-aware uncertainty and the complete multiplicity family;
9. all requested, produced, warned, and abstained denominators;
10. strong non-PID baselines at matched information access and compute;
11. retained null, negative, and contradictory results; and
12. a claim table that separates functional, coordinate, estimator, application, and causal conclusions.

Potential publication contributions must stay distinct:

- a methods/assurance paper can study the scientific-object graph, sparse laws, exact bridges, and
  misuse-resistant types;
- a categorical paper can study one exact fixed-law MGW property and its bounded evaluator;
- a continuous methods paper can study a matched Ehrlich estimator regime and abstention boundary;
- an application paper can test held-out incremental value only after the first three gates pass;
- a rigorous negative paper can localize why a popular application regime is undefined,
  non-identifiable, or estimator-invalid.

None of these requires pretending that all PIDs agree.

## 12. Ecosystem contract

Every consumer must negotiate exact capability, not a family nickname. The handshake must bind the
functional or hierarchy, quantity and lattice coordinate or output index, component, aggregation,
route and parameters, source count, input-law kind, units, sampling and uncertainty design, software
revision, features, and gate status. An objective consumer additionally binds the complete
coefficient vector and every input-quantity identity. A Galadriel result and a Prisoma result that
use the same `pid-rs` route are correlated uses of one implementation family. They are not
independent replications merely because two applications emitted them.

Cross-project comparison is admissible only when the result-identity tuple matches or a mapping
theorem defines the conversion. Otherwise, report a multi-estimand comparison. Never pool the
atoms. Never let a consumer relabel an abstention as zero or route it to a different functional.

## 13. Twenty-question review before any method enters a claim

1. What exact functional or hierarchy and output quantity, lattice coordinate, or hierarchy index
   are being evaluated?
2. What law class is its mathematical domain?
3. Which equations, axioms, trade-offs, and typed relations to nearby objects define it?
4. What source count, lattice, source order, and target are fixed?
5. What units, aggregation law, component, null-event convention, and sign convention are used;
   for an objective, what exact input identities, coefficient vector, residual terms, and
   optimization direction define the instance?
6. Is the route an evaluator or an estimator?
7. What transform created the variables, and where was it fit?
8. What row identities prove the fit/evaluation relation?
9. What marginal and joint support assertions are required?
10. What gauge, metric, scale, regularization, and route parameters affect identity?
11. What dependence, sampling unit, resampling design, and multiplicity family define uncertainty?
12. Can resampling create ties or leave the estimator's admitted domain?
13. What independent oracle, theorem, or counterexample applies?
14. At what dimensions, sample sizes, signals, and support has the route been validated?
15. Which numerical, arithmetic, resource, and determinism bounds apply?
16. Which simpler MI, invariant, prediction, or causal baseline answers the same question?
17. What would falsify the functional, estimator, and application claims separately?
18. What interpretation is forbidden without intervention evidence?
19. Which ecosystem outputs are genuinely independent, which share code, fixtures, prompts,
    reviewers, or assumptions, and which load-bearing derivations and results has the human
    researcher independently reproduced and accepted under the applicable disclosure policy?
20. What exact evidence level can the paper claim, and which higher levels remain open?

Any unanswered question blocks a confirmatory PID claim. It does not block preserving the work as
a typed research candidate.

## 14. Reproducible method-selection and publication process

The process is evidence. A final number without its decision path is not a reproducible PID result.
Use the stages below for every new method, route, comparison, or application. The `PID-M*` labels
name workflow stages. They are not the `PID-P*` evidence levels in Section 11.

### 14.1 Stage contract

| Stage | Required work | Required output | Stop rule |
|---|---|---|---|
| `PID-M0 preserve` | Snapshot the proposal, defining papers, code, fixtures, and known failures before reinterpretation | Immutable intake manifest and origin record | Stop if the defining object or version cannot be identified |
| `PID-M1 type` | Assign separate IDs to the functional, quantity, evaluator, estimator, transform, certifier, validation artifact, objective, and interpretation | Scientific-object graph with typed edges | Stop if one ID spans nonidentical objects |
| `PID-M2 prove` | Re-derive the equations, domains, bounds, invariances, source-count scope, and limiting claims | Mathematics ledger with theorem locators, derivations, and counterexamples | Stop if an asserted mapping lacks a theorem or proof |
| `PID-M3 admit` | Match the population law, source and target ancestry, support, gauge, dependence, sampling unit, and transform | Applicability decision with explicit rejected routes | Stop if the law or timing is unresolved |
| `PID-M4 design` | Choose one evaluator or estimator, oracle, resource bound, uncertainty method, multiplicity family, and non-PID baselines | Frozen route and validation plan | Stop if the route cannot target the named quantity |
| `PID-M5 freeze` | Freeze hypotheses, coordinates, parameters, splits, warnings, abstentions, and claim language before outcomes | Content-bound analysis packet and amendment policy | Stop if outcome access predates the freeze |
| `PID-M6 execute` | Run the declared commands and retain all requested, produced, warned, abstained, and failed cases | Command log, environment identity, outputs, and complete denominator | Stop on identity drift or unplanned substitution |
| `PID-M7 challenge` | Recompute critical quantities and review mathematics, numerics, statistics, software, and application claims | Review dispositions, negative controls, and unresolved objections | Stop if a required objection is unresolved |
| `PID-M8 publish` | Build the paired Markdown and PDF artifacts and bind them to the evidence bundle | Publication packet, hashes, visual review, and claim table | Stop if source and publication view diverge |

Do not skip a stage because an API returns a finite value. A stopped route remains a preserved
research object. Record the exact blocker and the evidence needed to resume it.

### 14.2 Mathematics and applicability ledger

The mathematics ledger must contain:

1. the defining equation for every reported quantity;
2. the admitted probability-law class and source count;
3. every symmetry, invariance, bound, and sign claim used in interpretation;
4. a locator for each paper theorem and a separate label for each project derivation;
5. the exact construction from cumulative values to atoms or hierarchy outputs;
6. a counterexample for every tempting but invalid cross-method substitution;
7. numerical conditioning and arithmetic assumptions;
8. the evaluator or estimator target and its failure set; and
9. a route-by-route decision that says `admitted`, `blocked`, `deferred`, or `not_applicable`.

For a new derivation, include enough algebra for another researcher to reproduce it without the
implementation. For a paper claim that appears inconsistent with its displayed objective, retain
the paper statement and the project derivation as separate records. Do not repair the citation by
silently changing the functional.

### 14.3 Execution and review packet

Every execution packet must bind:

- the complete result-identity tuple from Section 3;
- source, data, split, environment, software, feature, and configuration hashes;
- exact commands, exit status, stdout and stderr custody, and resource observations;
- the complete requested denominator and every abstention or failure reason;
- independent oracle results and deliberately failing controls;
- uncertainty and multiplicity receipts;
- comparison baselines with matched information access and compute;
- reviewer role, review scope, objection, disposition, and timestamp;
- reviewer and tool ancestry, including shared prompts, fixtures, implementations, model families,
  and any AI-assisted review or derivation that must not be counted as independent replication;
- a candidate-ownership record for every load-bearing result: the human researcher must be able to
  derive it, explain its assumptions and failure modes, reproduce its decisive checks, and comply
  with the university's disclosure and authorship rules; and
- amendment history without overwriting the earlier packet.

The reviewer must distinguish recomputation from rerunning the same implementation. Shared code,
fixtures, or assumptions are shared ancestry. They are not independent replication.

### 14.4 Paired Markdown and PDF contract

The Markdown file is the canonical scientific source. The PDF is a deterministic publication
view. The PDF must never become a second authority with hand-edited claims.

For each released pair:

1. render the PDF from the exact Markdown bytes with a versioned repository script;
2. record the source, renderer, and PDF SHA-256 digests and byte sizes;
3. record the renderer, Python, and font identities plus the exact command;
4. extract PDF text and check all required headings, equations, warnings, and references;
5. render every page to an image;
6. inspect every page for clipping, overlap, missing glyphs, broken equations, split tables,
   unreadable type, unexpected blank pages, and stale headers;
7. record page count and visual-review disposition; and
8. rerender after any source or renderer change.

A matching hash proves byte identity only. It does not prove readable layout or correct
mathematics. Visual review does not replace text extraction or scientific review. The build
receipt must state both results.

### 14.5 Publication decision

A positive paper claim needs the achieved `PID-P*` level, every missing higher level, and the exact
claim boundary. A negative paper must identify the earliest failed stage, reproduce the failure,
and state which nearby objects remain valid. A comparison paper must treat each PID as a separate
estimand and retain disagreement.

Publication does not require every research route to become active. It requires every route to be
named honestly. Preserve deferred, blocked, superseded, and negative work with enough information
to resume, audit, or cite it.

## 15. Primary sources

- [Williams and Beer, nonnegative decomposition and `I_min`](https://arxiv.org/abs/1004.2515)
- [Bertschinger et al., unique information and the BROJA construction](https://arxiv.org/abs/1311.2852)
- [Wibral et al., PID coordinates for neural goal functions](https://pubmed.ncbi.nlm.nih.gov/26475739/)
- [Makkeh, Gutknecht, and Wibral, categorical shared exclusions](https://arxiv.org/abs/2002.03356)
- [Gutknecht, Wibral, and Makkeh, parthood and formal logic](https://arxiv.org/abs/2008.09535)
- [Schick-Poland et al., general measure-theoretic PID](https://arxiv.org/abs/2106.12393)
- [Ehrlich et al., continuous shared exclusions and estimation](https://arxiv.org/abs/2311.06373)
- [Kraskov, Stögbauer, and Grassberger, k-nearest-neighbor mutual-information estimation](https://doi.org/10.1103/PhysRevE.69.066138)
- [Venkatesh and Schamberg, bivariate deficiency PID for multivariate Gaussian laws](https://arxiv.org/abs/2105.00769)
- [Venkatesh, Gurushankar, and Schamberg, \(\delta^\lambda\), I-PID, and interpretations of \(\delta\) and BROJA/\(\sim\)](https://arxiv.org/abs/2302.11873)
- [Venkatesh et al., restricted-Gaussian \(\sim_G\) PID, estimation, and bias correction](https://arxiv.org/abs/2307.10515)
- [Koçillari et al., component-specific PID sampling bias and heuristic corrections](https://doi.org/10.1101/2024.06.04.597303)
- [Lyu, Clark, and Raviv, conditional-independence Gaussian information hierarchies](https://arxiv.org/abs/2605.09919)
- [Makkeh et al., bivariate infomorphic networks](https://pmc.ncbi.nlm.nih.gov/articles/PMC11912414/)
- [Schneider et al., trivariate infomorphic objectives](https://proceedings.iclr.cc/paper_files/paper/2025/hash/87d8ed41d250c401a68f05100e0a4ef0-Abstract-Conference.html)
- [Matthias et al., PID inconsistency results](https://arxiv.org/abs/2512.16662)
