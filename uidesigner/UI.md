# PID‑VLA UI/UX Specification (Docset v10.0)

This file is the **UI contract** for the PID‑Splat viewer/intervention harness described in `grandplan.md` and referenced by the v10.0 `README.md` engineering plan.

The UI is **offline‑first** and **agent‑native**:
- **Offline‑first:** the first usable UI is a **run‑log viewer** (record → replay → analyze) before any live transport.
- **Agent‑native:** the GUI must call the same **Agent Bridge** API that scripts/LLM tools call; every action is logged and replayable.

## 0) Design Principles (Non‑Negotiable)

Aligned to `grandplan.md` §A.7.

1. **Gate-driven:** make Exp0/geometry status visible; avoid “pretty but unscientific” controls.
2. **No hidden state:** everything that changes the run (interventions, branch, config) must show provenance + be in the run log.
3. **One control plane:** the UI is a *client* of Agent Bridge; it cannot do secret local mutations.
4. **Viewer-first:** deliver value before “live mode”:
   - M1: run library + metadata + replay integrity
   - M2: Agent Bridge status + safety (GUI actions are RPC)
   - M4: 3D playback + timeline + overlays
5. **Physics is explicit:** splats are appearance; collisions/contacts come from collision geometry.
6. **Color semantics are explicit:** default overlay convention is **R = Syn⁺**, **G = Red**, **B = Unq(V)** (`grandplan.md` §10.10.4). The UI must always show a legend and allow exporting the mapping.
7. **Accessible & legible:** high contrast, readable typography, colorblind-aware palettes; never rely only on color (use labels/markers).

## 1) Information Architecture (Navigation)

Left navigation (minimal set; everything else is contextual):
- **Runs** (M1): browse/import/export run logs; view metadata; open viewer.
- **Viewer** (M4): playback + overlays + event inspection.
- **Compare** (optional): run‑vs‑run diff (cross‑backend replay is a core use case; `DIAGRAMS.md` §11).
- **Capture** (optional): 3DGS capture utilities; GauSS‑MI uncertainty + view selection (`GAUSS_MI_INTEGRATION.md`).
- **Settings**: local paths, renderer backend, Agent Bridge status, (optional) Zenoh.

## 2) Core UI Objects (What the UI “talks about”)

The UI is anchored on the **run log** (M1). Everything else is derived:
- **Run**: a directory or single artifact containing:
  - config + hashes + provenance
  - time series: `state`, `action`, `embeddings` (`V/L/D/A`), `metrics` (PID/CI), `events` (interventions)
- **Event**: an agent‑bridge RPC call or system event, with `request_id`, timestamps, actor identity, payload hash.
- **Frame**: a replay step (time index) that maps to state + optional renderable snapshots.
- **Overlay**: a mapping from metrics → visuals (must be exported and versioned).

## 3) Screens & Components (Ordered by Engineering Plan)

### 3.1 Runs Screen (M1) — “Run Library”

Purpose: select a run, inspect provenance, open viewer/compare, export artifacts.

**Must-have UI elements**
- Search + filters: experiment (Exp1..Exp5), model, physics backend, date, outcome, tags.
- Run table/list with stable identifiers and quick status badges:
  - Exp0 gate status for that run (GO/PIVOT/NO‑GO) if available
  - physics backend
  - presence/absence of `V/L/D/A`, `Flow_gt`, `Flow_pred`
- Right‑hand details panel for selected run:
  - config hash, code revision, dataset version, actor provenance
  - buttons: **Open Viewer**, **Open Compare**, **Export**

