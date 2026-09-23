# Recorded checkpoint families

The optional `crebain_family` adapter records one canonical experiment across independently bound CREBAIN checkpoint endpoints.
It uses the installed family owner and public NCP clients.
It adds no simulator, model, or estimator to the default Rust workspace.

Source controls exercise actual SDK processes and sockets with an explicitly synthetic native fixture.
They do not qualify native checkpoint equivalence, rendering, predictor quality, or scientific results.
The [E1 reference](../../docs/E1_REFERENCE.md) owns the separate numerical study.

## Ownership and order

The trusted collector receives `FamilyExperiment`.
Give predictors only the immutable observations and candidate actions permitted by the frozen study.
This separation provides no sandbox against hostile Python code.

CREBAIN owns native checkpoints, restored ancestry, sensor buffers, pressure evaluation, and installed process retirement.
Prisoma records the command order, original forecast bytes, execution receipts, and captured NCP exchanges.
NCP retains its existing protocol and binding rules.
The PID run-log schema and pinned submodule remain unchanged.

| Phase | Required order |
| --- | --- |
| Observation | Prepare; advance through the landmark; release every observed buffer |
| Decision | Checkpoint; commit original forecast bytes and the selected collection case |
| Selected execution | Advance the canonical endpoint through its final tick |
| Each restored branch | Reserve; restore; advance the frozen continuation; evaluate; finish |
| Completion | Release checkpoint; finish the canonical owner; finalize captures and canonical log |

The frozen family plan determines every command, endpoint, tick, and branch order.
A wrong role or command order rejects before dispatch.
Each accepted request is synchronized before its callback executes.
Each callback consumes the recorded payload.
Its receipt includes the primary response and the exact per-endpoint capture span.

`advance` returns a `FamilyObservation` after every read, release, acknowledgement, and canonical response synchronization.
Its `response` is the primary advance response.
Its `observation` contains the original immutable sensor payloads.

`commit_decision` accepts one nonempty byte string, bounded to 32,768 bytes.
Base64 carries those exact bytes in the canonical request without JSON reserialization.
CREBAIN receives their SHA-256 value and the separately selected collection case.
The adapter does not interpret scores or determine scientific eligibility.

The study caller must validate its exact forecast, stage, model, action choice, and permitted inputs before commitment.
Training and development collect the prescribed neutral continuation before labels.
Held-out execution commits the fixed selected model and action before labels.
Eight qualification episodes remain separate from the 112 numerical study episodes.
Fifteen-branch capacity controls do not enter five-action scientific denominators.

## Installed execution

