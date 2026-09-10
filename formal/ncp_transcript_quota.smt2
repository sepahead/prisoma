; Conditional byte arithmetic, not refinement of Python or the filesystem.
; Each frame has 1..65536 bytes. Each stored frame adds a 1-byte peer index
; and a 45-byte record envelope. One exchange contains two frames.
(set-logic QF_LIA)
(declare-const request_bytes Int)
(declare-const response_bytes Int)
(assert (and (<= 1 request_bytes) (<= request_bytes 65536)))
(assert (and (<= 1 response_bytes) (<= response_bytes 65536)))
(define-fun pair_bytes () Int (+ request_bytes response_bytes 92))
(push 1)
(assert (> pair_bytes 131164))
(check-sat) ; unsat: the two-frame bound
(pop 1)
(push 1)
(assert (= pair_bytes 131164))
(check-sat) ; sat: the upper bound is reachable
(pop 1)
(push 1)
(assert (> pair_bytes 131163))
(check-sat) ; sat: one byte less is a false bound
(pop 1)

; used is the persisted prefix length. remaining is the reserved pair count.
; terminal includes the terminal envelope and maximum terminal payload.
(declare-const used Int)
(declare-const remaining Int)
(declare-const quota Int)
(define-fun terminal () Int 1069)
(assert (and (<= 0 used) (<= 1 remaining) (<= remaining 8190)))
(assert (<= (+ used (* remaining 131164) terminal) quota))
(define-fun next_reserved () Int
  (+ used pair_bytes (* (- remaining 1) 131164) terminal))
(push 1)
(assert (> next_reserved quota))
(check-sat) ; unsat: each admitted pair preserves the remaining quota
(pop 1)
(push 1)
(assert (and (= remaining 1) (= pair_bytes 131164)
             (= quota (+ used 131164 terminal))))
(check-sat) ; sat: exact-capacity execution remains feasible
(pop 1)
(push 1)
; Omitting the response from reservation is intentionally false.
(assert (> pair_bytes (+ request_bytes 46)))
(check-sat) ; sat: a request-only reservation cannot cover both frames
(pop 1)