**ASCII sketch (attempt A)**
```
┌──────────────────────────────────────────────────────────────────────────────┐
│ PID‑VLA ─ Runs                                              [Import] [New]   │
├───────┬──────────────────────────────────────────────────────┬───────────────┤
│ NAV   │ Search: [__________]  Filters: Exp ▾ Model ▾ Backend ▾ │ RUN DETAILS  │
│ Runs  │                                                      │ ┌───────────┐ │
│ Viewer│  ┌────────────────────────────────────────────────┐  │ │ Run: #42  │ │
│ Compare│ │ Run ID  Exp  Model   Phys   Date     Outcome   │  │ │ GO: Exp0  │ │
│ Capture│ ├────────────────────────────────────────────────┤  │ │ hashes…   │ │
│ Settings│ │ 42     Exp1 SmolVLA Rapier 2026‑01‑05 fail     │  │ │ artifacts │ │
│       │ │ 43     Exp1 OpenVLA MuJoCo 2026‑01‑05 pass     │  │ │ [Viewer]  │ │
│       │ │ 44     Exp4 …     Rapier 2026‑01‑05 …        │  │ │ [Compare] │ │
│       │ └────────────────────────────────────────────────┘  │ │ [Export]  │ │
└───────┴──────────────────────────────────────────────────────┴───────────────┘
```

**ASCII sketch (attempt B)**
```
┌──────────────┬───────────────────────────────────────────────────────────────┐
│ Runs (M1)     │  [Search runs…]  [Exp ▾] [Model ▾] [Backend ▾] [Tags ▾]      │
├──────────────┼───────────────────────────────────────────────────────────────┤
│ Recent        │  ▸ Run 44  Exp4  Flow_gt ✓  PID ✓  outcome: —                 │
│ Starred       │  ▸ Run 43  Exp1  Flow_gt ✗  PID ✓  outcome: pass              │
│ Imported      │  ▸ Run 42  Exp1  Flow_gt ✓  PID ✓  outcome: fail              │
│              │                                                               │
│              │  Selected: Run 42                                             │
│              │  - revision: abc123   config_hash: …                          │
│              │  - actors: human_gui, script                                  │
│              │  [Open Viewer] [Open Compare] [Export]                        │
└──────────────┴───────────────────────────────────────────────────────────────┘
```

```json
{
  "type": "ui_part",
  "id": "runs_library",
  "title": "Runs Screen (Run Library)",
  "milestone": "M1",
  "requirements": [
    "Desktop app screenshot (not mobile).",
    "Left navigation includes: Runs, Viewer, Compare, Capture, Settings.",
    "Main area shows a run list/table with filters (experiment, model, physics backend, date, outcome).",
    "Selected run details panel shows provenance (hash/revision) and buttons: Open Viewer, Compare, Export.",
    "Design is offline-first: shows local run logs / imports; no cloud dashboards."
  ],
  "prompt_seed": "High-fidelity product UI mockup of a cross-platform desktop app called PID-VLA. Dark theme, clean typography, high contrast. Left sidebar navigation: Runs, Viewer, Compare, Capture, Settings. Main content is a Run Library: search bar and filter chips (Experiment, Model, Physics backend, Date, Outcome). A table of runs with columns Run ID, Experiment, Model, Physics, Date, Outcome. Right-hand details panel for selected run showing config hash/revision, artifact badges (Flow_gt, PID metrics), and buttons Open Viewer, Open Compare, Export. Minimalistic, modern, realistic spacing, legible text.",
  "negative_prompt": "low resolution, blurry, illegible text, mobile UI, browser UI, neon, clutter, random charts without labels",
  "image": {"width": 1600, "height": 1000},
  "score_threshold": 9.0,
  "max_iterations": 8,
  "allow_img2img": true
}
```

---

### 3.2 Agent Bridge Panel (M2) — “API & Safety”

Purpose: expose local control plane status and make the UI obviously scriptable.

**Must-have UI elements**
- Status: listening address, auth token, connected clients.
- Safety mode: read‑only by default for external sessions; capability toggles.
- Recent RPC calls list (request id, method, actor).

