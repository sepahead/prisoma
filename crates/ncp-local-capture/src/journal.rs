use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

use ncp_local::bounded_json::preflight;
use ncp_local::local::{local_digest, LocalCode, LocalError};

use crate::contract::*;
use crate::validation::{invalid, CausalState};

fn io_error(_: std::io::Error) -> LocalError {
    LocalError(LocalCode::ExecutionUnknown)
}
fn digest(record: &JournalRecord) -> Result<String, LocalError> {
    let mut value = serde_json::to_value(record).map_err(|_| invalid())?;
    value
        .as_object_mut()
        .ok_or_else(invalid)?
        .remove("record_digest");
    local_digest("ncp.local.capture.v1", &value)
}
pub(crate) fn record_limit(event: &Event) -> u64 {
    match event {
        Event::Header(_) => STEP_BYTES,
        Event::Reserve(_) => RESERVE_BYTES,
        Event::Capture(_) => CAPTURE_BYTES,
        Event::Finish(_) | Event::Abort(_) => TERMINAL_BYTES,
    }
}

pub(crate) struct Journal {
    file: Option<File>,
    pub bytes: u64,
    pub next_ordinal: u64,
    pub last_digest: Option<String>,
}
impl Journal {
    pub fn create(path: &Path) -> Result<Self, LocalError> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        File::open(
            path.parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )
        .and_then(|parent| parent.sync_all())
        .map_err(io_error)?;
        Ok(Self {
            file: Some(file),
            bytes: 0,
            next_ordinal: 0,
            last_digest: None,
        })
    }
    pub fn preview(&self, event: Event) -> Result<(JournalRecord, Vec<u8>), LocalError> {
        let mut record = JournalRecord {
            schema: "prisoma.local-journal.v1".into(),
            ordinal: self.next_ordinal,
            previous_digest: self.last_digest.clone(),
            event,
            record_digest: String::new(),
        };
        record.record_digest = digest(&record)?;
        let mut bytes = serde_json::to_vec(&record).map_err(|_| invalid())?;
        bytes.push(b'\n');
        if bytes.len() as u64 > record_limit(&record.event) {
            return Err(LocalError(LocalCode::Capacity));
        }
        preflight(&bytes).map_err(|_| invalid())?;
        Ok((record, bytes))
    }
    pub fn append(
        &mut self,
        record: &JournalRecord,
        bytes: &[u8],
        quota: u64,
    ) -> Result<(), LocalError> {
        if record.ordinal != self.next_ordinal
            || record.previous_digest != self.last_digest
            || self
                .bytes
                .checked_add(bytes.len() as u64)
                .is_none_or(|total| total > quota)
        {
            return Err(LocalError(LocalCode::Capacity));
        }
        let file = self.file.as_mut().ok_or(LocalError(LocalCode::Retired))?;
        file.write_all(bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        self.bytes += bytes.len() as u64;
        self.next_ordinal += 1;
        self.last_digest = Some(record.record_digest.clone());
        Ok(())
    }
    pub fn close(&mut self) {
        self.file = None;
    }
}

pub(crate) fn summary(state: &CausalState, digest: String, bytes: u64) -> Verification {
    let complete = state.terminal == Some(true);
    Verification {
        schema: "prisoma.local-verification.v1".into(),
        plan_digest: state.plan_digest.clone(),
        planned_steps: state.plan.planned_steps,
        captured_steps: state.completed,
        store_completion: if complete { "complete" } else { "aborted" }.into(),
        execution_plan_complete: complete,
        remaining_suffix: if complete {
            None
        } else {
            Some("unresolved".into())
        },
        journal_digest: digest,
        journal_bytes: bytes,
        scientific_validation: false,
    }
}

