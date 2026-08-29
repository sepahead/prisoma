# Prisoma managed observer

This package is a read-only Engram Host API 2 child runtime.
It observes content-bound Engram closed-loop receipts.

The runtime never controls an agent, simulator, robot, drone, plant, or network.
The host sends complete source receipts through inherited private pipes.
The operation contract exposes no filesystem, network, or bulk-artifact operation.
The reviewed sandbox enforces network isolation, but not filesystem isolation.

## Contract boundary

The binary implements `engram.managed-runtime-stdio.v1`.
Each frame has a big-endian `uint32` length and one strict JSON object.
The generic envelope is `engram.managed-runtime-ipc.v1`.

The runtime accepts these operations:

| Operation | Purpose | Compute grant | Frame limit |
| --- | --- | --- | ---: |
| `prisoma.observer.prepare.v1` | Record a host-declared run and roster projection. | `none` | 32,768 bytes |
| `prisoma.observer.observe.v1` | Verify the next source step receipt. | `none` | 49,152 bytes |
| `prisoma.observer.finish.v1` | Verify the terminal receipt and clear state. | `none` | 32,768 bytes |

Each response has an 8,192-byte operation limit.
Each operation has a 1,000 ms declared timeout.
Engram owns hard deadline enforcement and process termination.

The frame ceilings include the largest semantically admissible ASCII rosters.
They also include 64 maximally escaped 128-byte fault codes.
Tests admit those complete envelopes and reject one byte above each ceiling.
The runtime applies the 128-byte bound to UTF-8 bytes, not Unicode scalar count.

L0 schemas bound identifier and digest lengths without plugin-authored regular expressions.
The runtime enforces ASCII entity identifiers and lowercase SHA-256 digests.
This semantic narrowing is part of request admission.

The manifest permits one in-flight operation.
It permits 1,042 operations in one generation.
This bound reserves 16 attempts for correctable domain rejections.
Framing, envelope, replay, and schema failures terminate the generation.
Canonicalization failures and contained operation panics also fail stop.
Those internal failures clear state and do not consume the rejection reserve.
The runtime emits at most one bounded failure response before exit.
It permits no automatic restart.

## Observation lifecycle

`prepare` records these host-declared values:

- the study run and definition digests
- the closed-loop and runtime binding digests
- the runtime adapter configuration digest
- the neural provider identity digest
- one to 64 sorted channel identifiers
- an equal roster of unique subject identifiers
- an immutable planned count of one to 1,024 source steps
- a prepared maximum of one to 1,024 steps

The subject roster can name three CREBAIN drones.
Engram source receipts do not carry or authenticate those names.
The observer reports `source_roster_authenticated=false`.
This projection does not control those drones.

`observe` accepts the next complete source step receipt.
It recomputes the Engram V2 step identifier and receipt digest.
It binds the declared provider execution scope and provider receipt digest.
It rejects gaps, replay, identity drift, digest drift, and budget exhaustion.
Before state mutation, each later input must equal the preceding step output.
`finish` joins the first input to the initial runtime snapshot.
It also revalidates the complete snapshot chain as defense in depth.

The rolling observer transcript covers accepted semantic receipts only.
Rejected requests and error envelopes do not advance that transcript.
The transcript is not an attempt log.

`finish` reconstructs the complete Engram V2 terminal receipt.
It verifies runtime, lifecycle, neural-session, step, cleanup, transcript, and run lineage.
It mirrors the terminal `neural_durable_evidence_profile` into transcript V5.
It verifies the independent controller and runtime logical-clock schedule.
It verifies the last host-confirmed runtime tic against the completed step count.
It verifies every V1 neural execution binding against its V2 step receipt.
At most one neural execution can exist beyond the completed runtime steps.
That tail must use the host-derived identifier for the prepared run.
It accepts zero observed steps for a valid failed or cancelled source run.
It clears all retained identifiers and receipts after success.

Cleanup has exactly two receipts in `runtime`, then `neural`, order.
Their owners must equal the prepared runtime and neural identities.
Those owner identities must differ.
Runtime mode is `finish` exactly when a runtime-finish receipt exists.
Otherwise, runtime mode is `generation-kill`.
Neural mode is always `close`.
A bound `finish` lifecycle requires `clean-exit`.
A bound `generation-kill` lifecycle requires `terminated` or `killed`.
The runtime cleanup repeats the exact optional lifecycle binding.
The neural cleanup cannot carry a runtime lifecycle.
Host API 2 carries that lifecycle through one fixed 22-scalar projection.
It carries the timebase through one fixed 10-scalar projection.
It carries an optional neural execution tail through six scalar slots.
It carries each cleanup receipt through 12 scalar slots.

