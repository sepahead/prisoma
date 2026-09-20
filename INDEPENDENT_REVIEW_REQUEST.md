# Independent review: unfinished CREBAIN ownership helper

This branch provides unqualified source for review. It does not authorize main promotion or a release.

Base: `56e835d7d6979df0a2ffeac6050e3860ddf8b1ea`.

| File | SHA-256 |
| --- | --- |
| `integrations/agent-bridge/python/prisoma_agent_bridge/crebain.py` | `00207dce6808baabd859c2b180065341c4c46d35d029aed7f027f5b2fbb3236a` |
| `integrations/agent-bridge/tests/test_owned_crebain.py` | `ec724f551fd0180308b623c340ab8ffb55fe2aa4430e30d0c34a5a840580134b` |

The helper enters CREBAIN ownership inside the synchronized canonical Prepare callback.
Successful exit requires explicit Finish, finalized capture, and finalized canonical recording.
Incomplete execution takes the exceptional cleanup path.

Reported local results: source-distribution build, wheel installation, 48 installed Python tests, and focused Ruff checks passed.
Tests cover synthetic lifecycle behavior, including caught-first-then-later failures, single Prepare/Finish, and combined cleanup errors.
The installed implementation matched the reviewed source. These are reported local observations, not independently accessible execution logs.

The capability projection check failed because generated views are stale.
Candidate audit failed with `LIVE_SOURCE_DRIFT`.
The complete development gate and native owned-helper campaign remain **NOT RUN**.
Local observations do not qualify native retirement, real-time performance, or scientific validity.
No PID source, dependency pin, schema, or Rust implementation changed.

Please investigate these questions:

1. Does the completion contract distinguish possible external effects, durable recorded completion, and process retirement correctly?
   Give counterexample traces and a minimal corrective design.
2. Can acknowledged chunk transfers and synchronized recording sustain useful 120-Hz experiments?
   Derive the operating envelope and compare bounded batching, partitioning, and shared-memory alternatives.
3. What minimal embodied world-model study would demonstrate value beyond recording and matched controllers?
   Define complete fork state, action support, proper scores, independent sampling units, leakage controls, and decisive negative results.

Review [the implementation](integrations/agent-bridge/python/prisoma_agent_bridge/crebain.py),
[lifecycle controls](integrations/agent-bridge/tests/test_owned_crebain.py), and
[existing contract](integrations/agent-bridge/README.md).
The contract has not yet been updated for this helper.

The [ecosystem review request](https://github.com/sepahead/NCP/blob/review/ncp-v1-independent-20260920/INDEPENDENT_REVIEW_REQUEST.md) supplies the broader requirements and source roster.
Leave this branch unmerged until the owning gates pass.
