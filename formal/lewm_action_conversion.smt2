; LeWM action conversion: exact integer packing and ideal real affine maps.
; Indices are zero-based. Raw primitives k=0..24 have coordinates j=0..1;
; model blocks b=0..4 have slots q=0..9. Candidate identity is unchanged.
; Real u is a dimensionless raw coordinate, m its mean, and s a positive scale.
; a and b in the zero-scale control denote two distinct model coordinates.
; Each numbered obligation has an independent assertion/declaration scope.
; These are elementary formulas, not IEEE-754 or executable refinement proofs.
; They establish no empirical training support, model quality, or execution authority.
(set-logic ALL)

; 01. raw-to-block-roundtrip: expected unsat.
; Raw-to-block packing and its inverse preserve every valid primitive coordinate.
(push 1)
(declare-const k Int) (declare-const j Int)
(assert (and (<= 0 k) (< k 25) (<= 0 j) (< j 2)))
(define-fun b () Int (div k 5))
(define-fun q () Int (+ (* 2 (mod k 5)) j))
(assert (not (and (<= 0 b) (< b 5) (<= 0 q) (< q 10)
  (= (+ (* 5 b) (div q 2)) k) (= (mod q 2) j))))
(check-sat)
(pop 1)

; 02. block-to-raw-roundtrip: expected unsat.
; Block-to-raw unpacking and its inverse cover every valid model slot.
(push 1)
(declare-const b Int) (declare-const q Int)
(assert (and (<= 0 b) (< b 5) (<= 0 q) (< q 10)))
(define-fun k () Int (+ (* 5 b) (div q 2)))
(define-fun j () Int (mod q 2))
(assert (not (and (<= 0 k) (< k 25) (<= 0 j) (< j 2)
  (= (div k 5) b) (= (+ (* 2 (mod k 5)) j) q))))
(check-sat)
(pop 1)

; 03. wrong-stride-collision: expected sat.
; Wrong stride: removing the factor two admits a collision between distinct pairs.
(push 1)
(declare-const k Int) (declare-const j Int)
(declare-const p Int) (declare-const h Int)
(assert (and (<= 0 k) (< k 25) (<= 0 j) (< j 2)
  (<= 0 p) (< p 25) (<= 0 h) (< h 2)))
(assert (or (not (= k p)) (not (= j h))))
(assert (= (div k 5) (div p 5)))
(assert (= (+ (mod k 5) j) (+ (mod p 5) h)))
(check-sat)
(pop 1)

; 04. packing-positive-witness: expected sat.
; Valid packing witness: the final primitive coordinate maps to the final slot.
(push 1)
(declare-const k Int) (declare-const j Int)
(assert (and (= k 24) (= j 1)))
(assert (and (= (div k 5) 4) (= (+ (* 2 (mod k 5)) j) 9)))
(check-sat)
(pop 1)

; 05. positive-scale-box-equivalence: expected unsat.
; Positive real scale maps the raw interval exactly to its affine image.
(push 1)
(declare-const u Real) (declare-const m Real) (declare-const s Real)
(assert (> s 0))
(define-fun a () Real (/ (- u m) s))
(define-fun lower () Real (/ (- (- 1) m) s))
(define-fun upper () Real (/ (- 1 m) s))
(assert (not (= (and (<= (- 1) u) (<= u 1)) (and (<= lower a) (<= a upper)))))
(check-sat)
(pop 1)

; 06. drop-positive-scale-counterexample: expected sat.
; Negative scale invalidates the same endpoint order: a concrete counterexample.
(push 1)
(declare-const u Real) (declare-const m Real) (declare-const s Real)
(assert (and (= u 0) (= m 0) (= s (- 1))))
(define-fun a () Real (/ (- u m) s))
(assert (not (= (and (<= (- 1) u) (<= u 1))
  (and (<= (/ (- (- 1) m) s) a) (<= a (/ (- 1 m) s))))))
(check-sat)
(pop 1)

; 07. real-affine-inverse: expected unsat.
; Positive real scale makes affine standardization and reconstruction inverses.
(push 1)
(declare-const u Real) (declare-const m Real) (declare-const s Real)
(assert (> s 0))
(assert (not (= (+ (* (/ (- u m) s) s) m) u)))
(check-sat)
(pop 1)

; 08. zero-scale-inverse-counterexample: expected sat.
; Zero scale collapses distinct model coordinates; no division by zero is used.
(push 1)
(declare-const a Real) (declare-const b Real) (declare-const m Real) (declare-const s Real)
(assert (and (= s 0) (= m 0) (= a 0) (= b 1)))
(assert (not (= a b)))
(assert (= (+ (* s a) m) (+ (* s b) m)))
(check-sat)
(pop 1)

; 09. affine-positive-witness: expected sat.
; A concrete positive-scale interior value satisfies the affine premises and inverse.
(push 1)
(declare-const u Real) (declare-const m Real) (declare-const s Real)
(assert (and (= u (/ 1 2)) (= m (/ 1 4)) (= s (/ 1 2))))
(assert (and (> s 0) (<= (- 1) u) (<= u 1)
  (= (/ (- u m) s) (/ 1 2)) (= (+ (* (/ (- u m) s) s) m) u)))
(check-sat)
(pop 1)
