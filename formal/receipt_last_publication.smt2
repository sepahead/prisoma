; Two receipt-last abstractions for the NCP observer.
;
; The core finalizer constructs bounded dataset/run-log buffers and a receipt
; binding their digests, then installs dataset, run log, and receipt in order.
; For a new destination it does not reread the installed dataset/run-log bytes.
; The outer observatory additionally verifies an installed snapshot before the
; receipt write and rereads/verifies the complete snapshot after that write.
;
; These states denote ordered code events, not permanent storage,
; adversarial-race resistance, or a cryptographic proof about the filesystem.
; The uninterpreted digest function proves exact field binding structurally; it
; does not assert collision resistance or model SHA-256 computation.
(set-logic QF_UFLIA)
(declare-sort Bytes 0)
(declare-sort Digest 0)
(declare-fun digest (Bytes) Digest)

(declare-const prepared_dataset_bytes Bytes)
(declare-const prepared_runlog_bytes Bytes)
(declare-const core_receipt_dataset_digest Digest)
(declare-const core_receipt_runlog_digest Digest)
(define-fun core-receipt-binding () Bool
  (and (= core_receipt_dataset_digest (digest prepared_dataset_bytes))
       (= core_receipt_runlog_digest (digest prepared_runlog_bytes))))

(declare-const installed_dataset_bytes Bytes)
(declare-const installed_runlog_bytes Bytes)
(declare-const reread_dataset_bytes Bytes)
(declare-const reread_runlog_bytes Bytes)
(declare-const outer_receipt_dataset_digest Digest)
(declare-const outer_receipt_runlog_digest Digest)
(define-fun outer-receipt-binding () Bool
  (and (= outer_receipt_dataset_digest (digest installed_dataset_bytes))
       (= outer_receipt_runlog_digest (digest installed_runlog_bytes))))
(define-fun outer-postcommit-binding () Bool
  (and outer-receipt-binding
       (= reread_dataset_bytes installed_dataset_bytes)
       (= reread_runlog_bytes installed_runlog_bytes)
       (= outer_receipt_dataset_digest (digest reread_dataset_bytes))
       (= outer_receipt_runlog_digest (digest reread_runlog_bytes))))

; ----- Core Observer::finalize -----

(define-fun core-invariant (
    (dataset_ready Bool) (runlog_ready Bool) (receipt_bound Bool)
    (dataset_step_confirmed Bool) (runlog_step_confirmed Bool)
    (receipt_path_visible Bool) (receipt_step_confirmed Bool)
    (finalize_returned Bool)) Bool
  (and (=> runlog_ready dataset_ready)
       (=> receipt_bound
           (and dataset_ready runlog_ready core-receipt-binding))
       (=> dataset_step_confirmed dataset_ready)
       (=> runlog_step_confirmed (and dataset_step_confirmed runlog_ready))
       (=> receipt_path_visible
           (and dataset_step_confirmed runlog_step_confirmed receipt_bound))
       (=> receipt_step_confirmed receipt_path_visible)
       (=> finalize_returned receipt_step_confirmed)))