Install the optional packages as described in the [application bridge README](README.md#use-with-crebain).
Select a separately installed CREBAIN checkpoint-family runtime and an admitted immutable `FamilyPlan`.
Create a new owner-private output directory before execution.
Keep the complete family capability inside the trusted collector.

```python
from prisoma_agent_bridge.crebain_family import owned_family_experiment

with owned_family_experiment(runtime, plan, output / "run.jsonl") as experiment:
    for tick in range(1, plan.landmark_tick + 1):
        step = experiment.advance(initial_target if tick == 1 else None)
        collect_permitted_observation(step)
    experiment.checkpoint()
    experiment.commit_decision(forecast_bytes, selected_case_id)

    for tick in range(plan.landmark_tick + 1, plan.body.planned_ticks + 1):
        experiment.advance(selected_target if tick == plan.landmark_tick + 1 else None)

    for branch in plan.branches:
        experiment.reserve(branch.case_id)
        experiment.restore(branch.slot)
        for tick in range(plan.landmark_tick + 1, plan.body.planned_ticks + 1):
            experiment.advance(
                branch.target if tick == plan.landmark_tick + 1 else None,
                slot=branch.slot,
            )
        label = experiment.evaluate(branch.slot)
        collect_reference_label(label)
        experiment.branch_finish(branch.slot)

    experiment.release_checkpoint()
    result = experiment.finish()
```

The example assumes an already validated forecast and selected target.
It supplies no scientific study runner or fallback predictor.
The canonical selected trajectory and matched restored neutral trajectory remain separate policy-comparison inputs.

Call `finish()` explicitly after the complete frozen command roster.
Its synchronized callback exits the installed owner and requires a successful process-retirement observation.
A normal context exit without completion fails and retires the owner exceptionally.
Caught execution failures still prevent successful context completion.
Original execution and cleanup exceptions remain available without formatting exception diagnostics during cleanup.

## Captures and readback

Each endpoint has its own transcript journal because restored endpoints require distinct run IDs.
The unchanged transcript contract requires one shared run ID within each journal.
The adapter satisfies both contracts with single-peer journals.

`family.capture.json` binds the frozen plan hash and the ordered journal roster.
Each row includes its role, binding, basename, exact content identity, and terminal verification summary.
The canonical terminal binds this index as its sibling artifact.
The index does not contain the canonical log hash.

A branch journal finalizes after its complete BranchFinish receipt.
The parent journal finalizes after the canonical Finish callback observes owner retirement.
The index is written without replacement only after every journal completes.
The canonical log finalizes after binding the index.
An interrupted write retains incomplete evidence and grants no completion claim.
These writes do not form an atomic transaction across files.

```python
from prisoma_agent_bridge.crebain_family import inspect_family_run

events = []
report = inspect_family_run(output / "run.jsonl", events.append)
# Accept retained visitor output only after successful return.
```

Readback starts no producer and grants no native mutation authority.
It validates canonical events through the pinned Rust reader.
Public family clients reconstruct every command from captured responses.
Each generated request must equal its original captured bytes.

The parent journal drives canonical commands until an acknowledged Reserve boundary.
The verifier then inspects the exact next child journal through its terminal.
The parent resumes only after that complete child inspection succeeds.
At most two journal readers remain active: the parent and one child.

Visitor results remain provisional through all child terminals, the final parent terminal, and repeated artifact identity checks.
Late parent corruption rejects the entire inspection, including previously visited children.
The report sets `family_replayed=true` and `scientific_validation=false`.
It sets `producer_process_retirement_verified=false` because readback cannot independently observe historical operating-system retirement.

The canonical compatibility envelope remains `record_role=execution_receipt` and `is_outcome_label=false`.
An inner Evaluate result does not change that outer event's meaning.

## Bounds

Let `T` be the final tick and `L` the landmark tick.
Let `H=T-L` be the continuation length and `B` the restored branch count.
The family has `B+1` endpoints, bounded to 16.
Let `A(k)` count original request-response exchanges for one complete advance at tick `k`.

Each advance and acknowledgement costs two exchanges.
Each due payload adds two exchanges per 32,768-byte read chunk, then two exchanges for release and acknowledgement.
The parent requires `10 + 2B + sum(A(k), k=1..T)` exchanges.
Each child requires `6 + sum(A(k), k=L+1..T)` exchanges.
The canonical log requires `T + 5 + B(H+4)` calls.

Admission limits the aggregate capture to 8,190 exchanges and one GiB of reserved journal bytes.
Each journal also passes its own public admission limits.
The public capacity function includes each journal's header and terminal allowance.
The capture index admits at most 16 rows and 65,536 encoded bytes.
The canonical bridge admits at most 1,024 calls and 65,536 bytes per application JSON value.

The fixed E1 scene uses two 160-by-120 RGBA cameras, both with period three, and one 16-kHz pressure source.
With `T=36` and `L=12`, five branches require 1,818 exchanges and 181 canonical calls.
Fifteen branches require 4,618 exchanges and 461 canonical calls.
The largest advance requires 22 exchanges and reserves 2,883,584 original frame bytes during readback.

These frame bounds exclude Python objects, reconstructed payloads, and canonical replay state.
A suspended parent reader also retains its current parser and exchange buffers.
The caller owns retained visitor outputs and derived numerical arrays.
Native memory, GPU allocations, disk headroom, and campaign limits require a separate operational freeze.

## Verification scope

The source controls cover three and sixteen endpoints through real SDK sockets with synthetic native values.
Paired rejection controls cover roles, ordering, commitment bytes, foreign journals, incomplete rosters, and late corruption.
Lifecycle controls retain incomplete evidence after launch, callback, recording, and cleanup failures.
The transcript reader closes rejected descriptors and preserves primary failures when cleanup also fails.

Use the complete application bridge gate with an isolated installed wheel.
Select the constructed family producer and synthetic support bridge explicitly for socket controls.
Native checkpoint equivalence, the E1 study, advertised model qualification, and stable NCP release remain separate gates.
