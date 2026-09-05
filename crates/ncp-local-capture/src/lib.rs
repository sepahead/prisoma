//! Native local NCP capture with bounded append-only storage and explicit completion.
//!
//! This observer issues no producer operation, command, PID request, or network call.
//! The journal establishes locally observed causal content, not remote attestation.
//! File quota admission is logical capacity, not a promise of future disk availability.

mod contract;
mod journal;
mod validation;

pub use contract::*;
pub use journal::verify_journal;

use journal::{summary, Journal};
use ncp_local::local::{LocalBackend, LocalBinding, LocalCode, LocalError, LocalOperation};
use ncp_local::local_data::PrepareData;
use serde_json::{json, Value};
use std::path::Path;
use validation::{invalid, parse, CausalState};

/// Single capture generation. Output path authority exists only at construction.
pub struct CaptureBackend {
    binding: LocalBinding,
    journal: Journal,
    state: Option<CausalState>,
    retired: bool,
}
struct Pending {
    state: CausalState,
    record: JournalRecord,
    bytes: Vec<u8>,
}
impl CaptureBackend {
    /// Create a new mode-0600, no-replace journal at a trusted supervisor path.
    ///
    /// No request can change the path. Existing files and symlinks reject.
    pub fn new(binding: LocalBinding, path: &Path) -> Result<Self, LocalError> {
        binding.validate()?;
        if binding.role != ncp_local::local::LocalRole::Capture {
            return Err(LocalError(LocalCode::Role));
        }
        Ok(Self {
            binding,
            journal: Journal::create(path)?,
            state: None,
            retired: false,
        })
    }
    fn pending(&self, operation: LocalOperation, body: &Value) -> Result<Pending, LocalError> {
        if self.retired {
            return Err(LocalError(LocalCode::Retired));
        }
        let (state, event) = if operation == LocalOperation::Prepare {
            if self.state.is_some() {
                return Err(LocalError(LocalCode::State));
            }
            let preparation: PrepareData = parse(body)?;
            let header = Header {
                binding: self.binding.clone(),
                preparation,
            };
            let state = CausalState::prepare(&header)?;
            (state, Event::Header(Box::new(header)))
        } else {
            let mut state = self.state.clone().ok_or(LocalError(LocalCode::State))?;
            let event = match operation {
                LocalOperation::Reserve => Event::Reserve(parse(body)?),
                LocalOperation::Capture => Event::Capture(Box::new(parse(body)?)),
                LocalOperation::Finish => Event::Finish(Box::new(parse(body)?)),
                LocalOperation::Abort if body == &json!({}) => Event::Abort(AbortData {
                    plan_digest: state.plan_digest.clone(),
                    completed_steps: state.completed,
                    reserved_step: state.reserved,
                    remaining_suffix: "unresolved".into(),
                }),
                _ => return Err(invalid()),
            };
            state.apply(&event)?;
            (state, event)
        };
        let (record, bytes) = self.journal.preview(event)?;
        let required = if operation == LocalOperation::Prepare {
            bytes.len() as u64 + state.plan.planned_steps * STEP_BYTES + TERMINAL_BYTES
        } else if operation == LocalOperation::Reserve {
            self.journal.bytes + bytes.len() as u64 + CAPTURE_BYTES + TERMINAL_BYTES
        } else {
            self.journal.bytes
                + bytes.len() as u64
                + if state.terminal.is_some() {
                    0
                } else {
                    TERMINAL_BYTES
                }
        };
        if required > state.quota {
            return Err(LocalError(LocalCode::Capacity));
        }
        Ok(Pending {
            state,
            record,
            bytes,
        })
    }
}
impl LocalBackend for CaptureBackend {
    fn validate(&self, operation: LocalOperation, body: &Value) -> Result<(), LocalError> {
        self.pending(operation, body).map(|_| ())
    }
    fn execute(&mut self, operation: LocalOperation, body: &Value) -> Result<Value, LocalError> {
        let pending = self.pending(operation, body)?;
        if let Err(error) =
            self.journal
                .append(&pending.record, &pending.bytes, pending.state.quota)
        {
            self.retire();
            return Err(error);
        }
        let terminal = pending.state.terminal.is_some();
        let output = if terminal {
            serde_json::to_value(summary(
                &pending.state,
                pending.record.record_digest.clone(),
                self.journal.bytes,
            ))
            .map_err(|_| invalid())
        } else {
            Ok(
                json!({"application_profile":APPLICATION_PROFILE,"plan_digest":pending.state.plan_digest,"captured_steps":pending.state.completed,"reserved_step":pending.state.reserved,"journal_digest":pending.record.record_digest,"journal_bytes":self.journal.bytes,"durable_record_committed":true,"scientific_validation":false}),
            )
        };
        if output.is_err() {
            self.retire();
            return output;
        }
        self.state = Some(pending.state);
        if terminal {
            self.retired = true;
            self.journal.close();
        }
        output
    }
    fn retire(&mut self) {
        self.retired = true;
        self.state = None;
        self.journal.close();
    }
}