; op: 0=prepare dataset buffer, 1=prepare run-log buffer,
;     2=construct receipt from prepared-buffer digests,
;     3=dataset write/recovery step confirms, 4=run-log step confirms,
;     5=receipt path becomes visible, 6=receipt write/recovery step confirms,
;     7=finalize returns success, 8=stutter/failure.
;
; Splitting ops 5 and 6 represents the real post-install-fsync failure window:
; a receipt path can be visible before the write call returns success. An exact
; retry may then reread that receipt and re-establish its fsync state.
(define-fun core-transition (
    (dataset_ready Bool) (runlog_ready Bool) (receipt_bound Bool)
    (dataset_step_confirmed Bool) (runlog_step_confirmed Bool)
    (receipt_path_visible Bool) (receipt_step_confirmed Bool)
    (finalize_returned Bool)
    (dataset_ready_next Bool) (runlog_ready_next Bool) (receipt_bound_next Bool)
    (dataset_step_confirmed_next Bool) (runlog_step_confirmed_next Bool)
    (receipt_path_visible_next Bool) (receipt_step_confirmed_next Bool)
    (finalize_returned_next Bool) (op Int)) Bool
  (or
    (and (= op 0) (not dataset_ready) dataset_ready_next
         (= runlog_ready_next runlog_ready) (= receipt_bound_next receipt_bound)
         (= dataset_step_confirmed_next dataset_step_confirmed)
         (= runlog_step_confirmed_next runlog_step_confirmed)
         (= receipt_path_visible_next receipt_path_visible)
         (= receipt_step_confirmed_next receipt_step_confirmed)
         (= finalize_returned_next finalize_returned))
    (and (= op 1) dataset_ready (not runlog_ready)
         dataset_ready_next runlog_ready_next
         (= receipt_bound_next receipt_bound)
         (= dataset_step_confirmed_next dataset_step_confirmed)
         (= runlog_step_confirmed_next runlog_step_confirmed)
         (= receipt_path_visible_next receipt_path_visible)
         (= receipt_step_confirmed_next receipt_step_confirmed)
         (= finalize_returned_next finalize_returned))
    (and (= op 2) dataset_ready runlog_ready (not receipt_bound)
         core-receipt-binding
         dataset_ready_next runlog_ready_next receipt_bound_next
         (= dataset_step_confirmed_next dataset_step_confirmed)
         (= runlog_step_confirmed_next runlog_step_confirmed)
         (= receipt_path_visible_next receipt_path_visible)
         (= receipt_step_confirmed_next receipt_step_confirmed)
         (= finalize_returned_next finalize_returned))
    (and (= op 3) dataset_ready (not dataset_step_confirmed)
         dataset_ready_next (= runlog_ready_next runlog_ready)
         (= receipt_bound_next receipt_bound) dataset_step_confirmed_next
         (= runlog_step_confirmed_next runlog_step_confirmed)
         (= receipt_path_visible_next receipt_path_visible)
         (= receipt_step_confirmed_next receipt_step_confirmed)
         (= finalize_returned_next finalize_returned))
    (and (= op 4) dataset_step_confirmed runlog_ready
         (not runlog_step_confirmed) dataset_ready_next runlog_ready_next
         (= receipt_bound_next receipt_bound) dataset_step_confirmed_next
         runlog_step_confirmed_next
         (= receipt_path_visible_next receipt_path_visible)
         (= receipt_step_confirmed_next receipt_step_confirmed)
         (= finalize_returned_next finalize_returned))
    (and (= op 5) dataset_step_confirmed runlog_step_confirmed receipt_bound
         (not receipt_path_visible) dataset_ready_next runlog_ready_next
         receipt_bound_next dataset_step_confirmed_next runlog_step_confirmed_next
         receipt_path_visible_next
         (= receipt_step_confirmed_next receipt_step_confirmed)
         (= finalize_returned_next finalize_returned))
    (and (= op 6) receipt_path_visible (not receipt_step_confirmed)
         dataset_ready_next runlog_ready_next receipt_bound_next
         dataset_step_confirmed_next runlog_step_confirmed_next
         receipt_path_visible_next receipt_step_confirmed_next
         (= finalize_returned_next finalize_returned))
    (and (= op 7) receipt_step_confirmed (not finalize_returned)
         dataset_ready_next runlog_ready_next receipt_bound_next
         dataset_step_confirmed_next runlog_step_confirmed_next
         receipt_path_visible_next receipt_step_confirmed_next
         finalize_returned_next)
    (and (= op 8) (= dataset_ready_next dataset_ready)
         (= runlog_ready_next runlog_ready) (= receipt_bound_next receipt_bound)
         (= dataset_step_confirmed_next dataset_step_confirmed)
         (= runlog_step_confirmed_next runlog_step_confirmed)
         (= receipt_path_visible_next receipt_path_visible)
         (= receipt_step_confirmed_next receipt_step_confirmed)
         (= finalize_returned_next finalize_returned))))

