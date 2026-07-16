; Concrete algebraic countermodel: the three two-source consistency equations
; and nonnegative atoms do not identify a unique decomposition. Both locally
; positive decompositions have I1=I2=1 and I12=3/2, but their redundancy and
; synergy differ. This does not assert that both arise from one globally defined
; redundancy functional on concrete probability laws.
(set-logic QF_LRA)
(declare-const red_a Real)
(declare-const unique1_a Real)
(declare-const unique2_a Real)
(declare-const synergy_a Real)
(declare-const red_b Real)
(declare-const unique1_b Real)
(declare-const unique2_b Real)
(declare-const synergy_b Real)

(define-fun decomposition ((r Real) (u1 Real) (u2 Real) (s Real)) Bool
  (and (>= r 0.0) (>= u1 0.0) (>= u2 0.0) (>= s 0.0)
       (= (+ r u1) 1.0)
       (= (+ r u2) 1.0)
       (= (+ r u1 u2 s) (/ 3.0 2.0))))

; Witness A: (R,U1,U2,S) = (1/2,1/2,1/2,0).
(assert (= red_a (/ 1.0 2.0)))
(assert (= unique1_a (/ 1.0 2.0)))
(assert (= unique2_a (/ 1.0 2.0)))
(assert (= synergy_a 0.0))

; Witness B: (R,U1,U2,S) = (3/4,1/4,1/4,1/4).
(assert (= red_b (/ 3.0 4.0)))
(assert (= unique1_b (/ 1.0 4.0)))
(assert (= unique2_b (/ 1.0 4.0)))
(assert (= synergy_b (/ 1.0 4.0)))

(assert (decomposition red_a unique1_a unique2_a synergy_a))
(assert (decomposition red_b unique1_b unique2_b synergy_b))
(assert (and (not (= red_a red_b)) (not (= synergy_a synergy_b))))
(check-sat)
