; Marginal binary potential-outcome laws and their ATE do not identify
; the prevalence of nonzero individual treatment effects.
;
; Each model is a joint law for (Y(0), Y(1)).  The two laws have the
; same Bernoulli(1/2) marginals and ATE zero.  Model A couples the
; potential outcomes identically, whereas model B couples them
; antithetically.  Their nonzero-effect prevalences are therefore 0 and 1.
(set-logic QF_LRA)

(declare-const p00_a Real)
(declare-const p01_a Real)
(declare-const p10_a Real)
(declare-const p11_a Real)
(declare-const p00_b Real)
(declare-const p01_b Real)
(declare-const p10_b Real)
(declare-const p11_b Real)

(define-fun valid_probability ((p Real)) Bool
  (and (<= 0.0 p) (<= p 1.0)))

(assert (valid_probability p00_a))
(assert (valid_probability p01_a))
(assert (valid_probability p10_a))
(assert (valid_probability p11_a))
(assert (valid_probability p00_b))
(assert (valid_probability p01_b))
(assert (valid_probability p10_b))
(assert (valid_probability p11_b))

(assert (= (+ p00_a p01_a p10_a p11_a) 1.0))
(assert (= (+ p00_b p01_b p10_b p11_b) 1.0))

; Model A: Y(0) = Y(1) almost surely.
(assert (= p00_a (/ 1.0 2.0)))
(assert (= p01_a 0.0))
(assert (= p10_a 0.0))
(assert (= p11_a (/ 1.0 2.0)))

; Model B: Y(0) != Y(1) almost surely.
(assert (= p00_b 0.0))
(assert (= p01_b (/ 1.0 2.0)))
(assert (= p10_b (/ 1.0 2.0)))
(assert (= p11_b 0.0))

; Equal observed control-arm and treatment-arm marginals.
(assert (= (+ p10_a p11_a) (+ p10_b p11_b)))
(assert (= (+ p01_a p11_a) (+ p01_b p11_b)))
(assert (= (+ p10_a p11_a) (/ 1.0 2.0)))
(assert (= (+ p01_a p11_a) (/ 1.0 2.0)))

; Equal ATEs: E[Y(1) - Y(0)] = p01 - p10 = 0.
(assert (= (- p01_a p10_a) (- p01_b p10_b)))
(assert (= (- p01_a p10_a) 0.0))

; Different prevalence of a nonzero individual treatment effect.
(assert (= (+ p01_a p10_a) 0.0))
(assert (= (+ p01_b p10_b) 1.0))

(check-sat)