Cleanup V2 binds optional provider terminal and lifecycle receipt digests.
Null and non-null positions enter the cleanup, transcript, and run digests.
Those fields describe upstream receipt lineage only.
They do not establish NEST semantic closure or scientific evidence.

This source receipt version is float-free by contract.
Numeric receipt fields are bounded integers.
A future float field requires a versioned canonicalization decision before adoption.
An absent lifecycle uses exactly 22 null values.
Partial projections and nested control objects fail closed.

Neural preparation and session digests must occur together.
A neural preparation requires an initial runtime snapshot.
A runtime finish requires preparation, a snapshot, and every planned step.
A completed run requires preparation, a snapshot, steps, runtime finish, and complete cleanup.
Any recorded step requires complete preparation lineage.
Runtime finish requires primary reason `loop.completed`, and that reason requires runtime finish.
Complete cleanup plus runtime finish requires completed status.
Without finish, complete cleanup maps `loop.cancelled` to cancelled status.
It maps `runtime.overload` to overloaded status and every other reason to failed status.
Incomplete cleanup always requires failed status and reason `cleanup.unconfirmed`.
Complete cleanup requires the terminal reason to equal the primary reason.

A valid fixture retains runtime `finish` after neural close is unconfirmed.
That source run has failed status, while observation itself succeeds descriptively.

Clean EOF also clears retained state.
`Drop` supplies a final cleanup defense.
The process boundary contains request and reader panics.

## Authority boundary

All operations use class `observation`, effect `none`, and compute grant `none`.
They request zero CPU time through the Host API operation contract.
The runtime performs bounded local verification within its child-generation budget.

The runtime has no action, intervention, Agent Bridge command, or PID result.
The operation contract exposes no NCP, network, filesystem, artifact, or process-launch operation.
It has no embodiment, actuation, plant, or physical authority.

An accepted source receipt is not scientific evidence.
An observer receipt is not an attestation or safety finding.
It cannot prove an upstream event that Engram did not include.
Every child response sets `source_durable_evidence_verified=false`.
The profile name does not prove that a NEST evidence bundle exists or rejoins.

## Source compatibility

`contracts/PROVENANCE.json` binds the copied generic IPC schema.
It also binds every imported Engram fixture-generator Python source to one Git commit.
Each source row records its path, SHA-256, Git blob, byte count, and module aliases.
Source closure does not attest Python's loaded bytecode or response-bound loaded bytes.

Three source receipts were generated by Engram's `build_run_receipt` function.
Their inputs are synthetic cross-language vectors, not NEST execution evidence.
Two receipts bind an exact reviewed-runtime lifecycle.
One has two steps and a neural-session digest.
It declares the NEST bundle-v2 profile without supplying or validating a bundle.
Its neural cleanup binds non-null provider terminal and lifecycle digests.
One retains runtime finish after an unconfirmed neural close.
The third is an honest zero-step failure without a lifecycle binding.

All vectors use `engram.managed-runtime-json.v1` canonicalization.
They bind the V5 terminal transcript and both logical clocks.
The shared float corpus checks 25 boundary cases and 4,096 SplitMix64 samples.

The Rust tests verify all three source receipts without adapting their fields.
The fixture gate requires every imported source byte to match the expected commit.
It rejects revision drift, working-tree source drift, and import-roster drift.
The helper removes Git repository-selection overrides and rejects imported paths that traverse links.

Regenerate the vectors with the exact Engram receipt builders:

```bash
/path/to/engram-env/bin/python \
  integrations/engram/managed-observer/scripts/generate-source-fixtures.py \
  --engram-root /path/to/engram \
  --expected-engram-revision <full-commit-id>
```

Use `--verify` to check the same builders without changing the fixtures.

## Verify the runtime

Run this complete gate from the Prisoma repository:

```bash
just engram-managed-observer-check
```

