; Abstract the validated OfflineVldaOutcome publication boundary.
; status: 0=not_requested, 1=produced, 2=produced_with_warning, 3=abstained.
; `has_numeric_payload` means a complete finite scalar/vector payload has already
; passed the Rust shape and finiteness checks; those floating-point checks are
; outside this Boolean model.
(set-logic QF_LIA)
(declare-const status Int)
(declare-const has_numeric_payload Bool)
(declare-const has_abstention_code Bool)
(declare-const has_nonempty_detail Bool)
(declare-const abstention_code_matches_gate_code Bool)
(declare-const all_gates_not_applicable Bool)
(declare-const interpretation_allowed Bool)
(declare-const all_gates_passed Bool)
(declare-const has_support_envelope Bool)
(declare-const published Bool)

(define-fun valid-status ((s Int)) Bool (and (<= 0 s) (<= s 3)))
(define-fun produced-status ((s Int)) Bool (or (= s 1) (= s 2)))
(define-fun publication-contract (
    (s Int) (has_value Bool) (has_code Bool) (has_detail Bool)
    (code_matches Bool) (gates_na Bool) (interpret Bool)
    (gates_pass Bool) (has_envelope Bool)) Bool
  (and
    (valid-status s)
    (= has_value (produced-status s))
    (=> (= s 0) (and (not has_code) has_detail gates_na (not interpret)))
    (=> (= s 1) (and (not has_code) (not has_detail)))
    (=> (= s 2) (and (not has_code) has_detail (not interpret)))
    (=> (= s 3) (and has_code has_detail code_matches (not interpret)))
    (=> interpret (and gates_pass has_envelope))))

(assert
  (=> published
      (publication-contract status has_numeric_payload has_abstention_code
                            has_nonempty_detail abstention_code_matches_gate_code
                            all_gates_not_applicable interpretation_allowed
                            all_gates_passed has_support_envelope)))

; Every tag is satisfiable under its required payload/provenance contract.
(push)
(assert published)
(assert (= status 0))
(check-sat)
(pop)

(push)
(assert published)
(assert (= status 1))
(check-sat)
(pop)

(push)
(assert published)
(assert (= status 2))
(check-sat)
(pop)

(push)
(assert published)
(assert (= status 3))
(check-sat)
(pop)

; No unrecognized tag can cross the publication boundary.
(push)
(assert published)
(assert (or (< status 0) (> status 3)))
(check-sat)
(pop)

; Numeric presence is exact for each tagged computation status.
(push)
(assert published)
(assert (= status 0))
(assert has_numeric_payload)
(check-sat)
(pop)

(push)
(assert published)
(assert (= status 3))
(assert has_numeric_payload)
(check-sat)
(pop)

(push)
(assert published)
(assert (= status 1))
(assert (not has_numeric_payload))
(check-sat)
(pop)

(push)
(assert published)
(assert (= status 2))
(assert (not has_numeric_payload))
(check-sat)
(pop)

; Warning/abstention provenance cannot be omitted or mismatched.
(push)
(assert published)
(assert (= status 2))
(assert (not has_nonempty_detail))
(check-sat)
(pop)

(push)
(assert published)
(assert (= status 3))
(assert (or (not has_abstention_code) (not has_nonempty_detail)
            (not abstention_code_matches_gate_code)))
(check-sat)
(pop)

; Interpretation permission requires all four gates and a support envelope.
(push)
(assert published)
(assert interpretation_allowed)
(assert (or (not all_gates_passed) (not has_support_envelope)))
(check-sat)
(pop)
