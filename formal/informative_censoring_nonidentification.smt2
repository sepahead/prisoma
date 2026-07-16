; Finite-artifact missing-outcome countermodel; no censoring mechanism or
; population-level informativeness condition is encoded. Eight eligible landmark
; rows contain two observed targets, four observed
; non-targets, and two censored outcomes. Two latent completions share the exact
; same observed artifact but have target risks 1/4 and 1/2. Without a censoring
; assumption, the point risk is not identified. The second obligation proves the
; conservative rowwise bounds used by the H2 software reference.
(set-logic QF_LIA)

(declare-const eligible Int)
(declare-const observed_targets Int)
(declare-const observed_non_targets Int)
(declare-const censored Int)
(assert (= eligible 8))
(assert (= observed_targets 2))
(assert (= observed_non_targets 4))
(assert (= censored 2))
(assert (= eligible (+ observed_targets observed_non_targets censored)))

; Concrete observational-equivalence witness.
(push 1)
(declare-const hidden_targets_a Int)
(declare-const hidden_targets_b Int)
(assert (= hidden_targets_a 0))
(assert (= hidden_targets_b censored))
(assert (and (<= 0 hidden_targets_a) (<= hidden_targets_a censored)))
(assert (and (<= 0 hidden_targets_b) (<= hidden_targets_b censored)))
(assert (not (= (+ observed_targets hidden_targets_a)
                (+ observed_targets hidden_targets_b))))
(check-sat)
(pop 1)

; Every binary completion lies between observed_targets/eligible and
; (observed_targets+censored)/eligible. The common positive denominator means it
; is sufficient to prove the corresponding count bounds.
(push 1)
(declare-const hidden_targets Int)
(assert (and (<= 0 hidden_targets) (<= hidden_targets censored)))
(assert (or
  (< (+ observed_targets hidden_targets) observed_targets)
  (> (+ observed_targets hidden_targets) (+ observed_targets censored))))
(check-sat)
(pop 1)