The gate formats, builds, lints, tests, and documents the Rust crate.
It checks every schema digest and authority field.
It also executes the real binary and compares the normalized transcript.
This portable gate does not authorize a release artifact.

Run the clean-source release gate on an Apple silicon Mac:

```bash
just engram-managed-observer-observed-release <full-prisoma-commit-id>
```

The gate requires a clean checkout at the named `origin/main` commit.
It uses one isolated, locked, offline Cargo build.
It records the exact source roster and toolchain bytes.
The roster includes the build generator and its validator sources.
It accepts only a thin arm64 Mach-O executable for macOS.
The adjacent receipt uses mode `0600`.
The executable uses mode `0700`.
The receipt does not attest reproducibility or external dependency closure.
It grants no production, NCP, MUSIC, physical, or scientific authority.

Use this command for the standalone supply-chain gate:

```bash
cargo deny --manifest-path crates/engram-managed-observer/Cargo.toml \
  --all-features check
```

## Author a reviewed package

Run the clean-source release gate before package staging.
Do not stage a binary from a target-path inference or permission change.

Choose a new package directory.
The staging script never replaces an existing path.

```bash
python3 integrations/engram/managed-observer/scripts/stage-package.py \
  --binary-build-receipt \
    crates/engram-managed-observer/target/release/prisoma-engram-managed-observer.observed-build.json \
  --expected-prisoma-revision <full-prisoma-commit-id> \
  --output /private/tmp/prisoma-observer-package \
  --stage-receipt /private/tmp/prisoma-observer-package-stage-receipt.json
```

The staging script reopens the receipt before and after copying.
It copies only the exact receipted executable bytes.
The script stages one executable and eight schema contracts.
It copies no configuration, fixture, transcript, or provenance file.
It does not change the source recipe or configuration permissions.
It writes one owner-private stage receipt at the requested sibling path.
The receipt binds the exact build receipt and committed staging sources.
It also binds every staged package byte and file mode.
The receipt grants no installation, execution, NCP, or MUSIC authority.

Pass `authoring.macos-aarch64-darwin.json` to Engram's authoring tool.
Engram must seal the package and create its installation identity.

The manifest template contains zero lock and package digests.
Those placeholders grant no launch authority.
No current receipt establishes a production manager execution.

## Reviewed development interoperability

`evidence/engram-reviewed-development-e2e.json` is a historical v1 audit record.
Its source state was a working-tree candidate.
Its old schema bytes are not retained as a current contract.
The gate binds its exact bytes and keeps it audit-only.
It does not satisfy the adjacent v2 schema.

The current v2 operational gate is `NOT RUN`.
The v2 schema and harness are bootstrap surfaces only.
A future v2 receipt must close every imported Engram source against one commit.
It must bind the input bundle, source fixture, and sample transcript.
It must set `engram_loaded_source_bytes_attested=false`.

The historical run installed a packed runtime in an owner-private temporary store.
Engram rejected removal while the child retained its package lease.
Engram launched a verified, owner-private, user-immutable staging copy by absolute path.
The launch did not reopen the package path.
Path lookup remained possible, and external dependency closure stayed unattested.
The sandbox enforced network isolation but did not enforce filesystem isolation.
The child returned exact transcript responses for prepare, two observations, and finish.
The harness verified every response authority constant.
Clean EOF ended the child.
Engram sealed the process group while the guardian was unreaped.
It then reaped both processes and released the retained guardian lease.
Removal then succeeded, and the harness erased the temporary store.
The bounded projection excludes numeric process and process-group identifiers.

The subject projection names three CREBAIN drones.
It authenticates no subject roster and controls no drone.
The receipt records no NCP, physical, or scientific authority.

The historical evidence uses `engram.reviewed-native-development.v1`.
It does not establish production manager execution or publisher authentication.
It grants no automatic restart or replayable live-launch authority.
Only a future v2 run can exercise the current imported-source closure.

Reproduce the operational check from an Engram Python environment:

```bash
/path/to/engram-env/bin/python \
  integrations/engram/managed-observer/scripts/run-reviewed-development-e2e.py \
  --engram-root /path/to/engram \
  --expected-engram-revision <full-commit-id> \
  --bundle /path/to/packed-bundle \
  --output /private/tmp/prisoma-observer-e2e.json \
  --provenance-output /private/tmp/prisoma-observer-PROVENANCE.json
```

