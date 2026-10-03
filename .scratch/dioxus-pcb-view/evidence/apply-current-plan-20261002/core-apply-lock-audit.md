# Core ApplyElectrical saved-lock audit

Audit baseline: PCB mode source `57829464369f40556c57e2de7520e7109950793b`. This is a source audit and bounded ticket, not implementation clearance or a passing Apply request regression.

## Reproduced source chain

- React `app/src/useElectricalPlanning.ts:29-37` builds `ResolveElectrical` with the exact selected-board configuration's saved `mode`, `locks`, and controller ID. `applyWiring` at lines 69-74 resolves from the then-current project before submitting `ApplyElectrical` with that plan and the same document revision.
- Dioxus's reviewed `electrical_preview_request` production helper in `web/src/pcb_wiring_mode_operation.rs` reads Direct/Matrix mode, lock map, and controller ID from the accepted board configuration; its native regression verifies Direct plus a persisted lock on the new accepted revision.
- The existing Core lock resolver test `core/tests/electrical_wiring.rs::resolver_scopes_board_and_honors_controller_and_locks` supplies a saved lock and verifies the produced assignment is marked locked. Existing `apply_materializes_switch_diode_and_controller_pins_and_preserves_manual_net` exercises the materializer, but no existing test applies a lock-bearing plan through `CoreRequest::ApplyElectrical`.
- `core/src/lib.rs:271-294` first rejects a stale base revision, then reconstructs its review plan from the current Core document but hard-codes `locks: Default::default()` at line 284. It compares that lock-free plan's fingerprint with the incoming plan.
- `core/src/electrical.rs:1204-1214` hashes `plan.assignments`; the resolver sets each assignment's `locked` bit from a matching saved key/row/column lock. Therefore a lock-bearing resolved plan and same-source lock-free review plan have different assignments/fingerprints. With an available locked plan, the current request reaches the “wiring inputs changed” Error branch despite unchanged accepted inputs. This last outcome is a code-derived finding; the new direct-request Core regression has not yet been executed in this planning packet.
- Existing `core/src/electrical.rs::materialize` at lines 1218-1219 can materialize an exact already-current plan without re-resolving it. The F5.2d private UI route uses that Core-owned helper and the ordinary Session `ReplaceDocument` owner, while Core issue 09 independently repairs the direct ApplyElectrical request for its other callers.

## Bounded correction / follow-up

Issue 09 requires Core to source locks from `self.document.hardware.boards` for the exact `plan.board_id` after the revision guard, preserve the current request and wire format, retain authoritative re-resolution/fingerprint and all stale/diagnostic/materialization checks, and prove board isolation plus lock-free behavior. Its regression must be observed red on the current code and green after correction. No request schema, public API, saved-project shape, or frontend lock inference changes are authorized.

F5.2d's private route is not a claim that the direct Core request was repaired. It captures one immutable `Current` plan for the exact accepted token/revision/board, invokes the existing Core materializer synchronously, and submits one strict-revision Session ReplaceDocument edit. The accepted plan's currentness guard and Session stale revision guard are its boundaries; the code does not duplicate resolver or materialization rules.

Retain under existing RF-009 as source-to-behavior/parity evidence; RF-001 (Session/Runtime ownership) and RF-006 (electrical board scope versus physical selection) remain the applicable architecture handoffs. No new RF ID is proposed. No Core implementation, regression result, paired public browser evidence, undo/redo, or save/reopen acceptance is claimed here.
