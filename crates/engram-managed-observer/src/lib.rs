//! Read-only observation of Engram managed closed-loop receipts.
//!
//! This crate implements the inherited-pipe Host API 2 child protocol. It
//! verifies an ordered sequence of Engram step receipts and one terminal run
//! receipt. It cannot issue an Agent Bridge command, action, intervention,
//! plant request, NCP message, network request, file request, or artifact.

mod canonical;
mod contract;
mod observer;
mod protocol;

pub use contract::{
    FinishRequest, ObserveRequest, ObserverOutcome, ObserverResponse, PrepareRequest,
    RuntimeConfiguration, RuntimeLifecycleReceiptBinding, RuntimeLifecycleScalar,
};
pub use observer::{
    closed_loop_step_id, observer_response_receipt_sha256, source_cleanup_receipt_sha256,
    source_run_receipt_sha256, source_step_receipt_sha256, CleanupReceipt, ObserverError,
    ObserverRuntime, SourceRunReceipt, SourceStepReceipt,
};
pub use protocol::{
    ipc_schema_sha256, operation_roster_sha256, serve_managed_runtime, ProtocolError,
    RuntimeSessionReceipt,
};
