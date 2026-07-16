; H3 scores a PID-augmented policy over the complete frozen target ledger. When
; the PID path abstains, the deployed output is the exact same-fold baseline
; output. For any nonnegative target weight, that row's paired loss improvement
; is therefore exactly zero. The satisfiable counterexample shows why averaging
; only over PID-produced rows changes the estimand: one produced improvement of
; one and one fallback row yield a produced-only mean of one but a full-ledger
; mean of one half.
(set-logic QF_NRA)

; Exact fallback implies zero weighted paired improvement.
(push 1)
(declare-const premise_baseline_loss Real)
(declare-const premise_deployed_loss Real)
(declare-const premise_target_weight Real)
(assert (>= premise_target_weight 0.0))
(assert (= premise_deployed_loss premise_baseline_loss))
(check-sat)
(pop 1)

(push 1)
(declare-const baseline_loss Real)
(declare-const deployed_loss Real)
(declare-const target_weight Real)
(assert (>= target_weight 0.0))
(assert (= deployed_loss baseline_loss))
(assert (not (= (* target_weight (- baseline_loss deployed_loss)) 0.0)))
(check-sat)
(pop 1)

; Produced-only and full-ledger averages can differ even though fallback rows
; contribute exactly zero to the numerator.
(push 1)
(declare-const produced_weight Real)
(declare-const fallback_weight Real)
(declare-const produced_improvement Real)
(declare-const fallback_improvement Real)
(declare-const produced_only_mean Real)
(declare-const full_ledger_mean Real)
(assert (= produced_weight 1.0))
(assert (= fallback_weight 1.0))
(assert (= produced_improvement 1.0))
(assert (= fallback_improvement 0.0))
(assert (= produced_only_mean
           (/ (* produced_weight produced_improvement) produced_weight)))
(assert (= full_ledger_mean
           (/ (+ (* produced_weight produced_improvement)
                 (* fallback_weight fallback_improvement))
              (+ produced_weight fallback_weight))))
(assert (not (= produced_only_mean full_ledger_mean)))
(check-sat)
(pop 1)
