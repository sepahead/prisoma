; Concrete countermodel: equal Bernoulli(1/2) marginals do not identify expected
; paired disagreement. One valid coupling is comonotone; another is antimonotone,
; so a paired-response distance can depend on the coupling.
(set-logic QF_LRA)
(declare-const a00 Real)
(declare-const a01 Real)
(declare-const a10 Real)
(declare-const a11 Real)
(declare-const b00 Real)
(declare-const b01 Real)
(declare-const b10 Real)
(declare-const b11 Real)

(define-fun bernoulli-half-coupling
    ((p00 Real) (p01 Real) (p10 Real) (p11 Real)) Bool
  (and (>= p00 0.0) (>= p01 0.0) (>= p10 0.0) (>= p11 0.0)
       (= (+ p00 p01 p10 p11) 1.0)
       (= (+ p10 p11) (/ 1.0 2.0))
       (= (+ p01 p11) (/ 1.0 2.0))))

; Coupling A: both responses are equal almost surely.
(assert (= a00 (/ 1.0 2.0)))
(assert (= a01 0.0))
(assert (= a10 0.0))
(assert (= a11 (/ 1.0 2.0)))

; Coupling B: the responses differ almost surely.
(assert (= b00 0.0))
(assert (= b01 (/ 1.0 2.0)))
(assert (= b10 (/ 1.0 2.0)))
(assert (= b11 0.0))

(assert (bernoulli-half-coupling a00 a01 a10 a11))
(assert (bernoulli-half-coupling b00 b01 b10 b11))
(define-fun disagreement_a () Real (+ a01 a10))
(define-fun disagreement_b () Real (+ b01 b10))
(assert (= disagreement_a 0.0))
(assert (= disagreement_b 1.0))
(assert (not (= disagreement_a disagreement_b)))
(check-sat)
