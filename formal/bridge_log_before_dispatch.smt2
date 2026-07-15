; Prove the ordering guaranteed by pid_bridge::LocalBridge::dispatch.
; `request_accepted` means RunLogWriter::append returned success to LocalBridge;
; it does not mean flush or fsync. A handler call and an accepted response append
; each require an accepted request append. A safe-mode rejection deliberately
; permits a response without calling the handler.
;
; op: 0=request append accepted, 1=handler called,
;     2=response append accepted, 3=stutter/failure.
(set-logic QF_LIA)

(define-fun invariant (
    (request_accepted Bool) (handler_called Bool) (response_accepted Bool)) Bool
  (and (=> handler_called request_accepted)
       (=> response_accepted request_accepted)))

(define-fun transition (
    (request_accepted Bool) (handler_called Bool) (response_accepted Bool)
    (request_accepted_next Bool) (handler_called_next Bool)
    (response_accepted_next Bool) (op Int)) Bool
  (or
    (and (= op 0) (not request_accepted) request_accepted_next
         (= handler_called_next handler_called)
         (= response_accepted_next response_accepted))
    (and (= op 1) request_accepted (not handler_called) (not response_accepted)
         request_accepted_next handler_called_next
         (= response_accepted_next response_accepted))
    (and (= op 2) request_accepted (not response_accepted)
         request_accepted_next (= handler_called_next handler_called)
         response_accepted_next)
    (and (= op 3) (= request_accepted_next request_accepted)
         (= handler_called_next handler_called)
         (= response_accepted_next response_accepted))))

; The initial state satisfies the invariant.
(push)
(declare-const request_accepted_0 Bool)
(declare-const handler_called_0 Bool)
(declare-const response_accepted_0 Bool)
(assert (and (not request_accepted_0) (not handler_called_0)
             (not response_accepted_0)))
(assert (not (invariant request_accepted_0 handler_called_0 response_accepted_0)))
(check-sat)
(pop)

; Every modeled step preserves it.
(push)
(declare-const request_accepted Bool)
(declare-const handler_called Bool)
(declare-const response_accepted Bool)
(declare-const request_accepted_next Bool)
(declare-const handler_called_next Bool)
(declare-const response_accepted_next Bool)
(declare-const op Int)
(assert (invariant request_accepted handler_called response_accepted))
(assert (transition request_accepted handler_called response_accepted
                    request_accepted_next handler_called_next
                    response_accepted_next op))
(assert (not (invariant request_accepted_next handler_called_next
                        response_accepted_next)))
(check-sat)
(pop)

; Non-vacuity: the normal request -> handler -> response path is reachable.
(push)
(assert (transition false false false true false false 0))
(assert (transition true false false true true false 1))
(assert (transition true true false true true true 2))
(check-sat)
(pop)

; Non-vacuity: the safe-mode request -> response path is reachable without a handler call.
(push)
(assert (transition false false false true false false 0))
(assert (transition true false false true false true 2))
(check-sat)
(pop)