/// Reopen a bounded journal and verify every link, causal exchange, and terminal record.
///
/// This is read-only. It executes no producer, command, estimator, or protocol owner.
/// An absent terminal record or an omitted complete step returns an error.
pub fn verify_journal(path: &Path) -> Result<Verification, LocalError> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(io_error)?;
    let initial = file.metadata().map_err(io_error)?;
    if !initial.is_file()
        || initial.len() == 0
        || initial.len() > MAX_CAPTURE_BYTES
        || initial.mode() & 0o077 != 0
    {
        return Err(invalid());
    }
    let mut reader = BufReader::new(&file);
    let mut state: Option<CausalState> = None;
    let mut ordinal = 0;
    let mut previous = None;
    let mut total = 0;
    loop {
        let mut line = vec![];
        let read = reader
            .by_ref()
            .take(STEP_BYTES + 1)
            .read_until(b'\n', &mut line)
            .map_err(io_error)?;
        if read == 0 {
            break;
        }
        if line.last() != Some(&b'\n') || line.len() as u64 > STEP_BYTES {
            return Err(invalid());
        }
        total += line.len() as u64;
        if total > MAX_CAPTURE_BYTES {
            return Err(invalid());
        }
        preflight(&line).map_err(|_| invalid())?;
        let raw: serde_json::Value = serde_json::from_slice(&line).map_err(|_| invalid())?;
        let record: JournalRecord = serde_json::from_value(raw.clone()).map_err(|_| invalid())?;
        // Every typed journal field is explicit, including null option fields.
        // Otherwise serde could silently reconstruct a deleted optional member.
        if serde_json::to_value(&record).map_err(|_| invalid())? != raw {
            return Err(invalid());
        }
        if record.schema != "prisoma.local-journal.v1"
            || record.ordinal != ordinal
            || record.previous_digest != previous
            || record.record_digest != digest(&record)?
            || line.len() as u64 > record_limit(&record.event)
        {
            return Err(invalid());
        }
        if let Some(active) = &mut state {
            active.apply(&record.event)?;
        } else {
            let Event::Header(header) = &record.event else {
                return Err(invalid());
            };
            let prepared = CausalState::prepare(header)?;
            if total + prepared.plan.planned_steps * STEP_BYTES + TERMINAL_BYTES > prepared.quota {
                return Err(LocalError(LocalCode::Capacity));
            }
            state = Some(prepared);
        }
        if state.as_ref().is_none_or(|state| total > state.quota) {
            return Err(LocalError(LocalCode::Capacity));
        }
        previous = Some(record.record_digest);
        ordinal += 1;
    }
    let final_metadata = file.metadata().map_err(io_error)?;
    if initial.len() != final_metadata.len()
        || initial.dev() != final_metadata.dev()
        || initial.ino() != final_metadata.ino()
        || initial.mtime() != final_metadata.mtime()
        || initial.mtime_nsec() != final_metadata.mtime_nsec()
    {
        return Err(invalid());
    }
    let state = state.ok_or_else(invalid)?;
    if state.terminal.is_none() {
        return Err(LocalError(LocalCode::State));
    }
    Ok(summary(&state, previous.ok_or_else(invalid)?, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_write_failure_does_not_commit_journal_metadata() {
        let path = std::env::temp_dir().join(format!(
            "prisoma-journal-write-control-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut journal = Journal::create(&path).unwrap();
        let (record, bytes) = journal
            .preview(Event::Reserve(ReserveData {
                plan_digest: "0".repeat(64),
                step: 1,
            }))
            .unwrap();
        // An actual read-only descriptor makes the write syscall fail.
        journal.file = Some(File::open(&path).unwrap());
        assert!(journal.append(&record, &bytes, STEP_BYTES).is_err());
        assert_eq!(journal.bytes, 0);
        assert_eq!(journal.next_ordinal, 0);
        assert!(journal.last_digest.is_none());
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
        // A writable descriptor proves the same bounded storage operation works.
        journal.file = Some(OpenOptions::new().write(true).open(&path).unwrap());
        journal.append(&record, &bytes, STEP_BYTES).unwrap();
        assert_eq!(journal.bytes, bytes.len() as u64);
        assert_eq!(journal.next_ordinal, 1);
        assert_eq!(journal.last_digest, Some(record.record_digest));
        journal.close();
        std::fs::remove_file(path).unwrap();
    }
}
