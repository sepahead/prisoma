# Prisoma UI Design Blueprint

This file defines image-generation prompts for a deferred user interface. It is not a description
of an implemented application. The current runnable viewer surface is the `pid-rerun` adapter.

The product order is:

1. Rerun-first offline inspection.
2. A thin Agent Bridge client when live control is justified.
3. An optional Tauri/SparkJS shell after the evidence workflow is stable.

Every future UI must preserve two rules. The canonical run log remains authoritative. Every
mutation goes through the Agent Bridge.

## 1. Design principles

- Show evidence status before visual polish.
- Distinguish computation status from all four scientific gates.
- Keep replay mode visibly separate from live mode.
- Show hashes, revisions, actors, and artifact identities near derived views.
- Never imply that a missing metric equals zero.
- Never imply that PID is always available or valid.
- Use text and shape in addition to color.
- Keep optional rendering and NCP state out of the default navigation.

The five parts below are design prompts. None is a release commitment.

## 2. Run library

The run library is the first useful product view. It loads local canonical logs and displays their
validation, terminal, provenance, and gate status.

```json
{
  "type": "ui_part",
  "id": "runs_library",
  "title": "Validated Run Library",
  "milestone": "deferred product surface; Rerun-first",
  "requirements": [
    "Show a desktop run library for local canonical run logs.",
    "Display validation, terminal, and scientific-gate badges as separate fields.",
    "Show the source revision, config hash, run id, and artifact count for the selected run.",
    "Offer Open in Rerun, Validate, Compare, and Export actions.",
    "Do not show a cloud dashboard, New Experiment wizard, or automatic scientific verdict."
  ],
  "prompt_seed": "Professional desktop research UI for Prisoma. Dark neutral theme with high contrast and compact spacing. Left navigation has Runs, Replay, Compare, and Settings. The main table lists local canonical run logs with separate columns for validation, terminal status, population gate, measure gate, estimator gate, and application gate. The selected-run panel shows exact revision, config hash, run id, artifacts, and buttons Open in Rerun, Validate, Compare, Export. Clearly label the screen Offline. Avoid decorative simulation imagery.",
  "negative_prompt": "mobile UI, cloud analytics, game HUD, neon, automatic PASS badge, unlabeled PID score, clutter, illegible hashes",
  "image": {"width": 1536, "height": 1024},
  "score_threshold": 9.0,
  "max_iterations": 8,
  "allow_img2img": true
}
```

## 3. Agent Bridge status

This panel describes the future thin client. It must use the exported method contract. Standard
profiles have no authentication. The paired Engram-host profile is a distinct read-only profile.

```json
{
  "type": "ui_part",
  "id": "agent_bridge_panel",
  "title": "Agent Bridge Status and Request Inspector",
  "milestone": "deferred thin control client",
  "requirements": [
    "Show the active local endpoint, transport, safe-mode state, profile, and run id.",
    "List the exact allowed dotted method names from bridge.describe.",
    "Show recent request ids, methods, actors, and response outcomes.",
    "State that standard profiles are unauthenticated and local-only.",
    "If the Engram-host profile is active, show Paired without revealing the startup secret.",
    "Do not show fictional capability toggles, bearer tokens, scene.spawn, or run.pause."
  ],
  "prompt_seed": "Professional Prisoma desktop panel titled Agent Bridge. Show endpoint 127.0.0.1, transport, Safe Mode, active profile, run id, and a compact list of canonical dotted methods. Include a request table with request id, actor, method, and response outcome. Display a clear warning: Standard profile is unauthenticated and local-only. Include a separate Paired indicator only for the read-only Engram-host profile. No secret value, auth token, or fictional capability switches.",
  "negative_prompt": "API key on screen, bearer token, scene.spawn, run.pause, permission toggles, remote cloud endpoint, security shield claim, mobile settings",
  "image": {"width": 1536, "height": 1024},
  "score_threshold": 9.0,
  "max_iterations": 8,
  "allow_img2img": true
}
```

## 4. Replay inspector

The replay view derives all state from one validated run log. It can surface recorded geometry,
events, labels, and diagnostics. The current adapter does not implement this complete panel set.