**ASCII sketch (attempt A)**
```
┌──────────────────────────────────────────────────────────────┐
│ Agent Bridge (Local)                                         │
├──────────────────────────────────────────────────────────────┤
│ Status: ● running   ws://127.0.0.1:9123   Token: [copy]       │
│ Clients: UI(connected), script(disconnected), llm(disconn)    │
│ Safe mode: [ON]  External default: read-only                  │
│ Capabilities: [scene.edit] [run.control] [export] [network ✗] │
│ Recent calls:                                                  │
│  - req_91  scene.spawn   actor: human_gui                      │
│  - req_92  run.pause     actor: script                         │
└──────────────────────────────────────────────────────────────┘
```

**ASCII sketch (attempt B)**
```
┌──────────────────────────────────────────────────────────────────────┐
│ Settings ▸ Agent Bridge                                               │
├───────────────────────────────┬──────────────────────────────────────┤
│ Endpoint: ws://127.0.0.1:9123 │ Clients: UI ✓  script ✓  llm ✓       │
│ Token:  *************** [Copy]│ Safe mode: ON (external read-only)   │
├───────────────────────────────┴──────────────────────────────────────┤
│ Capabilities: [x] scene.edit  [x] run.control  [x] export  [ ] net    │
│ Recent RPC: req_91 scene.spawn  req_92 run.pause  req_93 export.bundle│
└──────────────────────────────────────────────────────────────────────┘
```

```json
{
  "type": "ui_part",
  "id": "agent_bridge_panel",
  "title": "Agent Bridge Panel (API & Safety)",
  "milestone": "M2",
  "requirements": [
    "A panel or modal titled Agent Bridge with local endpoint and a copy-token control.",
    "Shows connected clients and recent RPC calls with request ids.",
    "Shows safe-mode defaults (read-only external sessions) and capability toggles.",
    "Consistent visual style with the rest of the app."
  ],
  "prompt_seed": "High-fidelity desktop app settings panel titled 'Agent Bridge' for PID-VLA. Dark theme. Shows status (running), local WebSocket endpoint ws://127.0.0.1:9123, an auth token field with a Copy button, a list of connected clients (UI, script, llm tools), Safe Mode toggle (ON) with note 'external default: read-only', capability toggles (scene.edit, run.control, export; network disabled), and a small table of recent RPC calls with request_id and actor. Clean and readable.",
  "negative_prompt": "terminal screenshot, code editor, mobile settings screen",
  "image": {"width": 1400, "height": 900},
  "score_threshold": 9.0,
  "max_iterations": 8,
  "allow_img2img": true
}
```

---

### 3.3 Viewer Screen (M4) — “Offline Playback + Diagnostics”

Purpose: replay a run deterministically, inspect events, overlays, and metrics, and create branches for interventions.

**Layout (baseline)**
- **Top toolbar**: open run, play/pause, step, speed, overlay toggles, export.
- **Center**: 3D viewport (splats + meshes) with overlay legend.
- **Bottom**: timeline scrubber + event markers + optional metric strip charts.
- **Right inspector**: selected object/splat, pose/collider, per-frame metrics, event details.
- **Left scene tree** (optional in MVP): objects, cameras, overlay layers.

**Must-have UI elements**
- “You are in replay mode” indicator; “branch from here” action creates a new run.
- Event list with provenance (actor type, request id).
- Overlay legend (R=Syn⁺, G=Red, B=Unq(V)) + toggle.

**ASCII sketch (attempt A)**
```
┌──────────────────────────────────────────────────────────────────────────────┐
│ PID‑VLA Viewer  Run: #42  [⏮] [⏯] [⏭] speed:1.0x  Overlays ▾  Export ▾       │
├───────┬──────────────────────────────────────────────────────┬───────────────┤
│ Scene │                                                      │ Inspector     │
│ ▸ obj │   ┌──────────────────────── 3D VIEW ───────────────┐ │ Selected: cup │
│ ▸ cam │   │ splats + mesh robot + PID overlay (legend)     │ │ pose: …       │
│ ▸ ovl │   └────────────────────────────────────────────────┘ │ collider: …   │
│       │                                                      │ PID @cursor:  │
│       │                                                      │ Syn: … Red:…  │
├───────┴──────────────────────────────────────────────────────┴───────────────┤
│ Timeline: |■■■●■■■■■■■■■■■■|  Events: ▲ ▲   Charts: Syn/Red/Unq vs time       │
│ [Branch from here]  [Jump to event]  [Annotate failure]                        │
└──────────────────────────────────────────────────────────────────────────────┘
```

