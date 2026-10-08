# Map: Module deepening

Label: wayfinder:map
Spec: [spec.md](spec.md) · Parallel runs: [handoff.md](handoff.md) · Source: [architecture review 2026-10-07](../../investigations/architecture-review-2026-10-07.html)
(all six candidates)

## Destination

One deep module per concept, each tested through its interface: pending-edit
settlement for every panel, a native test Runtime on the real Session and Core, export
leases, per-workspace page state, and (through [typed Core edits](../typed-core-edits/map.md))
mechanical settings and outline version rules in Core.

## Notes

- Domain terms: [CONTEXT.md](../../../CONTEXT.md) (accepted document, draft value,
  pending edit, landed edit, field edit, one-shot action, edit preview).
- Skills: `codebase-design` for interface and seam questions, `grilling` +
  `domain-modeling` for decision tickets, `tdd` for build tickets, `code-review`
  before resolving a build ticket.
- This map carries build execution under [the handoff](handoff.md), as well as
  decision planning. The current split preserves the already approved settlement
  policy; human approval is still required for the separate typed Core proposals.
- Tracker conventions: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
- Line numbers in tickets are at `915d305c0`, orientation only.
- Text search: `rg` (skips the stale `.claude/worktrees/` copies).

## Order

```text
ES-20 ──┬── native Runtime ───────────────────────────────────────────┐
        ├── outline split ──┬───────────────────────────────────────┤
        └── typed wiring ───┴── resolution constructors ── ticket Scope
                                                            │
                                                       keyed PendingEdits
                                                            ├── shared UI helpers ─┐
                                                            └── Matrix actions ────┤
                                                                                  │
                                                                          Matrix fields
                                                                                  │
                                                                        tracer gate (05)
                                                                                  ├── Layout panels (also outline split, workspace state)
                                                                                  ├── Parts + PCB
                                                                                  └── Case, Keymap, Keycaps, Library
independent export leases
            Editor tracer ── Editor workspace state
cross-effort typed wiring ── mechanical patch (approved spec required)
             typed wiring + outline split ── outline intents (human)
independent Parts/PCB native-test preparation (native Runtime + constructors) ── Parts + PCB
finish       cleanup after the three panel migrations, export leases and workspace state
```

ES-20 is [edit settlement 20](../edit-settlement/issues/20-preview-only-direct-edit-event.md);
TCE-nn are [typed Core edits](../typed-core-edits/map.md) tickets. Preview-only events,
the native Runtime, resolution constructors, outline split, typed wiring, export leases
and Editor state are resolved. Captured Scope, keyed settlement, shared UI helpers,
Matrix actions and Matrix fields are merged and resolved. The complete tracer gate
is resolved; Case/Keymap/Keycaps/Library and the PCB half are accepted. Layout and
Parts remain unblocked.
Mechanical patch and outline intent proposals await human approval.

## Decisions so far

- All six review candidates are in scope, including the `Editor` decomposition now
  rather than alongside feature work (user, 2026-10-07).
- The Core moves (mechanical settings, outline versions) run as typed Core edits
  tickets after `SetWiringMode`, not in this effort (ADR-0006 direction).
