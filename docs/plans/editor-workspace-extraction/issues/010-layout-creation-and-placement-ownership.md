# 010: Give Layout ownership of creation and placement workflows

Status: ready-for-agent
Type: build
Blocked by: [005: Parts routes](005-parts-browsing-attachment-and-routes.md), [009: Layout selection and Inspector ownership](009-layout-selection-and-inspector-ownership.md)
Parent: [001: Editor workspace ownership specification](001-editor-workspace-ownership.md)
Related: [Plan and verification contract](../map.md), [ownership research](../../../research/editor-workspace-extraction.md)

## What to build

Make Layout own complete matrix, mirrored-pair and component creation/placement
orchestration while preserving Parts, PCB and guide entry points, preview, commit,
cancel and exact post-Landed selection.

## Scope and constraints

- Move creation/placement mounts, action admission, source/destination capture,
  eligibility masking, preview projection and workspace/panel follow-ups together.
- Reuse existing feature controllers and the one shared canvas arbiter; do not move
  destination transactions into Parts or create one arbiter per workflow.
- Keep operation-specific captured scope/generation/token/revision checks and exact
  accepted/saved-result checks. Observation retirement does not cancel submitted work.
- Preserve current ports to Parts, PCB and guide, including consumers not yet moved
  into the guide module. Ticket 007 need not complete first because it uses the same
  capabilities, not a new placement implementation.

## Acceptance criteria

- [ ] The existing Layout module owns creation/placement controller wiring, admission, previews and follow-up actions; Editor supplies only shared authorities/ports.
- [ ] A real Parts component action reaches Layout 2D, displays its preview and commits through pointer or Enter with the intended accepted membership/pose and selection.
- [ ] A real matrix assembly placement and mirrored-pair creation preserve captured target checks, selection and panel behavior, with existing Undo transaction boundaries.
- [ ] PCB controller choosing and guide Layout-stage entry use the same destination workflow; current source/query/guide ownership survives intended workspace transitions.
- [ ] Matrix setup and mirrored-pair setup retain mutual eligibility masking, and part/outline/transform workflows cannot claim the canvas concurrently.
- [ ] Escape/cancel, owner departure and 3D-mode transition retain their current preview cleanup and no-edit behavior; cancellation does not accidentally commit.
- [ ] Hold preparation or saving, replace owner/project or actually unmount, then release; no obsolete selection/navigation/focus follows, and permitted Session-owned work still settles once.
- [ ] One-shot activation remains disabled while pending and a double activation cannot create duplicate parts or pairs.
- [ ] Shared placement/setup capabilities remain usable by already-moved Parts and later guide ownership without exposing internal feature controllers.

## Verification

Run documented quick checks, page-only WASM compilation, the page-bin browser suite
and existing matrix/pair/part-placement tests. Drive at least one actual Parts action
through Editor's canvas commit and cancellation paths; hook-only geometry and event
probes remain supporting coverage, not proof of page wiring. Use the shared contract.