**ASCII sketch (attempt B)**
```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Viewer (Replay)  Run 42  Backend: Rapier  Exp:1  Model:SmolVLA               │
│ [Play] [Step] [Pause@checkpoint]  Overlay: PID ▣  Uncertainty ▢  Export ▾   │
├──────────────────────────────────────────────────────────────────────────────┤
│ 3D View (center)                          │ Event Log (right)                │
│ ┌───────────────────────────────┐         │ - t=12.4s  rpc: scene.move       │
│ │ (splats + meshes)             │         │ - t=13.0s  rpc: perturb.friction │
│ │ legend: R Syn+ G Red B Unq(V) │         │ - t=13.2s  failure_label: miss   │
│ └───────────────────────────────┘         │                                  │
├──────────────────────────────────────────────────────────────────────────────┤
│ Timeline + markers + mini charts (Syn/Red/Unq, success prob)                 │
└──────────────────────────────────────────────────────────────────────────────┘
```

```json
{
  "type": "ui_part",
  "id": "viewer_replay",
  "title": "Viewer Screen (Offline Replay + Diagnostics)",
  "milestone": "M4",
  "requirements": [
    "Desktop app screenshot with a large central 3D viewport and a bottom timeline scrubber.",
    "Top toolbar includes play/pause/step, speed, overlay toggles, export.",
    "Visible legend: R=Syn+, G=Red, B=Unq(V) (or equivalent explicit legend).",
    "Right panel shows Inspector and/or Event Log with provenance (request id / actor).",
    "A clear call-to-action for branching from a replay checkpoint (e.g., 'Branch from here').",
    "Design emphasizes offline replay mode (no live streaming implied)."
  ],
  "prompt_seed": "High-fidelity product UI mockup of a cross-platform desktop app called PID-VLA in Viewer (Replay) mode. Dark theme, modern and minimal. Top toolbar: Play/Pause, Step, speed dropdown, overlay toggles, Export. Center is a large 3D viewport placeholder showing splats + mesh robot with a subtle PID overlay. Always-visible legend: R=Syn+ G=Red B=Unq(V). Bottom has a timeline scrubber with event markers and small strip charts for Syn/Red/Unq over time. Right side has an Inspector + Event Log listing interventions with actor type and request id. Include a prominent 'Branch from here' button near the timeline. Crisp layout, readable text, realistic spacing.",
  "negative_prompt": "mobile UI, web browser chrome, tiny illegible labels, messy rainbow colors, fantasy sci-fi UI",
  "image": {"width": 1800, "height": 1100},
  "score_threshold": 9.0,
  "max_iterations": 10,
  "allow_img2img": true
}
```

---

### 3.4 Compare Screen (Cross‑Backend Replay) — “Run A vs Run B”

Purpose: compare two runs (often Rapier vs MuJoCo replay) and quantify divergence (`grandplan.md` §E.1; `DIAGRAMS.md` §11).

**Must-have UI elements**
- Side‑by‑side synchronized viewports or “difference mode”.
- Shared timeline with two traces and divergence overlays.
- A divergence summary panel (state error, contact mismatch rate, success mismatch).

