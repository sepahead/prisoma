; Conditional integer arithmetic for the installed sensor-session contract.
; Every universal counterexample check has a feasible positive control.
; This model does not prove the implementation, renderer, timing, or statistics.
(set-logic QF_LIA)
(declare-const bytes Int)
(declare-const k Int)
(declare-const previous_total Int)
(declare-const chunks Int)
(declare-const sensors Int)
(declare-const calls Int)
(define-fun ceil_chunks ((n Int)) Int (div (+ n 32767) 32768))
(define-fun sample_start ((tick Int)) Int (div (* (- tick 1) 16000) 120))
(define-fun sample_end ((tick Int)) Int (div (* tick 16000) 120))
(define-fun samples ((tick Int)) Int (- (sample_end tick) (sample_start tick)))

; A positive byte length has exactly enough fixed-size chunks.
(push 1)
(assert (and (> bytes 0) (<= bytes 27857088)))
(push 1)
(assert (not (and (> (ceil_chunks bytes) 0)
                 (< (* (- (ceil_chunks bytes) 1) 32768) bytes)
                 (<= bytes (* (ceil_chunks bytes) 32768)))))
(check-sat) ; unsat
(pop 1)
(assert (= bytes 307200))
(assert (= (ceil_chunks bytes) 10))
(check-sat) ; sat
(pop 1)

; Exact 16-kHz windows on the installed 120-Hz body clock.
(push 1)
(assert (and (>= k 1) (<= k 1022)))
(push 1)
(assert (not (or (= (samples k) 133) (= (samples k) 134))))
(check-sat) ; unsat
(pop 1)
(push 1)
(assert (= k 3))
(assert (= (samples k) 134))
(check-sat) ; sat
(pop 1)
; Negative control: a constant 133-sample window is a false bound.
(assert (not (= (samples k) 133)))
(check-sat) ; sat
(pop 1)

; Inductive accumulation: the next window reaches the next exact boundary.
(push 1)
(assert (and (>= k 1) (<= k 1022) (= previous_total (sample_start k))))
(push 1)
(assert (not (= (+ previous_total (samples k)) (sample_end k))))
(check-sat) ; unsat
(pop 1)
(assert (= k 6))
(assert (= (+ previous_total (samples k)) 800))
(check-sat) ; sat
(pop 1)

; Adjacent windows neither leave a gap nor overlap in sample indices.
(push 1)
(assert (and (>= k 1) (< k 1022)))
(push 1)
(assert (not (= (sample_end k) (sample_start (+ k 1)))))
(check-sat) ; unsat
(pop 1)
(assert (= k 2))
(assert (= (sample_end k) 266))
(check-sat) ; sat
(pop 1)

; Advance/ACK + reads/ACKs + releases/ACKs under admitted batch bounds.
(push 1)
(assert (and (>= chunks 0) (<= chunks 856) (>= sensors 0) (<= sensors 12)))
(define-fun exchanges () Int (+ 2 (* 2 chunks) (* 2 sensors)))
(push 1)
(assert (not (and (>= exchanges 2) (<= exchanges 1738))))
(check-sat) ; unsat
(pop 1)
(push 1)
(assert (and (= chunks 21) (= sensors 3) (= exchanges 50)))
(check-sat) ; sat
(pop 1)
; Negative control: the largest native-example tick needs more than 49.
(assert (and (= chunks 21) (= sensors 3) (> exchanges 49)))
(check-sat) ; sat
(pop 1)

; The frozen six-tick example: two terminal operations plus all tick work.
(push 1)
(assert (not (= (+ 4 (* 2 6) (* 2 56) (* 2 11)) 150)))
(check-sat) ; unsat
(pop 1)
(push 1)
(assert (= (+ 4 (* 2 6) (* 2 56) (* 2 11)) 150))
(check-sat) ; sat
(pop 1)

; Two prefix events, three events per call, artifact, and run end.
(push 1)
(assert (and (>= calls 1) (<= calls 1024)))
(push 1)
(assert (not (and (>= (+ (* 3 calls) 4) 7) (<= (+ (* 3 calls) 4) 3076))))
(check-sat) ; unsat
(pop 1)
(push 1)
(assert (and (= calls 8) (= (+ (* 3 calls) 4) 28)))
(check-sat) ; sat
(pop 1)
; Negative control: three extra slots omit an artifact or terminal event.
(assert (> (+ (* 3 calls) 4) (+ (* 3 calls) 3)))
(check-sat) ; sat
(pop 1)