; Initial-state proof.
(push 1)
(declare-const cd0 Bool)
(declare-const cl0 Bool)
(declare-const cb0 Bool)
(declare-const cdc0 Bool)
(declare-const clc0 Bool)
(declare-const crv0 Bool)
(declare-const crc0 Bool)
(declare-const cfr0 Bool)
(assert (and (not cd0) (not cl0) (not cb0) (not cdc0)
             (not clc0) (not crv0) (not crc0) (not cfr0)))
(assert (not (core-invariant cd0 cl0 cb0 cdc0 clc0 crv0 crc0 cfr0)))
(check-sat)
(pop 1)

; Inductiveness proof.
(push 1)
(declare-const cd Bool)
(declare-const cl Bool)
(declare-const cb Bool)
(declare-const cdc Bool)
(declare-const clc Bool)
(declare-const crv Bool)
(declare-const crc Bool)
(declare-const cfr Bool)
(declare-const cdn Bool)
(declare-const cln Bool)
(declare-const cbn Bool)
(declare-const cdcn Bool)
(declare-const clcn Bool)
(declare-const crvn Bool)
(declare-const crcn Bool)
(declare-const cfrn Bool)
(declare-const cop Int)
(assert (core-invariant cd cl cb cdc clc crv crc cfr))
(assert (core-transition cd cl cb cdc clc crv crc cfr
                         cdn cln cbn cdcn clcn crvn crcn cfrn cop))
(assert (not (core-invariant cdn cln cbn cdcn clcn crvn crcn cfrn)))
(check-sat)
(pop 1)

; Non-vacuity: the full core success path reaches a successful return.
(push 1)
(assert (core-transition false false false false false false false false
                         true false false false false false false false 0))
(assert (core-transition true false false false false false false false
                         true true false false false false false false 1))
(assert (core-transition true true false false false false false false
                         true true true false false false false false 2))
(assert (core-transition true true true false false false false false
                         true true true true false false false false 3))
(assert (core-transition true true true true false false false false
                         true true true true true false false false 4))
(assert (core-transition true true true true true false false false
                         true true true true true true false false 5))
(assert (core-transition true true true true true true false false
                         true true true true true true true false 6))
(assert (core-transition true true true true true true true false
                         true true true true true true true true 7))
(check-sat)
(pop 1)

; A receipt can be visible after install but before the finalizer reports success.
(push 1)
(assert (core-invariant true true true true true true false false))
(check-sat)
(pop 1)

; A successful return structurally binds both receipt digest fields to the
; prepared byte buffers.
(push 1)
(assert (core-invariant true true true true true true true true))
(assert (or (not (= core_receipt_dataset_digest
                    (digest prepared_dataset_bytes)))
            (not (= core_receipt_runlog_digest
                    (digest prepared_runlog_bytes)))))
(check-sat)
(pop 1)

; ----- Outer fault-observatory publication -----

(define-fun outer-invariant (
    (artifacts_installed Bool) (precommit_verified Bool)
    (receipt_visible Bool) (postcommit_verified Bool) (returned_success Bool)) Bool
  (and (=> precommit_verified artifacts_installed)
       (=> receipt_visible (and precommit_verified outer-receipt-binding))
       (=> postcommit_verified
           (and artifacts_installed receipt_visible
                outer-postcommit-binding))
       (=> returned_success postcommit_verified)))