```json
{
  "type": "ui_part",
  "id": "viewer_replay",
  "title": "Canonical Replay Inspector",
  "milestone": "specified complete Rerun diagnostic view",
  "requirements": [
    "Show a prominent Offline Replay state and the validated run identity.",
    "Use one timeline for events, actions, interventions, labels, and recorded metrics.",
    "Show request and response provenance in an event inspector.",
    "Represent abstained or not-requested metrics as typed states, never zeros.",
    "Show scientific gates separately from computation outcomes.",
    "Offer branch preparation only as an Agent Bridge request, not a direct environment edit."
  ],
  "prompt_seed": "High-fidelity Prisoma desktop replay inspector. Prominent banner Offline Replay. Center shows a restrained recorded-scene viewport placeholder. Bottom has one synchronized timeline with event, action, intervention, label, and metric tracks. Right panel shows request and response provenance plus separate computation and four-gate status. An abstained PID row says Abstained with a reason and no numeric value. A button says Prepare branch through Agent Bridge. Scientific research-tool aesthetic, compact and legible.",
  "negative_prompt": "live control joystick, direct scene editing, zero for abstention, universal PID heatmap, game HUD, fantasy 3D scene, unlabeled colors",
  "image": {"width": 1536, "height": 1024},
  "score_threshold": 9.0,
  "max_iterations": 10,
  "allow_img2img": true
}
```

## 5. Run comparison

Comparison is an optional replay analysis. It must state the alignment and tolerance contract.
Prisoma does not currently implement a complete cross-backend comparison application.

```json
{
  "type": "ui_part",
  "id": "compare_cross_backend",
  "title": "Content-Bound Run Comparison",
  "milestone": "optional deferred replay analysis",
  "requirements": [
    "Show two exact run identities and their validation status.",
    "Show the selected alignment key, replay level, and tolerance profile.",
    "Use a shared timeline with explicit unmatched-event markers.",
    "Separate state, contact, event, and terminal-outcome differences.",
    "Do not present one aggregate divergence score as a replay verdict."
  ],
  "prompt_seed": "Professional Prisoma run comparison screen. Header shows exact Run A and Run B identities, validation state, alignment key, replay level, and tolerance profile. Use synchronized restrained view placeholders and one shared timeline. Mark unmatched events explicitly. Right panel separates state delta, contact mismatch, event mismatch, and terminal outcome. No single overall score. Dark neutral scientific-tool style.",
  "negative_prompt": "single accuracy score, backend winner badge, unqualified percentage, game replay, mobile UI, decorative dashboard",
  "image": {"width": 1536, "height": 1024},
  "score_threshold": 9.0,
  "max_iterations": 8,
  "allow_img2img": true
}
```

## 6. Reconstruction-quality study

This optional E1 design supports a future reconstruction-quality covariate study. It does not
implement weighted PID, information gain, or active-view optimization.

```json
{
  "type": "ui_part",
  "id": "gauss_mi_uncertainty",
  "title": "Reconstruction-Quality Covariate Study",
  "milestone": "optional E1 interface specification",
  "requirements": [
    "Label the screen Optional Study and show the exact reconstruction artifact identity.",
    "Show coverage, held-out residual, and unreliable-region strata with provenance.",
    "Label every candidate view Unscored.",
    "Allow export of a content-bound quality artifact.",
    "Record an accepted capture proposal through the Agent Bridge.",
    "Do not show information gain, effective sample size, weighted MI, or weighted PID."
  ],
  "prompt_seed": "High-fidelity Prisoma optional study screen titled Reconstruction-Quality Covariate Study. Show exact artifact identity and provenance. A restrained 3D reconstruction view uses clearly labeled coverage and held-out residual strata. Candidate viewpoints are all visibly Unscored. Controls export a content-bound quality artifact and record a proposal through Agent Bridge. A boundary note says No information-gain or weighted-PID estimator. Professional scientific UI.",
  "negative_prompt": "information gain number, N_eff, weighted PID, autonomous camera motion, direct capture control, medical dashboard, neon heatmap",
  "image": {"width": 1536, "height": 1024},
  "score_threshold": 9.0,
  "max_iterations": 10,
  "allow_img2img": true
}
```

## 7. Prompt utility

`uidesigner/prompt_loop.py` is an optional operator-run design aid. It extracts these JSON blocks,
generates an image through FAL, and asks Vertex AI for a bounded critique. It does not build UI code.

Run a dry parse without external calls:

```bash
uv sync --locked --group ui
uv run --no-sync python uidesigner/prompt_loop.py --dry-run
```

Run selected parts only after configuring external credentials:

```bash
uv run --no-sync python uidesigner/prompt_loop.py \
  --only runs_library,viewer_replay
```

Set `FAL_KEY` for generation. Set `GOOGLE_CLOUD_PROJECT` and authenticate Application Default
Credentials for critique. Never commit credentials or generated external responses as evidence.

The checked FAL defaults use `fal-ai/gpt-image-1.5` for generation and its separate `/edit`
endpoint for revisions. They accept only `1024x1024`, `1536x1024`, or `1024x1536`.
The Vertex default is the `gemini-3.1-pro-preview` identifier observed on 2026-08-11.
Review both vendors' current model pages before a paid run. Override the endpoints or models when
their lifecycle changes.