Both output paths must not exist.
The harness preserves its input bundle and deletes only its temporary store.
It emits a complete provenance candidate beside the operational receipt.
Review both candidates before replacing the tracked receipt and provenance.
Do not describe the v2 gate as complete before that review passes.

## External NEST evidence validation

The child never receives `NestClosedLoopEvidenceBundleV2`.
It receives no filesystem operation or bulk-artifact route.
It cannot validate that large provider bundle.

Use the external validator after Engram writes a terminal run and evidence bundle:

```bash
/path/to/engram-env/bin/python \
  integrations/engram/managed-observer/scripts/summarize-nest-evidence.py \
  --engram-root /path/to/engram \
  --expected-engram-revision <full-commit-id> \
  --run-receipt /path/to/run-receipt.json \
  --evidence-bundle /path/to/nest-evidence-bundle.json \
  --output /private/tmp/prisoma-nest-evidence-summary.json
```

The tool calls Engram's exact `validate_nest_evidence_against_run` function.
It applies Engram's 240 MiB NEST evidence admission bound.
It closes every imported validator Python source against the expected commit.
It binds the exact bounded-summary schema used for validation.
It emits counts, terminal dispositions, lineage digests, and authority constants.
It emits no trace arrays, actions, process identifiers, or executable payloads.
Only this external summary sets `source_durable_evidence_verified=true`.
That field means exact bundle-to-run rejoin only.
It grants no execution, PID, NCP, physical, or scientific authority.
It does not attest Python's loaded bytecode or response-bound loaded bytes.
Use `--verify` with the same inputs to compare an existing summary exactly.

## CREBAIN real-NEST matrix review

The matrix harness reviews the CREBAIN 1/2/3-drone capture roster.
It does not generate or replace CREBAIN evidence.
It accepts capture v2 and evidence-index v2 only.
Evidence-index v1 remains historical and audit-only.
Unknown versions and unknown fields fail closed.

The harness joins each indexed capture to its exact bytes.
It accepts CREBAIN evidence-index v2 and installed proof v3 only.
It reads operational evidence from `real-nest-3.9-v2`.
The tracked operational input suite remains `real-nest-3.9-v1`.

The gate names the CREBAIN source commit as C0.
It names the evidence-publication commit as C1.
C1 must have C0 as its only parent.
C1 must add exactly `INDEX.json` and the three capture files.
Each added path must be one committed `100644` blob.
No other C1 change is permitted.
The gate reads C1's raw commit object.
It rejects grafts, shallow metadata, replacement refs, and non-normal index flags.
It rejoins Git's worktree, administrative directory, and common directory.
Each publication directory component must be owner-controlled.
Each publication file must be owner-owned and uniquely linked.
Its filesystem mode must agree with Git's non-executable data mode.
The index binds C0 through `crebain_source_repository`.
The output records C0 and C1 as separate closed objects.

It rejoins the tracked input suite and the four-file tool source closure.
It reopens every tool from C0 while the clean checkout remains at C1.
It validates the embedded observed-build receipt independently.
It verifies the exact Cargo command and committed build inputs.
It validates the embedded package-stage receipt independently.
It rejoins the staged executable and all staged contract bytes.
It validates the embedded Engram pack receipt independently.
Its policy requires one clean Engram `origin/main` commit across both operations.
Each operation must report source re-verification and local success.
It must bind committed `scripts/engram_extension.py` bytes and Git blob identity.
Each capture must join that tool row to its loaded host-module source closure.
The harness reopens the committed tool before final matrix publication.
It rejoins the installed-package proof and its package-store lineage.
It verifies distinct closed receipt stores for all three runs.
Each store must contain the exact eight-path V5 roster.
Receipt and evidence paths use their content digests.
Observation and authority paths use the terminal receipt digest.
Finalization paths use the reservation identifier.
Admission-anchor paths use the study-run key.
Prisoma rederives the terminal, evidence, store metadata, and lock bytes.
Stored terminal and evidence bytes omit their self-digest fields.
Those bytes use Engram managed-runtime JSON without a newline.
The output projects every file path, byte count, and digest.
Capture v2 embeds the metadata and four V5 sidecar bodies.
Prisoma reconstructs their managed-runtime bytes and content digests.
It rejoins reservations, dispatches, publication records, and authority records.
CREBAIN capture closures retain their separate ledger JSON rules.