; op: 0=all non-receipt installs succeed, 1=pre-commit snapshot verifies,
;     2=receipt path becomes visible, 3=post-commit snapshot verifies,
;     4=function returns success, 5=stutter/failure.
(define-fun outer-transition (
    (artifacts_installed Bool) (precommit_verified Bool)
    (receipt_visible Bool) (postcommit_verified Bool) (returned_success Bool)
    (artifacts_installed_next Bool) (precommit_verified_next Bool)
    (receipt_visible_next Bool) (postcommit_verified_next Bool)
    (returned_success_next Bool) (op Int)) Bool
  (or
    (and (= op 0) (not artifacts_installed) artifacts_installed_next
         (= precommit_verified_next precommit_verified)
         (= receipt_visible_next receipt_visible)
         (= postcommit_verified_next postcommit_verified)
         (= returned_success_next returned_success))
    (and (= op 1) artifacts_installed (not precommit_verified)
         artifacts_installed_next precommit_verified_next
         (= receipt_visible_next receipt_visible)
         (= postcommit_verified_next postcommit_verified)
         (= returned_success_next returned_success))
    (and (= op 2) precommit_verified (not receipt_visible)
         outer-receipt-binding
         artifacts_installed_next precommit_verified_next receipt_visible_next
         (= postcommit_verified_next postcommit_verified)
         (= returned_success_next returned_success))
    (and (= op 3) artifacts_installed receipt_visible (not postcommit_verified)
         outer-postcommit-binding
         artifacts_installed_next precommit_verified_next receipt_visible_next
         postcommit_verified_next (= returned_success_next returned_success))
    (and (= op 4) postcommit_verified (not returned_success)
         artifacts_installed_next precommit_verified_next receipt_visible_next
         postcommit_verified_next returned_success_next)
    (and (= op 5) (= artifacts_installed_next artifacts_installed)
         (= precommit_verified_next precommit_verified)
         (= receipt_visible_next receipt_visible)
         (= postcommit_verified_next postcommit_verified)
         (= returned_success_next returned_success))))

; Initial-state proof.
(push 1)
(declare-const oa0 Bool)
(declare-const op0 Bool)
(declare-const or0 Bool)
(declare-const ov0 Bool)
(declare-const os0 Bool)
(assert (and (not oa0) (not op0) (not or0) (not ov0) (not os0)))
(assert (not (outer-invariant oa0 op0 or0 ov0 os0)))
(check-sat)
(pop 1)

; Inductiveness proof.
(push 1)
(declare-const oa Bool)
(declare-const opv Bool)
(declare-const orv Bool)
(declare-const ov Bool)
(declare-const os Bool)
(declare-const oan Bool)
(declare-const opn Bool)
(declare-const orn Bool)
(declare-const ovn Bool)
(declare-const osn Bool)
(declare-const oop Int)
(assert (outer-invariant oa opv orv ov os))
(assert (outer-transition oa opv orv ov os oan opn orn ovn osn oop))
(assert (not (outer-invariant oan opn orn ovn osn)))
(check-sat)
(pop 1)

; Non-vacuity: the full outer success path reaches a verified return.
(push 1)
(assert (outer-transition false false false false false true false false false false 0))
(assert (outer-transition true false false false false true true false false false 1))
(assert (outer-transition true true false false false true true true false false 2))
(assert (outer-transition true true true false false true true true true false 3))
(assert (outer-transition true true true true false true true true true true 4))
(check-sat)
(pop 1)

; Receipt visibility alone is weaker than the observatory's verified-success return.
(push 1)
(assert (outer-invariant true true true false false))
(check-sat)
(pop 1)

; A successful outer return requires exact reread equality and receipt binding.
(push 1)
(assert (outer-invariant true true true true true))
(assert (or (not (= reread_dataset_bytes installed_dataset_bytes))
            (not (= reread_runlog_bytes installed_runlog_bytes))
            (not (= outer_receipt_dataset_digest
                    (digest reread_dataset_bytes)))
            (not (= outer_receipt_runlog_digest
                    (digest reread_runlog_bytes)))))
(check-sat)
(pop 1)