**ASCII sketch (attempt A)**
```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Compare  Run A: #42 (Rapier)   Run B: #42b (MuJoCo)   [Sync] [Export report]  │
├───────────────────────────────┬───────────────────────────────┬──────────────┤
│ View A (left)                 │ View B (right)                │ Divergence   │
│ ┌───────────────────────────┐ │ ┌───────────────────────────┐ │ pose Δ: …    │
│ │ (viewport A)              │ │ │ (viewport B)              │ │ contacts:…   │
│ └───────────────────────────┘ │ └───────────────────────────┘ │ success: …   │
├──────────────────────────────────────────────────────────────────────────────┤
│ Timeline (shared) + markers + divergence plot                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

**ASCII sketch (attempt B)**
```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Compare ▸ A: Run 42 (Rapier)  vs  B: Run 42b (MuJoCo)   Mode: Diff ▾  [Sync]  │
├──────────────────────────────────────────────────────────────────────────────┤
│ Diff View (ghosted overlays)                       │ Divergence (summary)     │
│ ┌───────────────────────────────────────────────┐  │ pose Δ peak: t=13.0s    │
│ │ red = A, blue = B, purple = overlap          │  │ contacts mismatch: 18%   │
│ │ (single viewport; toggle A/B/diff)           │  │ success mismatch: YES    │
│ └───────────────────────────────────────────────┘  │ [Jump to peak] [Report] │
├──────────────────────────────────────────────────────────────────────────────┤
│ Timeline: |■■●■■■■■■|   A trace   B trace   divergence trace                  │
└──────────────────────────────────────────────────────────────────────────────┘
```

```json
{
  "type": "ui_part",
  "id": "compare_cross_backend",
  "title": "Compare Screen (Cross-Backend Replay)",
  "milestone": "M6+",
  "requirements": [
    "Desktop compare screen with two synchronized viewports (Run A vs Run B).",
    "Visible labels for physics backend per side (e.g., Rapier vs MuJoCo).",
    "Shared timeline scrubber and a divergence summary panel (pose/contact/success).",
    "Design is consistent with Viewer screen styling and emphasizes reproducible comparison."
  ],
  "prompt_seed": "High-fidelity product UI mockup of PID-VLA in Compare mode. Dark theme. Top bar shows Run A (Rapier) vs Run B (MuJoCo) with Sync enabled. Two large synchronized 3D viewports side-by-side, each labeled with backend and run id. Right-side panel titled Divergence Summary with metrics like pose delta, contact mismatch, success mismatch, and a small divergence chart. Bottom shared timeline scrubber with markers and a divergence plot. Clean, professional, readable typography.",
  "negative_prompt": "split-screen video editor, mobile UI, gaming HUD, clutter",
  "image": {"width": 1800, "height": 1100},
  "score_threshold": 9.0,
  "max_iterations": 8,
  "allow_img2img": true
}
```

---

### 3.5 GauSS‑MI Capture & Uncertainty Screen (M8, Optional)

Purpose: treat 3DGS reconstruction quality as a confound control (`grandplan.md` §C.2; `GAUSS_MI_INTEGRATION.md`).

**Must-have UI elements**
- Uncertainty overlay toggle + legend.
- “Suggested next viewpoints” around the scene (active view selection).
- Export uncertainty artifacts (`SceneUncertaintyMap`) and view plan as run‑log interventions.

**ASCII sketch (attempt A)**
```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Capture / Uncertainty (GauSS‑MI)   Scene: table_v1   [Recompute] [Export]     │
├───────────────────────────────┬───────────────────────────────┬──────────────┤
│ 3D View + uncertainty overlay │ Suggested viewpoints           │ Stats        │
│ ┌───────────────────────────┐ │ - cam_01: +15° orbit (IG +0.8) │ mean σ: …    │
│ │ (splats colored by σ)     │ │ - cam_02: top-down (IG +0.6)   │ N_eff: …     │
│ │ legend: low→high          │ │ - cam_03: side (IG +0.4)       │ unreliable:… │
│ └───────────────────────────┘ │ [Send to Agent Bridge]         │              │
└──────────────────────────────────────────────────────────────────────────────┘
```

**ASCII sketch (attempt B)**
```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Capture (Optional)  Scene: table_v1   Overlay: Uncertainty ▣  Views: 12       │
├──────────────────────────────────────────────────────────────────────────────┤
│ 3D View (splats colored by σ)                 │ Next views (plan as events)   │
│ ┌──────────────────────────────────────────┐  │ 1) orbit +15°  IG +0.8       │
│ │ legend: low σ → high σ                   │  │ 2) top-down    IG +0.6       │
│ │ toggle: show unreliable gaussians only   │  │ 3) side        IG +0.4       │
│ └──────────────────────────────────────────┘  │ [Send plan] [Export plan]    │
├──────────────────────────────────────────────────────────────────────────────┤
│ Stats: N_eff …  unreliable frac …  residual@heldout …  [Re-run gate]          │
└──────────────────────────────────────────────────────────────────────────────┘
```

```json
{
  "type": "ui_part",
  "id": "gauss_mi_uncertainty",
  "title": "Capture/Uncertainty Screen (GauSS‑MI, Optional)",
  "milestone": "M8",
  "requirements": [
    "Desktop screen titled Capture/Uncertainty (GauSS-MI).",
    "Central viewport shows a 3DGS scene with an uncertainty overlay and a clear legend.",
    "A panel lists suggested next viewpoints with estimated information gain values.",
    "A stats panel shows N_eff and fraction unreliable; export actions are present.",
    "Design communicates this is optional and diagnostic (not a required runtime dependency)."
  ],
  "prompt_seed": "High-fidelity product UI mockup of PID-VLA in a Capture/Uncertainty mode labeled 'GauSS-MI (Optional)'. Dark theme. Left/center large 3D viewport showing a gaussian-splat scene with an uncertainty heat overlay and a clear legend (low to high uncertainty). Right panel lists 'Suggested next viewpoints' with 3-5 camera candidates and an estimated information gain score for each, with a button 'Send plan to Agent Bridge'. Another panel shows uncertainty stats including N_eff and fraction unreliable, plus Export buttons for SceneUncertaintyMap. Clean, scientific tool aesthetic, readable text.",
  "negative_prompt": "medical UI, finance dashboard, illegible labels, noisy gradients",
  "image": {"width": 1800, "height": 1100},
  "score_threshold": 9.0,
  "max_iterations": 10,
  "allow_img2img": true
}
```

---

## 4) Prompt‑Iteration Workflow (How the scripts use this file)

Each `ui_part` JSON block is machine‑readable. The `uidesigner/prompt_loop.py` script:
1. Extracts the `requirements` and `prompt_seed`.
2. Calls **gpt‑image‑1.5 via FAL** to render an image.
3. Sends the image + requirements to **Gemini (Vertex AI)** for critique + a numeric score.
4. Uses Gemini to propose a revised prompt and loops until the score threshold is met.

**Output convention**
- Writes artifacts under `uidesigner/out/<UTC timestamp>/<ui_part.id>/` and a session manifest at `uidesigner/out/<UTC timestamp>/session.json`:
  - `iter_01.prompt.txt`, `iter_01.png`, `iter_01.review.json`, `iter_01.fal.json`
  - … up to `max_iterations`
  - `best.png`, `best.prompt.txt`, `best.review.json`

**Quick usage**
```bash
python3 uidesigner/prompt_loop.py --dry-run
python3 uidesigner/prompt_loop.py --only runs_library,viewer_replay
```

**Required config**
- FAL: set `FAL_KEY` (and optionally `FAL_ENDPOINT`).
- Vertex AI: set `GOOGLE_CLOUD_PROJECT` (and optionally `GOOGLE_CLOUD_LOCATION`, `GEMINI_VISION_MODEL`, `GEMINI_TEXT_MODEL`) and authenticate via ADC (`gcloud auth application-default login`) or a service account.
