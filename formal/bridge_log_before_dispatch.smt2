; Prove the ordering guaranteed by pid_bridge::LocalBridge::dispatch.
; `request_accepted` means RunLogWriter::append returned success to LocalBridge;
; it does not mean flush or fsync. `request_failed` is the fail-closed branch in
; which that append returned an error. response_kind is 0=none,
; 1=handler-result, 2=safe-mode rejection.
;
; op: 0=request append accepted, 1=request append failed, 2=handler called,
;     3=handler response accepted, 4=safe-mode response accepted,
;     5=stutter/later writer failure.
(set-logic QF_LIA)

(define-fun valid-response-kind ((kind Int)) Bool
  (and (<= 0 kind) (<= kind 2)))

(define-fun invariant (
    (request_accepted Bool) (request_failed Bool) (safe_mode Bool)
    (handler_called Bool) (response_kind Int)) Bool
  (and
    (valid-response-kind response_kind)
    (not (and request_accepted request_failed))
    (=> request_failed (and (not handler_called) (= response_kind 0)))
    (=> handler_called (and request_accepted (not safe_mode)))
    (=> (not (= response_kind 0)) request_accepted)
    (=> (= response_kind 1) (and handler_called (not safe_mode)))
    (=> (= response_kind 2) (and safe_mode (not handler_called)))))

(define-fun transition (
    (request_accepted Bool) (request_failed Bool) (safe_mode Bool)
    (handler_called Bool) (response_kind Int)
    (request_accepted_next Bool) (request_failed_next Bool)
    (safe_mode_next Bool) (handler_called_next Bool)
    (response_kind_next Int) (op Int)) Bool
  (or
    (and (= op 0) (not request_accepted) (not request_failed)
         request_accepted_next (= request_failed_next request_failed)
         (= safe_mode_next safe_mode)
         (= handler_called_next handler_called)
         (= response_kind_next response_kind))
    (and (= op 1) (not request_accepted) (not request_failed)
         (= request_accepted_next request_accepted) request_failed_next
         (= safe_mode_next safe_mode)
         (= handler_called_next handler_called)
         (= response_kind_next response_kind))
    (and (= op 2) request_accepted (not safe_mode) (not handler_called)
         (= response_kind 0) request_accepted_next
         (= request_failed_next request_failed) (= safe_mode_next safe_mode)
         handler_called_next (= response_kind_next response_kind))
    (and (= op 3) request_accepted (not safe_mode) handler_called
         (= response_kind 0) request_accepted_next
         (= request_failed_next request_failed) (= safe_mode_next safe_mode)
         handler_called_next (= response_kind_next 1))
    (and (= op 4) request_accepted safe_mode (not handler_called)
         (= response_kind 0) request_accepted_next
         (= request_failed_next request_failed) (= safe_mode_next safe_mode)
         (= handler_called_next handler_called) (= response_kind_next 2))
    (and (= op 5) (= request_accepted_next request_accepted)
         (= request_failed_next request_failed) (= safe_mode_next safe_mode)
         (= handler_called_next handler_called)
         (= response_kind_next response_kind))))

; The initial state satisfies the invariant for either fixed mode.
(push 1)
(declare-const safe_mode_0 Bool)
(assert (not (invariant false false safe_mode_0 false 0)))
(check-sat)
(pop 1)

; Every modeled step preserves the invariant.
(push 1)
(declare-const request_accepted Bool)
(declare-const request_failed Bool)
(declare-const safe_mode Bool)
(declare-const handler_called Bool)
(declare-const response_kind Int)
(declare-const request_accepted_next Bool)
(declare-const request_failed_next Bool)
(declare-const safe_mode_next Bool)
(declare-const handler_called_next Bool)
(declare-const response_kind_next Int)
(declare-const op Int)
(assert (invariant request_accepted request_failed safe_mode
                   handler_called response_kind))
(assert (transition request_accepted request_failed safe_mode
                    handler_called response_kind request_accepted_next
                    request_failed_next safe_mode_next handler_called_next
                    response_kind_next op))
(assert (not (invariant request_accepted_next request_failed_next safe_mode_next
                        handler_called_next response_kind_next)))
(check-sat)
(pop 1)

; Non-vacuity: ordinary request -> handler -> handler response.
(push 1)
(assert (transition false false false false 0 true false false false 0 0))
(assert (transition true false false false 0 true false false true 0 2))
(assert (transition true false false true 0 true false false true 1 3))
(check-sat)
(pop 1)

; Non-vacuity: safe mode rejects after logging, without a handler call.
(push 1)
(assert (transition false false true false 0 true false true false 0 0))
(assert (transition true false true false 0 true false true false 2 4))
(check-sat)
(pop 1)

; Non-vacuity: a failed request append reaches an explicit terminal branch.
(push 1)
(assert (transition false false false false 0 false true false false 0 1))
(check-sat)
(pop 1)

; A failed request append cannot be followed by handler dispatch.
(push 1)
(assert (transition false true false false 0 false true false true 0 2))
(check-sat)
(pop 1)
