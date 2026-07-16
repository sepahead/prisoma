; In exact real arithmetic, the two-source definitions imply r_bar + v_bar = 2.
; The positive joint-MI premise matches the defined mathematical domain; the
; implementation additionally applies a small-denominator policy and floating-
; point representability checks that this model does not cover.
(set-logic QF_NRA)
(declare-const i1 Real)
(declare-const i2 Real)
(declare-const joint Real)
(assert (>= i1 0.0))
(assert (>= i2 0.0))
(assert (> joint 0.0))
(assert (<= i1 joint))
(assert (<= i2 joint))
(define-fun r_bar () Real (/ (+ i1 i2) joint))
(define-fun v_bar () Real (/ (+ (- joint i2) (- joint i1)) joint))

; Non-vacuity: ordinary positive-MI inputs satisfy the premises.
(push 1)
(assert (= i1 (/ 1.0 4.0)))
(assert (= i2 (/ 1.0 2.0)))
(assert (= joint (/ 3.0 4.0)))
(check-sat)
(pop 1)

(push 1)
(assert (not (= (+ r_bar v_bar) 2.0)))
(check-sat)
(pop 1)

; The same information-domain assumptions bound both normalized degrees.
(push 1)
(assert (or (< r_bar 0.0) (> r_bar 2.0)))
(check-sat)
(pop 1)

(push 1)
(assert (or (< v_bar 0.0) (> v_bar 2.0)))
(check-sat)
(pop 1)
