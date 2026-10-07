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
- Tracker conventions: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
- Line numbers in tickets are at `915d305c0`, orientation only.
- Text search: `rg` (skips the stale `.claude/worktrees/` copies).

## Order

```text
ES-20 ──┬── 03 native Runtime ─────────────────┐
        ├── 06 outline split ──┬───────────────┤
        └── TCE-01 ────────────┴── 04 resolution ── 05 PendingEdits + Matrix tracer
                                                     ├── 07 Layout panels (also 06, 12)
                                                     ├── 08 Parts + PCB
                                                     └── 09 Case, Keymap, Keycaps, Library
independent 10 export leases
            11 Editor tracer ── 12 Editor workspace state
cross-effort TCE-01 ── TCE-02 mechanical patch;  TCE-01 + 06 ── TCE-03 outline intents (human)
finish      13 cleanup (07, 08, 09, 10, 12)
```

ES-20 is [edit settlement 20](../edit-settlement/issues/20-preview-only-direct-edit-event.md);
TCE-nn are [typed Core edits](../typed-core-edits/map.md) tickets. ES-20 is resolved; 03, 06 and TCE-01 are unblocked. Tickets 10 and 11 are resolved; wave-2 tickets are claimed and running, and 12 is
unblocked.

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

- ES-20 Preview-only direct events landed; former test commits use resolvers:
  [Outcome](../edit-settlement/issues/20-preview-only-direct-edit-event.md#outcome).

- 10 Export capture, currency and lifecycle use one lease registry; mechanical
  formatting has native tests: [Outcome](issues/10-export-leases.md#outcome).

- 11 Editor composes private-state canvas navigation and Layout findings handles;
  their owned behaviour has mounted interface coverage: [Outcome](issues/11-editor-state-tracer.md#outcome).

- TCE-01 Core owns wiring-mode defaults and mutation; the PCB resolver uses the typed
  intent: [Outcome](../typed-core-edits/issues/01-set-wiring-mode.md#outcome).

## Tickets

- [01 Decide the pending-edit settlement rules and module shape](issues/01-decide-pending-edit-settlement.md) (resolved)
- [02 Decide the native test Runtime's shape](issues/02-decide-native-test-runtime.md) (resolved)
- [03 Native tests run the real Session and Core](issues/03-native-test-runtime.md)
- [04 Resolvers state only their intent](issues/04-resolution-constructors.md)
- [05 `PendingEdits`, with the Matrix Inspector as tracer bullet](issues/05-pending-edits-module.md)
- [06 Separate outline version planning from its Inspector and overlays](issues/06-split-outline-lifecycle.md)
- [07 Layout panels settle through `PendingEdits`](issues/07-layout-panels-onto-pending-edits.md)
- [08 Parts and PCB panels settle through `PendingEdits`](issues/08-parts-and-pcb-onto-pending-edits.md)
- [09 Case, Keymap, Keycaps and Library settle through `PendingEdits`](issues/09-case-keymap-keycaps-library-onto-pending-edits.md)
- [10 One export-lease module owns export capture and currency](issues/10-export-leases.md)
- [11 Editor state tracer: canvas navigation and layout findings](issues/11-editor-state-tracer.md)
- [12 Editor composes per-workspace state](issues/12-editor-workspace-state.md)
- [13 Cleanup and record](issues/13-cleanup.md)
- Cross-effort: [TCE-02 mechanical settings patch](../typed-core-edits/issues/02-mechanical-settings-patch.md),
  [TCE-03 decide outline version intents](../typed-core-edits/issues/03-decide-outline-version-intents.md)

## Out of scope

- Splitting `runtime.rs` beyond exports (preview pipelines, project lifecycle); the
  next review picks these up.
- Enforcing ADR-0006; finer-grained Undo; stale direct-edit previews (backlog).
