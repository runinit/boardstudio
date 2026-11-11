# Delivery contract: the real Layout redesign first

Parent: [map.md](map.md). This is the coordinator's execution contract, not a new design.
The user instructed: **“Execute the recommendations and then handoff to new thread for
coordination and implementation.”** This authorizes the bounded planning corrections and
coordination/implementation of M1 below, followed by a stop for hands-on review. It does not
authorize later workspace phases, production rollout, merging to main, or new domain policy.
The four frozen design references remain unchanged.

## Milestones and authority

1. **M1 — complete, usable Layout redesign in the actual app.** Implement 009 → 002 →
   003/004 → 005, with 006 after 009 and before 005. This combines the former foundation
   and Layout phases into one authorized delivery milestone. Internal ticket checkpoints
   are not new execution phases. Keep the same geometry, edit, selection and export owners.
2. **M2 — the other five workspace adapters.** 010–014 and the separately gated Case
   Preview contract 015. Prepare their outstanding decisions, but do not implement them
   until the user approves M2 after trying M1. Six-workspace scope is retained.
3. **M3 — six-workspace acceptance and old-dock retirement.** 007/008 only after all adapters.
   M1 does not certify or close those tickets.

M1 must include the 42px measured sliding Layout accordion, Objects and Inspect utilities,
compact Outline-first Objects, unpinned Inspector, and every retained Add/setup/placement/
legacy entry. It is not complete at an empty host, prototype, screenshots, or mocked controls.
Global topbar work in 006 must retain project/Export navigation and save honesty while the
other workspaces keep their current shell. Each workspace has exactly one panel authority.

## M1 implementation defaults and deferred decisions

The execution instruction adopts the already recommended conservative M1 path, not every
new question raised by the reviewer:

- Project → Add object… plus Command palette reach existing controllers and previews;
  reuse current setup/process/file-picker owners. No new wizard or scripting capability.
- Use architecture.md's proposed 42px, fixed-trigger/action-scroll, clamping, keyboard and
  session-draft literals as M1's concrete implementation target. Measure the real built CSS;
  an unmet mandatory accessibility requirement blocks, rather than being waived.
- Preserve each field's existing commit trigger. A real valid blur commits exactly once to
  its captured live owner; hiding without that trigger retains the owner's text, dirty/error
  and admission state. No synthetic commit, rebinding to the next selection, or silent loss.
- Explicit-submit unresolved owners already protected by the runtime remain protected while
  parked; retain their signals and registration together. Intentional Cancel keeps its existing
  semantics. Do not invent a new pending-close veto where the existing owner allows closing.
- Every Objects pick still closes it, including Add/Range/Toggle; the selection operation
  remains unchanged. Keeping modified picks open would be a new product decision.
- No opacity effects in the new toolbar, labels or panel entry/exit. Do not restyle unrelated
  shared menus/tooltips across the app; any nested animation that violates the frozen
  workbench's solid transition must be handled at the new composition, not globally waived.
- Work on the existing branch/worktree. No automatic merge of mixed old/new shells to main.

Keycaps-default relocation, Case Preview's interface/target sizing, and the exact treatment
of currently unregistered Keymap/Keycaps/Case drafts belong to M2's human gates. Classify
actual commit modes before proposing registrations; absence of a registration alone is not
proof of an explicit-submit draft or of a runtime defect. Do not block M1 on these choices.

## M1 acceptance and proof ownership

Use the existing ticket proof maps; the following is a delivery checklist, not duplicate tests.
All implementation evidence is **planned** until executed. Reuse sufficient assertions with
receipts; keep independently breakable timings separate.

| Requirement | Owning proof / independent oracle |
|---|---|
| Real styled 42px accordion with solid motion | 002: built Theme/toolbar stylesheet and fonts loaded; final measured fit before motion; no relative icon/label drift or opacity effect; reduced motion instant |
| Objects/Inspector selection and draft safety | 003/004: active-outline and half-focus invariants; ordinary picks do not open Inspector; captured-owner text/error survives; no geometry/history side effect |
| Retained creation and task workflows | 005: real MatrixSetup/legacy owner and preview, one-shot admission and existing Cancel, hidden-draft Update Now deferral, and workspace/session lifetime |
| Header/reveals and mixed-shell navigation | 006: all 13 helper sites plus non-helper reveals inventoried; Export review routes and explicit shortcuts use the target workspace's sole authority |
| Keyboard, narrow, contrast and frozen-direction fidelity | M1 portion of 007 criteria: 320/390/728/desktop, short viewport/200% zoom, both Themes/Appearances, real-page Escape/focus/targets, measured text/icon/SVG pairs |
| Cross-workspace non-regression | 005/006: Layout new host → Parts/PCB old host → Layout, correct current commands and retained return/query/owner; no duplicate host or invisible focus |

009 closes its generic interface/mechanics with labelled content through the production
private interface. M1 cannot close until 002 proves the real styled Layout consumer. Actual
registered-Inspector proofs are 003 and task-admission/registered-task proofs are 005,
not a circular dependency in 009. No widening production visibility for test-only fixtures.

M1 runs these named 007 rows for Layout and the global header: **Width/zoom, Focus/touch,
Rendered contrast, Packaged visual parity and Reduced motion**. **Adapter integration**
runs the Layout/project/Export workflows and the specified mixed-shell browse/return routes;
the five other migrated adapters remain M2/M3 work. Record receipts and limitations in the
owning M1 tickets and summarize them in the M1 result below. 007 stays open for its later
all-workspace evidence; reuse unchanged M1 receipts there rather than rerun duplicate tests.

### M1 result (coordinator-owned)

Not implemented. All production proof rows remain planned. Replace this paragraph with the
runnable candidate, owning-ticket receipts, stable full-gate result, keyboard/contrast/visual
measurements and remaining limitations before the hands-on stop.

## Coordination, verification and stop

The new coordinator is the sole shared-host/routing writer and full-build/suite owner. Assign
only disjoint domain/CSS files to delegates and require changed-file/ownership/evidence reports.
Use native Gortex impact/verify/edit/detect/tests/guards/contract. Before implementation, read
001's disposition and the completed single combined delta recheck. Its sole mandatory missing
case is now in 003 and verification.md: hidden registered Inspector safety is independent of
005's task safety. The reviewer required author diff-read and doc checks for that added case,
not another review round. Do not relabel the historical Spec verdict as PASS or weaken proof
rows. Normal implementation-ticket reviews still apply; do not restart the planning loop.

For M1: affected quick/lint/mounted checks and ticket mutations, then one stable-candidate
`python3 scripts/check.py` integrated gate (including packaged browser/build), and agent-browser
hands-on keyboard/visual verification. Coordinator reruns delegated reject paths independently.
No concurrent Cargo/wasm-pack owners or overridden build flags; logs only in `.evidence/`.
The full M3 gate still runs after retirement; do not claim M1 ran all future adapter acceptance.

Deliver a runnable real-app candidate, evidence and remaining limitations; stop for the user's
hands-on review before M2. Do not start a preview/server solely to display this handoff.
