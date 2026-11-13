//! Capture of CREBAIN NCP 2 loop runs: the checks before the journal and the journal's checks.

use prisoma_ncp_loop_capture::{import, verify_journal, MAX_CAPTURE_BYTES};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn scratch(name: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "prisoma-loop-capture-{}-{}-{name}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch directory");
    dir
}

/// A copy of a fixture run that a test may change.
fn copy(name: &str) -> PathBuf {
    let dir = scratch(name);
    for entry in fs::read_dir(fixture(name)).expect("fixture") {
        let entry = entry.expect("entry");
        fs::copy(entry.path(), dir.join(entry.file_name())).expect("copy");
    }
    dir
}

#[test]
fn both_runs_import_and_their_journals_verify() {
    // The runs of NCP 2 revision 2 wrote transcripts of version 1; those of revision 3, with
    // the suffix -v2, write version 2. The capture reads both.
    for (name, worlds, controllers) in [
        ("loop-1x1", 1, 1),
        ("loop-2x3", 2, 3),
        ("loop-1x1-v2", 1, 1),
        ("loop-2x3-v2", 2, 3),
    ] {
        let journal = scratch(name).join("journal.jsonl");
        let report = import(&fixture(name), &journal, MAX_CAPTURE_BYTES).expect("import");
        assert_eq!(
            (
                report.worlds,
                report.controllers,
                report.steps,
                report.records
            ),
            (worlds, controllers, 5, 7),
            "{name}"
        );
        assert_eq!(verify_journal(&journal).expect("verify"), report, "{name}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&journal).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}

#[test]
fn the_journal_keeps_the_state_chain_and_every_proposal() {
    let journal = scratch("chain").join("journal.jsonl");
    import(&fixture("loop-2x3"), &journal, MAX_CAPTURE_BYTES).expect("import");
    let records: Vec<Value> = fs::read_to_string(&journal)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let mut last = [Value::Null, Value::Null];
    for step in &records[1..6] {
        assert_eq!(step["data"]["controllers"].as_array().unwrap().len(), 3);
        for world in step["data"]["worlds"].as_array().unwrap() {
            let w = world["world"].as_u64().unwrap() as usize;
            if !last[w].is_null() {
                assert_eq!(
                    world["source_state_digest"], last[w],
                    "each step starts from the previous state"
                );
            }
            last[w] = world["state_digest"].clone();
        }
    }
    for (w, state) in last.iter().enumerate() {
        assert_eq!(
            &records[6]["data"]["worlds"][w]["final_state_digest"],
            state
        );
    }
}

fn refused(run: &Path, what: &str) {
    let journal = run.join("journal.jsonl");
    assert!(import(run, &journal, MAX_CAPTURE_BYTES).is_err(), "{what}");
    assert!(!journal.exists(), "{what}: no journal");
}

#[test]
fn a_changed_transcript_byte_is_refused_before_any_journal() {
    let run = copy("loop-2x3");
    let path = run.join("controller-1.ncpt");
    let original = fs::read(&path).unwrap();
    let at = original
        .windows(17)
        .rposition(|w| w == b"acceleration_mps2")
        .unwrap()
        + 21;
    let mut changed = original.clone();
    changed[at] = if changed[at] == b'1' { b'2' } else { b'1' };
    fs::write(&path, &changed).unwrap();
    refused(&run, "a changed controller result");
    fs::write(&path, &original).unwrap();
    let journal = run.join("restored.jsonl");
    import(&run, &journal, MAX_CAPTURE_BYTES).expect("positive control: the restored run imports");
    fs::rename(run.join("world-1.ncpt"), run.join("elsewhere.ncpt")).unwrap();
    refused(&run, "a missing world transcript");
}

#[test]
fn a_manifest_that_does_not_match_the_transcripts_is_refused() {
    type Edit = fn(&mut Value);
    let edits: [(&str, Edit); 4] = [
        ("a controller of another world", |m| {
            m["controllers"][1]["world"] = 1.into()
        }),
        ("another step count", |m| m["steps"] = 6.into()),
        ("a transcript outside the run", |m| {
            m["worlds"][0]["transcript"] = "../world-0.ncpt".into()
        }),
        ("another contract", |m| {
            m["body_contract"] = "0".repeat(64).into()
        }),
    ];
    for (what, edit) in edits {
        let run = copy("loop-2x3");
        let path = run.join("manifest.json");
        let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        edit(&mut manifest);
        fs::write(&path, manifest.to_string()).unwrap();
        refused(&run, what);
    }
}

#[test]
fn an_existing_journal_or_a_symbolic_link_is_never_replaced() {
    let dir = scratch("existing");
    let journal = dir.join("journal.jsonl");
    fs::write(&journal, b"kept\n").unwrap();
    assert!(import(&fixture("loop-1x1"), &journal, MAX_CAPTURE_BYTES).is_err());
    assert_eq!(fs::read(&journal).unwrap(), b"kept\n");
    #[cfg(unix)]
    {
        let target = dir.join("target");
        let link = dir.join("link.jsonl");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(import(&fixture("loop-1x1"), &link, MAX_CAPTURE_BYTES).is_err());
        assert!(!target.exists(), "nothing written through the link");
    }
}

#[test]
fn the_quota_is_checked_before_the_journal_exists() {
    let journal = scratch("quota").join("journal.jsonl");
    assert!(import(&fixture("loop-1x1"), &journal, 100).is_err());
    assert!(!journal.exists());
    assert!(
        import(&fixture("loop-1x1"), &journal, MAX_CAPTURE_BYTES + 1).is_err(),
        "above the ceiling"
    );
}

#[test]
fn a_changed_journal_fails_verification() {
    let dir = scratch("tamper");
    let journal = dir.join("journal.jsonl");
    import(&fixture("loop-1x1"), &journal, MAX_CAPTURE_BYTES).expect("import");
    let original = fs::read_to_string(&journal).unwrap();
    let lines: Vec<&str> = original.lines().collect();
    let mut cases: Vec<(&str, String)> = Vec::new();
    cases.push((
        "a changed number",
        original.replacen("\"tick_index\":3", "\"tick_index\":4", 1),
    ));
    cases.push((
        "no finish record",
        lines[..lines.len() - 1]
            .iter()
            .map(|l| format!("{l}\n"))
            .collect(),
    ));
    let mut swapped = lines.clone();
    swapped.swap(2, 3);
    cases.push((
        "swapped records",
        swapped.iter().map(|l| format!("{l}\n")).collect(),
    ));
    cases.push((
        "a record after the finish",
        format!("{original}{}\n", lines[1]),
    ));
    cases.push((
        "another encoding of the same record",
        original.replacen("{\"data\":", "{ \"data\":", 1),
    ));
    cases.push(("a missing last newline", original.trim_end().to_owned()));
    for (what, text) in cases {
        assert_ne!(text, original, "{what} changes the journal");
        fs::write(&journal, text).unwrap();
        assert!(verify_journal(&journal).is_err(), "{what}");
    }
    fs::write(&journal, &original).unwrap();
    verify_journal(&journal).expect("positive control: the original verifies");
}

#[test]
fn a_version_2_transcript_without_its_end_record_is_refused() {
    // A recorder that stopped early leaves no end record; the capture refuses the run.
    let run = copy("loop-1x1-v2");
    let path = run.join("world-0.ncpt");
    let bytes = fs::read(&path).unwrap();
    fs::write(&path, &bytes[..bytes.len() - 1]).unwrap();
    let journal = scratch("cut").join("journal.jsonl");
    assert!(import(&run, &journal, MAX_CAPTURE_BYTES).is_err());
    assert!(!journal.exists(), "no journal for a refused run");
}