The V2 gate validates the exact worker launch expectation.
It rejoins every project runtime file to the stable Engram source roster.
It separately rejoins the embedded exec-gate and guardian source identities.
It validates the isolated worker, sandbox dispatch, and guardian commands.
It validates launch, preparation, capabilities, session, and lifecycle bindings.
It rejoins each step attempt to its request and execution receipt.

Each run has one path-sensitive runtime source closure.
All three runtime closure digests must differ.
Each closure also carries one stable, path-independent source-roster digest.
All three stable roster digests must match.

It verifies the exact 6N population topology for each drone count.
It rejoins worker guardian, source, terminal, NEST, and runtime lifecycle receipts.
It invokes Engram's exact NEST validator in a separate process.
It also sends each terminal receipt through the release observer.
The validator reopens its own sources from the bound Prisoma revision.
It reopens imported Engram sources from the bound Engram revision.

The observer response remains descriptive and sets `source_durable_evidence_verified=false`.
Only the separate Engram validator summary sets that field to `true`.
That value means exact bundle-to-run rejoin only.
The pack receipt does not attest loaded bytecode, interpreter state, or Python dependencies.
It grants no publisher, signature, reproducibility, execution, or installation authority.

The output schema fixes row order to one, two, then three drones.
Host-declared subject rosters remain unauthenticated.
The receipt records `filesystem_isolation_enforced=false`.
Receipt-store closure records observed artifacts, not host filesystem isolation.
The receipt grants no Agent Bridge, NCP, MUSIC, physical, plant, or scientific authority.

After all repositories reach immutable pushed commits, run the observed release gate:

```bash
just engram-managed-observer-observed-release <full-prisoma-commit-id>
```

Then run the review from an Engram Python environment:

```bash
/path/to/engram-env/bin/python \
  integrations/engram/managed-observer/scripts/review-crebain-real-nest-matrix.py \
  --binary crates/engram-managed-observer/target/release/prisoma-engram-managed-observer \
  --binary-build-receipt \
    crates/engram-managed-observer/target/release/prisoma-engram-managed-observer.observed-build.json \
  --crebain-root /path/to/crebain \
  --engram-root /path/to/engram \
  --expected-prisoma-revision <full-commit-id> \
  --expected-crebain-source-revision <full-C0-commit-id> \
  --expected-crebain-publication-revision <full-C1-commit-id> \
  --expected-engram-revision <full-commit-id> \
  --output /private/tmp/prisoma-crebain-observer-matrix.json
```

The output path must not exist.
Use `--verify` to compare an existing receipt without replacement.
Prisoma and Engram must be clean at their named `origin/main` commits.
CREBAIN must be clean at C1, and `origin/main` must equal C1.
Use full-history checkouts for the operational review.
The harness reopens C0 directly from the same CREBAIN object database.
It rejects tracked changes, staged changes, and untracked files.
It requires the exact adjacent observed-build receipt.
It binds each origin URL, commit, tree, object format, and `origin/main` commit.
It rejoins source, toolchain, artifact, and arm64 Mach-O identities.
It snapshots each input before use.
It rechecks repository identities and core matrix inputs before output.

## Interoperability fixture

`sample-transcript.json` records one normalized real execution.
It contains one three-subject session and two observed steps.
The complete gate reconstructs it with the freshly built release binary.

The generator normalizes runtime message identifiers and the runtime nonce.
It does not recompute or replace semantic observer receipts.

Verify the fixture with this command:

```bash
python3 integrations/engram/managed-observer/scripts/generate-transcript.py \
  --binary crates/engram-managed-observer/target/release/prisoma-engram-managed-observer \
  --verify integrations/engram/managed-observer/sample-transcript.json
```

Regenerate it after an accepted contract or source-fixture change:

```bash
python3 integrations/engram/managed-observer/scripts/generate-transcript.py \
  --binary crates/engram-managed-observer/target/release/prisoma-engram-managed-observer \
  --compact \
  --output integrations/engram/managed-observer/sample-transcript.json
```

The fixture is compatibility evidence only.
It is not a sealed package, production launch, safety result, or scientific result.

## NCP separation

This runtime does not import, translate, or implement NCP.
It makes no NCP 1.0 compatibility or qualification claim.
The legacy wire-0.8 observer remains separate and unchanged.
