; Integer engineering admission only. No runtime allocation or RSS proof.
; N: complete rows; d: bytes per coordinate; A: matrix bytes; E: estimated bytes.
; Two coordinates, native float32 (d=4) or float64 (d=8).
; Ordered results: unsat, unsat, unsat, sat, sat, sat, sat.
(set-logic ALL)
(declare-const N Int)
(declare-const d Int)
(define-fun A () Int (* 2 N d))
(define-fun E () Int (+ (* 8 A) (* 32 N)))

; 1. Checked integer division is equivalent to the complete-matrix byte cap.
(push 1)
(assert (and (>= N 2) (or (= d 4) (= d 8))))
(assert (not (= (<= N (div 67108864 (* 2 d))) (<= A 67108864))))
(check-sat)
(pop 1)

; 2. Float32 admission implies N<=2^23 and E<=768 MiB.
(push 1)
(assert (and (>= N 2) (= d 4) (<= A 67108864)))
(assert (or (> N 8388608) (> E 805306368)))
(check-sat)
(pop 1)

; 3. Float64 admission implies N<=2^22 and E<=640 MiB.
(push 1)
(assert (and (>= N 2) (= d 8) (<= A 67108864)))
(assert (or (> N 4194304) (> E 671088640)))
(check-sat)
(pop 1)

; 4. Admitted float32 boundary attains both stated bounds.
(push 1)
(assert (and (= N 8388608) (= d 4) (= A 67108864) (= E 805306368)))
(assert (<= N (div 67108864 (* 2 d))))
(check-sat)
(pop 1)

; 5. One additional float32 row exceeds the cap by eight bytes.
(push 1)
(assert (and (= N 8388609) (= d 4) (= A 67108872)))
(assert (> N (div 67108864 (* 2 d))))
(check-sat)
(pop 1)

; 6. Admitted float64 boundary attains both stated bounds.
(push 1)
(assert (and (= N 4194304) (= d 8) (= A 67108864) (= E 671088640)))
(assert (<= N (div 67108864 (* 2 d))))
(check-sat)
(pop 1)

; 7. One additional float64 row exceeds the cap by sixteen bytes.
(push 1)
(assert (and (= N 4194305) (= d 8) (= A 67108880)))
(assert (> N (div 67108864 (* 2 d))))
(check-sat)
(pop 1)
