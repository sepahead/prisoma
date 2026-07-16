; H2 fixed-prediction paired Brier-improvement bounds.
;
; For one binary outcome y, baseline-minus-diagnostic Brier improvement is
;   delta(y) = (y - p_b)^2 - (y - p_d)^2.
; Its two endpoint values exhaust the support y in {0,1}. The obligations below
; prove the endpoint envelope, its [-1,1] range, rowwise sharpness for a concrete
; nondegenerate pair, and convex closure of bounds under nonnegative weighting.

(set-logic QF_NRA)

(declare-const p_b Real)
(declare-const p_d Real)
(declare-const y Real)

(define-fun delta_0 () Real (- (* p_b p_b) (* p_d p_d)))
(define-fun delta_1 () Real
  (- (* (- 1 p_b) (- 1 p_b))
     (* (- 1 p_d) (- 1 p_d))))
(define-fun delta_y () Real
  (- (* (- y p_b) (- y p_b))
     (* (- y p_d) (- y p_d))))
(define-fun lower () Real (ite (<= delta_0 delta_1) delta_0 delta_1))
(define-fun upper () Real (ite (<= delta_0 delta_1) delta_1 delta_0))

(assert (<= 0 p_b))
(assert (<= p_b 1))
(assert (<= 0 p_d))
(assert (<= p_d 1))

; The probability domain is non-vacuous.
(check-sat)

; A binary-outcome Brier improvement cannot leave [-1,1].
(push 1)
(assert (or (< delta_0 (- 1)) (> delta_0 1)))
(check-sat)
(pop 1)

(push 1)
(assert (or (< delta_1 (- 1)) (> delta_1 1)))
(check-sat)
(pop 1)

; The constructed endpoint envelope is ordered and remains in [-1,1].
(push 1)
(assert (> lower upper))
(check-sat)
(pop 1)

(push 1)
(assert (or (< lower (- 1)) (> upper 1)))
(check-sat)
(pop 1)

; Each endpoint is attainable for p_b=1/5 and p_d=4/5, so a rowwise bound
; cannot be narrowed without additional restrictions on the missing outcome.
(push 1)
(assert (= p_b (/ 1 5)))
(assert (= p_d (/ 4 5)))
(assert (= y 0))
(assert (= delta_y lower))
(check-sat)
(pop 1)

(push 1)
(assert (= p_b (/ 1 5)))
(assert (= p_d (/ 4 5)))
(assert (= y 1))
(assert (= delta_y upper))
(check-sat)
(pop 1)

; For either binary completion, the realized row contribution is enclosed.
(push 1)
(assert (or (= y 0) (= y 1)))
(assert (or (< delta_y lower) (> delta_y upper)))
(check-sat)
(pop 1)

; A nonnegative weighted combination of two bounded blocks remains bounded.
; Repeatedly combining the accumulated block with the next row gives finite
; nonnegative-weight aggregation by induction; an unweighted finite mean is the
; special case in which every row has weight one.
(declare-const lower_a Real)
(declare-const value_a Real)
(declare-const upper_a Real)
(declare-const lower_b Real)
(declare-const value_b Real)
(declare-const upper_b Real)
(declare-const weight_a Real)
(declare-const weight_b Real)

; Non-vacuity: the convex-aggregation premise admits a strict ordinary witness.
(push 1)
(assert (< lower_a value_a))
(assert (< value_a upper_a))
(assert (< lower_b value_b))
(assert (< value_b upper_b))
(assert (> weight_a 0))
(assert (> weight_b 0))
(check-sat)
(pop 1)

(push 1)
(assert (<= lower_a value_a))
(assert (<= value_a upper_a))
(assert (<= lower_b value_b))
(assert (<= value_b upper_b))
(assert (>= weight_a 0))
(assert (>= weight_b 0))
(assert (> (+ weight_a weight_b) 0))
(assert
  (or
    (< (/ (+ (* weight_a value_a) (* weight_b value_b))
          (+ weight_a weight_b))
       (/ (+ (* weight_a lower_a) (* weight_b lower_b))
          (+ weight_a weight_b)))
    (> (/ (+ (* weight_a value_a) (* weight_b value_b))
          (+ weight_a weight_b))
       (/ (+ (* weight_a upper_a) (* weight_b upper_b))
          (+ weight_a weight_b)))))
(check-sat)
(pop 1)
