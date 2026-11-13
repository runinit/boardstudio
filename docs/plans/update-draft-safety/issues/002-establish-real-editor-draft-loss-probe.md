# 002: Establish a real-editor draft-loss probe

Status: resolved
Type: task
Label: planning:task
Mode: AFK
Assignee: Chris D
Blocked by: none
Parent: [Draft-safe update adoption](../map.md)
Related: [Draft-safety research and change map](../../../research/2026-11-07-019-draft-safety-change-map.md)

## Question

At the actual packaged Update Now entry points, what happens to an otherwise-settled
project's real explicit-submit draft, and what observation seam makes the failure or
protection unambiguous? This factual probe informs the interface and regression plan;
it does not implement a fix or run the full acceptance matrix.

## Evidence to obtain

- Reuse the packaged A/B driver and a disposable saved fixture, initially choosing
  the Geometry scripts or Layout constraint form from the existing source evidence.
  A/B must be distinguishable as loaded applications, not just worker/server labels.
- Change a real valid value without Apply/Save. Record draft text, accepted/saved
  value and revision, waiting worker, focus and trusted pointer or keyboard sequence.
  Ensure another invalid field or pending operation is not the accidental blocker.
- Observe initial deferral/cancellation or actual B reload and loss/retention of that
  exact draft. Record events/admission ordering where relevant; do not assume synthetic
  `.click()` proves blur behavior. Do not inject `aria-invalid` as the draft substitute.
- Name the intended failing regression assertion and a follow-up case for the other
  entry point/owner. A source counterexample is not a browser reproduction; an
  unexpected safe result needs its actual owner/blocker explained.

## Completion and boundaries

Return a reproducible observation, identities/commands and linked logs, or an exact
blocker with the next materially different experiment. Leave the ticket unresolved
when blocked; a blocker transfer is not completed evidence. Use `agent-browser`; keep the
live preview, projects, workers and caches untouched. Builds stay serialized per target.
Narrow test-only instrumentation is allowed only if needed for this question; document
it and do not change production behavior or widen a public interface without approval.
No product fix, no discard permission, and no complete-safety claim. Claim first.

## Answer

Resolved 2026-11-07. **A real valid explicit-submit draft is lost on packaged
A→B adoption: the gate neither defers nor warns.** Reproducible observation
(log: `.evidence/update-draft-safety/002-probe.log`; fresh `--fast
--skip-providers --routes root,subpath` build of the current dirty tree:
`.evidence/update-draft-safety/002-build.log`; disposable probe driver
`/tmp/bs002/probe002.py` reusing `scripts/verify-update-adoption.py` stamping,
server, session and state payloads; fresh isolated `agent-browser` profile,
demo copy only, port 4733 owned and freed):

- Stamped A/B are distinguishable as loaded applications, not worker labels:
  packaged diff is exactly `index.html` + `service-worker.js` + page wasm per
  route; after adoption the loaded document meta and the application binary
  served to that page both read B, navigation type is `reload`, and only B's
  versioned cache survives (root A `boardstudio-9ef37ed793933307-…`, B
  `boardstudio-8b2afa895e232839-…`).
- Settled fixture: in build A, Add object → Board geometry → Geometry
  scripts… → `+ New script`, Name `Probe script 002`, Rhai source
  `// probe 002 baseline`, **Apply script**, then `Saved locally` observed —
  the accepted/saved baseline. Draft: Rhai source changed to
  `// probe 002 baseline\n// DRAFT-UNAPPLIED-002` **without Apply**. Recorded
  at activation: exact draft text in the field, `aria-invalid=true` count 0
  (no accidental blocker), save label still `Saved locally`, focus in
  `Rhai source`, then one trusted Tab moved focus to `Enable on Apply` and
  the draft survived (no blur commit).
- With B genuinely waiting (`waiting=installed`, document still A, load
  marker unchanged, draft still present), a trusted pointer click on the
  **toast Update Now** produced **no deferral dialog**: the page reloaded
  onto B. Reopening the saved project and the script shows Name
  `Probe script 002`, source `// probe 002 baseline` — the draft line is
  gone. This confirms the source-level gap in
  `GeometryScriptsEditor` (`web/crates/ui-shared/src/geometry_scripts.rs::GeometryScriptsEditor`:
  local name/source signals, only Apply script submits) is a real A→B loss,
  not only a detection-gap inference.

Observation seam that made this unambiguous: the driver's `state.js`
(build meta, load marker, waiting/controller, caches) plus reading the real
editor's Name/Rhai field values before activation and after reopening, and
`loaded-binary.js` for the served application binary. No synthetic
`.click()`, no injected `aria-invalid`, no product or test-hook changes.

**Intended failing regression assertion** (for the later implementation
ticket, not run as a product test here): settled saved project with an
applied Geometry script; change Rhai source without Apply; Tab away;
B waiting; click toast Update Now → expect a cancel-only deferral dialog,
load marker unchanged and document still A, and Cancel preserving the exact
draft text. Today it fails at the first expectation: adoption reloads and
the draft is lost. **Follow-up cases to name in the verification plan:**
the same draft through the header Update Now entry point (after toast
dismissal) and through the Layout constraint form owner; neither was run
here — the toast run already consumed the waiting update.

No new diagnostic question graduates: the probe exposed the expected
failure path, no materially different one. No product code changed; builds
and browser ran only on the disposable stamped copies/profile.
