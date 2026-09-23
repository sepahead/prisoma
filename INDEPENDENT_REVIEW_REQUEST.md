# Owned CREBAIN helper: recovery and review

This record preserves the submitted source, subsequent repairs, and qualification boundaries.
Source review does not authorize a release or establish native retirement.

Base: `56e835d7d6979df0a2ffeac6050e3860ddf8b1ea`.

The first submitted revision contained these bytes.

| File | SHA-256 |
| --- | --- |
| `integrations/agent-bridge/python/prisoma_agent_bridge/crebain.py` | `00207dce6808baabd859c2b180065341c4c46d35d029aed7f027f5b2fbb3236a` |
| `integrations/agent-bridge/tests/test_owned_crebain.py` | `ec724f551fd0180308b623c340ab8ffb55fe2aa4430e30d0c34a5a840580134b` |

## September 21 repair after review finding F16

Review finding F16 rejected the first revision's failure-graph search.
That search recursed through `__cause__`, `__context__`, and exception-group members without a bound.
A deep failure chain raised `RecursionError` before the cleanup block ran.
The session, the journal, and the bridge then stayed open, and both failure roots were lost.

The September 21 revision searches iteratively under an explicit node bound.
It returns a definite result or an explicit unknown result.
The caller collapses a retained failure root only for a definite positive result.
An unknown result keeps both roots.
Cleanup now runs for every observed failure depth.

These hashes identify the September 21 repair.

| File | SHA-256 |
| --- | --- |
| `integrations/agent-bridge/python/prisoma_agent_bridge/crebain.py` | `84671bbaeb977ebbea00f36fcc89bd7fa0c11a911b64909ae299d7bfaeb18f03` |
| `integrations/agent-bridge/tests/test_owned_crebain.py` | `15d9775fe809c6d2ff629c3bea9ab1abb00fabf7f6990498ebc71cc81855df76` |

Reported local results for the September 21 revision: wheel build, wheel installation, and 17 installed
`test_owned_crebain.py` tests passed. The three added tests fail against the first revision's
implementation and pass against that repair. The 14 earlier tests keep their behavior.
These are reported local observations, not independently accessible execution logs.
The repair does not qualify this branch for main promotion.

The helper enters CREBAIN ownership inside the synchronized canonical Prepare callback.
Successful exit requires explicit Finish, finalized capture, and finalized canonical recording.
Incomplete execution takes the exceptional cleanup path.

Reported local results: source-distribution build, wheel installation, 48 installed Python tests, and focused Ruff checks passed.
Tests cover synthetic lifecycle behavior, including caught-first-then-later failures, single Prepare/Finish, and combined cleanup errors.
The installed implementation matched the reviewed source. These are reported local observations, not independently accessible execution logs.

The initial capability projection check failed because generated views were stale.
Commit `bec9607512509878c10c7ba519f9cf87c12872d6` regenerated those views.
Candidate audit still failed with `LIVE_SOURCE_DRIFT`.
At that revision, the complete development gate and native owned-helper campaign were **NOT RUN**.
Local observations do not qualify native retirement, real-time performance, or scientific validity.
No PID source, dependency pin, schema, or Rust implementation changed.

The September 21 request asked reviewers to investigate these questions:

1. Does the completion contract distinguish possible external effects, durable recorded completion, and process retirement correctly?
   Give counterexample traces and a minimal corrective design.
2. Can acknowledged chunk transfers and synchronized recording sustain useful 120-Hz experiments?
   Derive the operating envelope and compare bounded batching, partitioning, and shared-memory alternatives.
3. What minimal embodied world-model study would demonstrate value beyond recording and matched controllers?
   Define complete fork state, action support, proper scores, independent sampling units, leakage controls, and decisive negative results.

Review [the implementation](integrations/agent-bridge/python/prisoma_agent_bridge/crebain.py),
[lifecycle controls](integrations/agent-bridge/tests/test_owned_crebain.py), and
[existing contract](integrations/agent-bridge/README.md).
At that revision, the contract did not document this helper.

The [ecosystem review request](https://github.com/sepahead/NCP/blob/review/ncp-v1-independent-20260920/INDEPENDENT_REVIEW_REQUEST.md) supplies the broader requirements and source roster.
Leave this branch unmerged until the owning gates pass.

## September 23 bounded traversal review

The node bound did not bound child-edge work or temporary memory.
A 100,000-reference shared group exhausted no node budget while scanning every child.
A 20,000-child group allocated 3,423,768 temporary bytes with a four-node budget in the retained source probe.
An overridden `exceptions` property could raise before cleanup.
Independent review also reproduced an overridden `__class__` property through `isinstance`.

The repair uses lazy child cursors and charges every root or child reference.
It bounds each search to 4,096 references and retains uncertain roots.
`failure_graph_truncated` records incomplete membership analysis without truncating the original exception objects.
Built-in type inspection and the exception-group descriptor avoid diagnostic callbacks.
Causes and contexts remain distinct from exception-group membership.
They cannot justify removing a separately retained failure root.

Five alternatives informed this decision:

| Approach | Assumption and benefit | Failure mode | Decisive control |
| --- | --- | --- | --- |
| Retain every root | Identity retention needs no search. | Repeated cleanup adds duplicate roots indefinitely. | Repeated shallow cleanup and exact reachable-root controls. |
| Depth-limited recursion | A shallow graph bounds stack use. | Broad groups and shared edges remain unbounded. | Deep groups and 20,000 repeated children. |
| Eager node-limited traversal | Unique-node count approximates total work. | Child enumeration and queued identities exceed the budget. | Four-node broad-group allocation probe. |
| Lazy edge-limited traversal | Built-in group tuples define membership. | Exhaustion leaves membership unknown. | Exact-budget, shared-edge, hostile-property, and cleanup controls. |
| Flatten or serialize failures | A copied diagnostic can replace original structure. | Identity is lost and formatting can execute arbitrary callbacks. | Original-root identity and hostile formatting controls. |

Lazy edge-limited traversal preserves existing shallow behavior while bounding deep and broad searches.
Unknown membership retains both roots and permits resource cleanup.
The selected bounds describe diagnostic traversal, not arbitrary callback execution or process retirement.

The review applied five separate lenses:

- Scientific and quality validity: diagnostic containment does not establish process retirement or scientific validity.
- Runtime and protocol correctness: bound references while preserving explicit Finish, canonical Prepare ordering, and exceptional owner retirement.
- Security and provenance: avoid diagnostic callbacks, retain original failure objects, and keep historical source receipts distinct.
- Statistical and generalization evidence: retain failing controls and test deep, broad, shared, cyclic, hostile, and healthy inputs without population claims.
- Maintainability and operations: distinguish failure-graph truncation from producer-output truncation and verify the installed successor separately.

Installed package gates and native qualification require their own exact-source execution records.
Candidate source capture follows the gated source commit through the owning generator.
