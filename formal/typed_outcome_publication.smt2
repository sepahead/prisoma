; Abstract the validated OfflineVldaOutcome publication boundary.
; status: 0=not_requested, 1=produced, 2=produced_with_warning, 3=abstained.
; gate state follows the Rust declaration order:
; 0=passed, 1=conditional, 2=not_evaluated, 3=blocked, 4=not_applicable.
; `has_numeric_payload` means a complete finite scalar/vector payload has already
; passed the Rust shape and finiteness checks; those floating-point checks are
; outside this integer/Boolean model. pid_metric_event_count models only numeric
; PID metric events, not diagnostic or provenance events.
(set-logic QF_LIA)
(declare-const status Int)
(declare-const population_gate Int)
(declare-const measure_gate Int)
(declare-const estimator_gate Int)
(declare-const application_gate Int)
(declare-const has_numeric_payload Bool)
(declare-const pid_metric_event_count Int)
(declare-const has_abstention_code Bool)
(declare-const has_nonempty_detail Bool)
(declare-const abstention_code_matches_gate_code Bool)
(declare-const interpretation_allowed Bool)
(declare-const has_support_envelope Bool)
(declare-const published Bool)

(define-fun valid-status ((s Int)) Bool (and (<= 0 s) (<= s 3)))
(define-fun valid-gate ((g Int)) Bool (and (<= 0 g) (<= g 4)))
(define-fun produced-status ((s Int)) Bool (or (= s 1) (= s 2)))
(define-fun all-gates-equal ((value Int)) Bool
  (and (= population_gate value) (= measure_gate value)
       (= estimator_gate value) (= application_gate value)))
(define-fun publication-contract (
    (s Int) (has_value Bool) (metric_count Int) (has_code Bool)
    (has_detail Bool) (code_matches Bool) (interpret Bool)
    (has_envelope Bool)) Bool
  (and
    (valid-status s)
    (valid-gate population_gate)
    (valid-gate measure_gate)
    (valid-gate estimator_gate)
    (valid-gate application_gate)
    (>= metric_count 0)
    (= has_value (> metric_count 0))
    (= has_value (produced-status s))
    (=> (= s 0)
        (and (not has_code) has_detail (all-gates-equal 4) (not interpret)))
    (=> (= s 1) (and (not has_code) (not has_detail)))
    (=> (= s 2) (and (not has_code) has_detail (not interpret)))
    (=> (= s 3) (and has_code has_detail code_matches (not interpret)))
    (=> interpret (and (all-gates-equal 0) has_envelope))))

(assert
  (=> published
      (publication-contract status has_numeric_payload pid_metric_event_count
                            has_abstention_code has_nonempty_detail
                            abstention_code_matches_gate_code
                            interpretation_allowed has_support_envelope)))

; Every tag is satisfiable under its required payload/provenance contract.
(push 1)
(assert published)
(assert (= status 0))
(check-sat)
(pop 1)

(push 1)
(assert published)
(assert (= status 1))
(check-sat)
(pop 1)

(push 1)
(assert published)
(assert (= status 2))
(check-sat)
(pop 1)

(push 1)
(assert published)
(assert (= status 3))
(check-sat)
(pop 1)

; Every Rust gate verdict is representable at the publication boundary.
(push 1)
(assert published)
(assert (= status 1))
(assert (= population_gate 0))
(check-sat)
(pop 1)

(push 1)
(assert published)
(assert (= status 1))
(assert (= population_gate 1))
(check-sat)
(pop 1)

(push 1)
(assert published)
(assert (= status 1))
(assert (= population_gate 2))
(check-sat)
(pop 1)

(push 1)
(assert published)
(assert (= status 1))
(assert (= population_gate 3))
(check-sat)
(pop 1)

(push 1)
(assert published)
(assert (= status 1))
(assert (= population_gate 4))
(check-sat)
(pop 1)

; Non-vacuity: an interpretable produced outcome is reachable.
(push 1)
(assert published)
(assert (= status 1))
(assert interpretation_allowed)
(check-sat)
(pop 1)

; No unrecognized status or gate tag can cross the publication boundary.
(push 1)
(assert published)
(assert (or (< status 0) (> status 3)))
(check-sat)
(pop 1)

(push 1)
(assert published)
(assert (or (< population_gate 0) (> population_gate 4)
            (< measure_gate 0) (> measure_gate 4)
            (< estimator_gate 0) (> estimator_gate 4)
            (< application_gate 0) (> application_gate 4)))
(check-sat)
(pop 1)

; Numeric presence is exact for each tagged computation status.
(push 1)
(assert published)
(assert (= status 0))
(assert has_numeric_payload)
(check-sat)
(pop 1)

(push 1)
(assert published)
(assert (= status 3))
(assert has_numeric_payload)
(check-sat)
(pop 1)

(push 1)
(assert published)
(assert (= status 1))
(assert (not has_numeric_payload))
(check-sat)
(pop 1)

(push 1)
(assert published)
(assert (= status 2))
(assert (not has_numeric_payload))
(check-sat)
(pop 1)

; Not-requested and abstained outcomes emit no numeric PID metric event.
(push 1)
(assert published)
(assert (or (= status 0) (= status 3)))
(assert (> pid_metric_event_count 0))
(check-sat)
(pop 1)

; Warning/abstention provenance cannot be omitted or mismatched.
(push 1)
(assert published)
(assert (= status 2))
(assert (not has_nonempty_detail))
(check-sat)
(pop 1)

(push 1)
(assert published)
(assert (= status 3))
(assert (or (not has_abstention_code) (not has_nonempty_detail)
            (not abstention_code_matches_gate_code)))
(check-sat)
(pop 1)

; Interpretation permission requires all four gates passed and an envelope.
(push 1)
(assert published)
(assert interpretation_allowed)
(assert (or (not (all-gates-equal 0)) (not has_support_envelope)))
(check-sat)
(pop 1)