- Pending-edit settlement: a retired edit is silent; no "Saved" status; the ticket
  answers lineage liveness from its captured `Scope`; the latest ticket per field
  drives it; panels place failure messages; `PendingEdits` in `runtime`, Signal
  helpers in `ui-shared`; resolver boilerplate is its own ticket:
  [01](issues/01-decide-pending-edit-settlement.md#answer),
  [ADR-0005 amendment](../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07).
- Native test Runtime: real Session and Core, one shared gate driver for native and
  wasm adapters, a read-only event log, no hand-settling; ticket 03 deletes the
  parallel Session implementations, panel hand-resolve tests move with their panel:
  [02](issues/02-decide-native-test-runtime.md#answer).

## Progress

<!-- one line per resolved build ticket -->

- [Case, Keymap, Keycaps and Library settlement](issues/09-case-keymap-keycaps-library-onto-pending-edits.md#outcome):
  all consumers use shared settlement and real field bindings; final native/browser
  checks and both reviews accepted, with the CAD baseline reported separately.

- [PCB half accepted](issues/08-parts-and-pcb-onto-pending-edits.md#pcb-half-outcome):
  native 23 and mounted 48 pass; typed draft and bounded action gaps are closed.
  Parent 08 remains claimed until the separate Parts half is accepted.

- [PendingEdits and Matrix tracer integration gate](issues/05-pending-edits-module.md#outcome):
  the reviewed complete tracer passes combined verification and opens the three
  independent panel migration streams.

- [Matrix fields complete the PendingEdits tracer](issues/18-matrix-field-pending-edits.md#outcome):
  one helper owns field/action observation; native 110/8/51 and the complete browser
  gate passed, with the documented native CAD baseline reported separately.

- [Matrix one-shot actions use PendingEdits](issues/17-matrix-one-shot-pending-edits.md#outcome):
  keyed actions retain precise selection and retire departed-owner reports silently;
  native 51 and browser 114+1 tests passed, final reviews clear.

- [Shared UI helpers present pending edits](issues/16-pending-edit-ui-helpers.md#outcome):
  one shared collection binds field drafts/failures and action disabling; native 8
  and browser 16+1 tests passed, final reviews clear.

- [Parts and PCB native test preparation](issues/19-parts-pcb-native-test-preparation.md#outcome):
  remaining Parts pumps removed, queued domain regressions added; native 42/21 and
  browser 50/38 tests passed.

- [PendingEdits owns keyed settlement](issues/15-keyed-pending-edits.md#outcome):
  a Dioxus-free collection owns latest-key observations and once-only terminal results;
  reviewed API published, Runtime 110 tests passed.

- [Edit tickets own captured Scope liveness](issues/14-edit-ticket-scope-lineage.md#outcome):
  retained Scope readers and shared retirement latches cover real navigation/Open/Close;
  native Runtime 105 and browser Runtime 29 tests passed.

- [Editor composes per-workspace state](issues/12-editor-workspace-state.md#outcome):
  six private-state handles own their projections/effects/actions; the complete
  browser gate and final page integration passed.

- [Resolvers state only their intent](issues/04-resolution-constructors.md#outcome):
  Session-owned constructors replace resolver command bookkeeping, including Runtime
  integration helpers.

- 03 Native test Runtime drives real Session/Core and shared gates; fake drivers and
  manual settlement are removed: [Outcome](issues/03-native-test-runtime.md#outcome).

- ES-20 Preview-only direct events landed; former test commits use resolvers:
  [Outcome](../edit-settlement/issues/20-preview-only-direct-edit-event.md#outcome).

- 10 Export capture, currency and lifecycle use one lease registry; mechanical
  formatting has native tests: [Outcome](issues/10-export-leases.md#outcome).

- 11 Editor composes private-state canvas navigation and Layout findings handles;
  their owned behaviour has mounted interface coverage: [Outcome](issues/11-editor-state-tracer.md#outcome).

- TCE-01 Core owns wiring-mode defaults and mutation; the PCB resolver uses the typed
  intent: [Outcome](../typed-core-edits/issues/01-set-wiring-mode.md#outcome).

- 06 Outline planning is independent of its hook, Inspector and overlays; native
  planner coverage runs directly: [Outcome](issues/06-split-outline-lifecycle.md#outcome).

## Tickets

- [01 Decide the pending-edit settlement rules and module shape](issues/01-decide-pending-edit-settlement.md) (resolved)
- [02 Decide the native test Runtime's shape](issues/02-decide-native-test-runtime.md) (resolved)
- [03 Native tests run the real Session and Core](issues/03-native-test-runtime.md)
- [04 Resolvers state only their intent](issues/04-resolution-constructors.md)
- [PendingEdits and Matrix tracer integration gate](issues/05-pending-edits-module.md)
  (orchestrator gate; retains downstream dependencies)
- [Edit tickets own captured Scope liveness](issues/14-edit-ticket-scope-lineage.md)
- [PendingEdits owns keyed settlement](issues/15-keyed-pending-edits.md)
- [Shared UI helpers present pending edits](issues/16-pending-edit-ui-helpers.md)
- [Matrix one-shot actions use PendingEdits](issues/17-matrix-one-shot-pending-edits.md)
- [Matrix fields complete the PendingEdits tracer](issues/18-matrix-field-pending-edits.md)
- [06 Separate outline version planning from its Inspector and overlays](issues/06-split-outline-lifecycle.md)
- [07 Layout panels settle through `PendingEdits`](issues/07-layout-panels-onto-pending-edits.md)
- [08 Parts and PCB panels settle through `PendingEdits`](issues/08-parts-and-pcb-onto-pending-edits.md)
- [Parts and PCB native tests prepare the panel migration](issues/19-parts-pcb-native-test-preparation.md)
- [09 Case, Keymap, Keycaps and Library settle through `PendingEdits`](issues/09-case-keymap-keycaps-library-onto-pending-edits.md)
- [10 One export-lease module owns export capture and currency](issues/10-export-leases.md)
- [11 Editor state tracer: canvas navigation and layout findings](issues/11-editor-state-tracer.md)
- [12 Editor composes per-workspace state](issues/12-editor-workspace-state.md)
- [13 Cleanup and record](issues/13-cleanup.md)
- Cross-effort: [TCE-02 mechanical settings patch](../typed-core-edits/issues/02-mechanical-settings-patch.md),
  [TCE-03 decide outline version intents](../typed-core-edits/issues/03-decide-outline-version-intents.md)

## Parallel frontier after the Matrix tracer

The complete tracer opened the three panel migrations. Ticket09 and the PCB half
are accepted; Layout and Parts remain separate work. Parent08 stays claimed until
Parts is accepted. Root owns dev/tracker/canonical index and serializes Chrome leases.
Cleanup follows all reviewed, merged migrations, including both Parts/PCB halves.

## Parallel app allocation

- This chat owns integration; the existing Layout work remains separate.
- The user's second app completed
  [Case/Keymap/Keycaps/Library settlement](handoff-09-case-keymap-keycaps-library.md);
  root accepted its integration and resolved ticket09.
- The existing third-app worktree owns [Parts settlement](handoff-08-parts-pcb.md),
  which remains unaccepted.
- The separate PCB app completed [PCB settlement](handoff-08-pcb.md); root accepted
  that half without closing parent08.

The handoffs name exact branches, worktrees and starting commits. Preserve ongoing
work when resuming; each stream reports final pinned reviews and verification.

## Out of scope

- Splitting `runtime.rs` beyond exports (preview pipelines, project lifecycle); the
  next review picks these up.
- Enforcing ADR-0006; finer-grained Undo; stale direct-edit previews (backlog).
