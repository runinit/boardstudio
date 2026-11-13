# Ticket 019: editor-local draft safety change map

Researched 2026-11-07 against the **dirty** automatic worktree overlay at HEAD
`366d52100d9dbf6686a9cd9dfdf02f0262a13eed`. Native Gortex served the exact checkout
`01a1b3cf-a8df-75da-ab9e-d3fe3b2d3378`, commit generation 2. Source collection began
at dirty generation 51; note creation/reindexing advanced the overlay afterward.
Source reads were disk-verified. No clean-checkout
substitution, shell source fallback, product edit, test run or ticket closure occurred.
This is a bounded source investigation and **proposed** change map, not authorization
to implement it or proof of observed browser data loss.

## Bottom line

**The draft-detection gap is real in source, not just missing test evidence.**
The Layout constraint form and Geometry scripts editor hold valid values locally
while awaiting explicit submission. Moving focus does not submit them, and they need not be
invalid. The update gate observes neither owner's dirty state. With all other owners
settled and no invalid marker, the gate permits adoption/reload although those values
have no save path until Save/Add/Apply. The later reload check uses the same inputs,
so it does not repair that omission. See the editor inventory and execution map below.
An actual A-to-B loss with these editors has **not** been reproduced in this task.

[019](../plans/ui-integration-recovery/issues/019-adopt-build-safely.md#acceptance-criteria)
and [013's user decisions](../plans/ui-integration-recovery/issues/013-surface-build-updates.md#later-answers-user-2026-11-06)
already require deferral for unresolved drafts, not a discard acknowledgment.
[ADR-0022](../adr/0022-static-local-first-rust-application-with-worker-execution.md#amendment-opt-in-adoption-of-a-waiting-build-2026-11-07)
records the gap; its general blur/invalid-field explanation is not true for every
editor below. [The plan map](../plans/ui-integration-recovery/map.md#next-step-and-boundaries)
keeps 019 separate from 010/011. Retained ticket logs remain historical, not reruns.
Use [Draft value, Pending edit and Edit preview](../../GLOSSARY.md#L411-L439)
as distinct states: only a submitted edit is in the engine/save pipeline.

## Editor evidence

Symbol IDs below are exact Gortex IDs; paths are repository-relative. A source
counterexample means the editor's local state is not represented by the named
DOM selector, not that a browser reload loss was observed.

| Owner and source | Actual behavior | Implication / proof limit |
| --- | --- | --- |
| [Layout component Inspector](../../web/src/presentation/inspector/layout_component_inspector.rs), lines 662–735, 1366–1416, 2410–2420, 2494–2502; ID `ts-boardstudio2/web/src/presentation/inspector/layout_component_inspector.rs::LayoutComponentInspector` | `mirror_coordinate` is a local signal initialized from the active constraint; numeric input changes only the signal; constraint submission is via Add/Save constraint. Parsing happens on save. No blur handler or `aria-invalid` is supplied on that input. | **Confirmed detection defect:** a changed valid coordinate remains local after blur until explicit Save/Add, and is not selected by the invalid-field query. Also an unparseable value has no field-level invalid marker here. Production mount is source-confirmed in `layout_workspace::inspector` at lines 942–963 (see reachability below); trusted Update Now interaction and actual reload loss remain unrun. |
| [Legacy position Inspector](../../web/src/presentation/inspector.rs), lines 50–81, 145–267, 293–355; IDs `ts-boardstudio2/web/src/presentation/inspector.rs::Inspector` and `ts-boardstudio2/web/src/presentation/inspector.rs::commit_numeric` | X/Y input updates local pair and dirty axes; valid finite pair sends a position preview. Enter or Apply calls commit; Escape cancels/restores. No blur commit appears in the controls; invalid parse returns early and commit reports an error without assigning `aria-invalid` on those inputs. | Source-level preview/draft risk; production fallback appears in `layout_workspace::inspector` at lines 965–966. An active preview request can temporarily block via Session; after it settles, whether the remaining local value is protected needs a real mounted trace. Do not count a preview as a durable edit or claim this loss was observed. |
| [Keymap macro number](../../web/crates/keymap/src/keymap/macro_editor.rs), lines 583–623; ID `ts-boardstudio2/web/crates/keymap/src/keymap/macro_editor.rs::NumberDraft` | Input clears failure, sets local string and dirty. Blur parses `u32` (empty becomes zero), commits only when parsing succeeds and dirty; `aria_invalid` reflects failure, not parse validity. | An unparseable number can remain dirty after blur without this component setting `aria-invalid`; native number input sanitization and realistic input values need browser verification. Graph callers `::StepEditor`, `::MacroCard`, `::MacroEditor` are inferred edges, not proof of a particular mounted route. |
| [Board name Inspector](../../web/crates/layout/src/board_inspector.rs), lines 90–153, 180–251, 371–473; IDs `ts-boardstudio2/web/crates/layout/src/board_inspector.rs::use_board_inspector`, `::BoardInspector`, `::commit_draft` (same path prefix) | Input writes draft; blur or Enter calls commit; Escape restores accepted name. Empty trimmed name restores accepted rather than submitting; valid changed name calls `on_rename`, whose owner checks precede `begin_field`. | A contrasting blur-committing owner; focus movement may dispatch work before the update check. Event ordering and settlement require a mounted run rather than assuming every text field persists immediately. |

Additional owners independently inspected by the coordinator:

| Owner and source | Actual behavior | Classification |
| --- | --- | --- |
| [Geometry scripts](../../web/crates/ui-shared/src/geometry_scripts.rs#L105-L438), ID `ts-boardstudio2/web/crates/ui-shared/src/geometry_scripts.rs::GeometryScriptsEditor` | Local name/source/enabled signals and serialized draft are bound to `PendingEditSignals` (105–139). Name/source input and checkbox changes update local state (360–390). Only **Apply script** calls `begin_field` with `apply_script_resolver` (392–428). No blur commit or invalid ARIA on those controls. Production mount: `layout_workspace.rs:925–932`. | **Second confirmed detection defect**, without relying on native numeric-input behavior. An already-saved script can have unsaved valid source text while all engine/save owners remain settled. Binding a field to the pending helper does not protect it before submission. Actual reload loss unrun. |
| [Layout scalar control](../../web/crates/layout/src/objects/matrix_transform_inspector.rs#L991-L1238), ID `ts-boardstudio2/web/crates/layout/src/objects/matrix_transform_inspector.rs::NumericTransformField` | Input sets dirty and clears error; blur/Enter parse a finite `f64`, call `on_edit` with the draft/baseline, or retain draft and set error. Escape restores accepted text. Error is projected as `aria_invalid`. | Concrete blur/validation contrast, not universal proof. Dirty state can precede error state; safety still depends on focus movement and timely admission of the request. |
| [Keymap binding configuration](../../web/crates/keymap/src/keymap/binding_editor.rs#L715-L780), ID `ts-boardstudio2/web/crates/keymap/src/keymap/binding_editor.rs::KeycodeField` | Input/combobox selection changes local draft. Container focusout calls `with_keycode`, then `emit_change` if dirty and conversion succeeds. `aria_invalid` reflects `!dirty && props.error`, not an all-drafts validity query. | Focusout/portal/conversion behavior needs a mounted trace; not classified as proven loss. |
| [Project-name form](../../web/crates/library/src/library.rs#L854-L903) and [rename disclosure](../../web/crates/library/src/library.rs#L1095-L1136), owner `Library` | Input sets dirty; Enter requests native blur; blur restores accepted text for empty trimmed input or calls rename action for a nonempty name; Escape restores accepted text. | Another real blur-commit path. Empty-name restoration is explicit existing behavior, not evidence of an update defect. A rename/control unmount race is unproven. |
| [Library delete dialogs](../../web/crates/library/src/library.rs#L1195-L1310), owner `Library` | Confirm/cancel/error UI, no typed draft in the inspected dialogs. Deletion has separate asynchronous ownership. | These dialogs are not draft counterexamples. Neither this inspection nor the gate establishes coverage of every dialog, file import, settings panel, hidden or portal-mounted editor. Do not equate “modal open” with dirty draft. |

### Production reachability and inventory boundary

`presentation.rs:8328–8355` constructs the Layout inspector input;
[`workspace_composition::inspector`](../../web/src/presentation/workspace_composition.rs#L107-L110)
routes it to [`layout_workspace::inspector`](../../web/src/presentation/layout_workspace.rs#L829-L966).
The latter renders Geometry scripts when open, otherwise the selected component
inspector or legacy position fallback. Gortex `relations(callers)` returned **no
incoming edge** for `LayoutComponentInspector`; native Gortex text search and these
source mounts resolve that graph omission. The exact IDs are
`ts-boardstudio2/web/src/presentation/workspace_composition.rs::inspector` and
`ts-boardstudio2/web/src/presentation/layout_workspace.rs::inspector`.
Source reachability is established; a particular visible form plus Toast/header
interaction is still a browser proof obligation.

This is a representative inventory sufficient to disprove the blanket assumption,
not a census of every draft in PCB, Keycaps, Case, Parts, setup and settings.
The identified owners are the first regression fixtures; remaining owner coverage
must be explicit before claiming all-draft safety.

## Existing shared seams

- [`PendingEditSignals`](../../web/crates/ui-shared/src/pending_edit_helpers.rs)
  lines 1–64, 117–142 and 168–201; ID
  `ts-boardstudio2/web/crates/ui-shared/src/pending_edit_helpers.rs::PendingEditSignals`:
  per-panel shared bindings, submitted-value memory, binding epochs, settlement
  and one-shot disabling. It protects newer drafts from old outcomes, but binding
  a signal is not itself a global dirty-draft inventory. It does not cover all
  local form signals illustrated above.
- [`InputDrafts` / `apply_drafts`](../../web/crates/pcb/src/pcb_wiring/part_input_settings/owner.rs)
  lines 129–173; ID
  `ts-boardstudio2/web/crates/pcb/src/pcb_wiring/part_input_settings/owner.rs::apply_drafts`:
  context-backed reapplication of **committed pending intents** to a part
  projection, not proof of a global unsubmitted-value registry.
- [`visible_invalid_draft`](../../web/crates/runtime/src/runtime.rs) lines 281–293;
  ID `ts-boardstudio2/web/crates/runtime/src/runtime.rs::visible_invalid_draft`:
  queries particular `aria-invalid=true` controls; it does not compare local
  values to accepted state. The selector has no visibility test despite the
  function name, and disabled exclusions vary by selector branch.
- [`OwnedEdits`](../../web/crates/ui-shared/src/owned_edits.rs#L1-L109), ID
  `ts-boardstudio2/web/crates/ui-shared/src/owned_edits.rs::OwnedEdits`: follows scope
  and generation, rebinds fields, exposes bound draft text and submitted pending keys.
  It supplies owner-lifetime patterns, not a page-wide accepted-versus-draft census.
- [`InputGroupInput` / `InputGroupTextarea`](../../web/crates/ui-shared/src/ui/input_group.rs#L154-L310)
  forward owner-supplied blur/keyboard/ARIA; they do not synthesize a commit handler,
  dirty state or invalid marker. Styling an invalid field is not validating it.
  Exact IDs: `ts-boardstudio2/web/crates/ui-shared/src/ui/input_group.rs::InputGroupInput`
  and `ts-boardstudio2/web/crates/ui-shared/src/ui/input_group.rs::InputGroupTextarea`.
  Earlier no-matches for `ScalarEditor`/`ScalarInput` were naming misses;
  `NumericTransformField` above is a real scalar implementation.

## Execution and observation map

These are source-confirmed stages, not one observed browser trace. In the IDs below,
`ts-boardstudio2/` is the indexed repository prefix. Graph-derived relationships
were cross-checked against source because trait/callback boundaries are incomplete.

| Stage | Source and exact symbol IDs | Ordering / what is observable |
| --- | --- | --- |
| Local value → explicit submission | Layout `LayoutComponentInspector` above, `save_constraint` closure at 1366–1478; Geometry `GeometryScriptsEditor` above at 392–428 | Before Save/Add/Apply there is **no document submission** for these values. Layout parsing can reject before calling `on_action`; successful parsing calls `SetConstraint`. Geometry submits a resolver using the captured name/source/enabled tuple. |
| Submission → Session admission | [`submit_layout_component_edit`](../../web/src/presentation.rs#L2307-L2344), ID `ts-boardstudio2/web/src/presentation.rs::submit_layout_component_edit`; [`EditTicketPort` Runtime adapter](../../web/crates/runtime/src/edit_ticket.rs#L203-L228); [`Runtime::submit`](../../web/crates/runtime/src/runtime.rs#L2001-L2031), ID `ts-boardstudio2/web/crates/runtime/src/runtime.rs::Runtime.submit` | The pending helper starts a field/one-shot edit; the adapter calls Runtime submit. Runtime synchronously calls `Session::submit` before asynchronously driving returned effects. This protects an already-admitted edit, not every local string or delayed callback. |
| Core work → save requested | [`Session::has_pending_work`](../../application/src/session.rs#L683-L694), ID `ts-boardstudio2/application/src/session.rs::Session.has_pending_work`; [`Session::core_completed`](../../application/src/session.rs#L1838-L1965), ID `ts-boardstudio2/application/src/session.rs::Session.core_completed` | Queue/active Core request blocks as pending work. Core Scene/document success sets `pending_save` and `Durability::Saving` **before** emitting `Effect::Persist`. The new document is not yet the accepted snapshot. A completed preview alone does not save it. |
| Persist → storage acknowledgment | [`Runtime::run`](../../web/crates/runtime/src/runtime.rs#L2217-L2264), ID `ts-boardstudio2/web/crates/runtime/src/runtime.rs::Runtime.run`; [`BrowserStore::save_document_expected`](../../web/crates/host/src/host/storage.rs#L158-L206), ID `ts-boardstudio2/web/crates/host/src/host/storage.rs::BrowserStore.save_document_expected`; [`persist_project`](../../web/crates/host/src/host/storage.rs#L558-L677), ID `ts-boardstudio2/web/crates/host/src/host/storage.rs::persist_project`; [`transaction_completion`](../../web/crates/host/src/host/storage.rs#L959-L1004), ID `ts-boardstudio2/web/crates/host/src/host/storage.rs::transaction_completion` | Runtime sets retained `pending_document` before awaiting persistence. Storage waits for IndexedDB transaction complete/abort, not just a put-request success. Success clears retained document/conflict then delivers `Completion::Persist`; failure retains the document and reports aborted/conflict. This is the application's storage acknowledgment, not a claim of crash-proof physical durability. |
| Acknowledgment → accepted and saved | [`Session::persist_completed`](../../application/src/session.rs#L1966-L2067), ID `ts-boardstudio2/application/src/session.rs::Session.persist_completed` | Matching save-attempt identity is required. Committed installs the accepted snapshot and `Durability::Saved`; failure retains pending save and sets Failed/RecoveryRequired. Queued work can resume afterward. `has_pending_work` intentionally excludes a lone pending save; the separate save inputs cover it. |
| Toast/header → initial safety check | [`NotificationProvider`](../../web/src/presentation/notifications.rs#L20-L33), ID `ts-boardstudio2/web/src/presentation/notifications.rs::NotificationProvider`; [`WorkbenchApp`](../../web/src/presentation.rs#L695-L724), ID `ts-boardstudio2/web/src/presentation.rs::WorkbenchApp`; [`Runtime::begin_update`](../../web/crates/runtime/src/runtime.rs#L952-L981), ID `ts-boardstudio2/web/crates/runtime/src/runtime.rs::Runtime.begin_update` | Both controls call the same gate. Non-idle ignores duplicate activation. Current waiting state is checked, then safety. Blocked shows a cancel-only dialog; safe starts the asynchronous adoption flow. There is no explicit draft flush, blur call or wait-for-editor-commit handshake here. |
| Gate's full view | [`Runtime::update_safety_inputs`](../../web/crates/runtime/src/runtime.rs#L930-L946), ID `ts-boardstudio2/web/crates/runtime/src/runtime.rs::Runtime.update_safety_inputs`; [`visible_invalid_draft`](../../web/crates/runtime/src/runtime.rs#L281-L293), ID `ts-boardstudio2/web/crates/runtime/src/runtime.rs::visible_invalid_draft`; [`update_safety`](../../web/crates/runtime/src/update_safety.rs#L139-L174), ID `ts-boardstudio2/web/crates/runtime/src/update_safety.rs::update_safety` | Lifecycle/recovery, storage initialization/error, Session pending work, durability versus accepted revision, retained/started save, save conflict and a DOM `aria-invalid` selector. **No general dirty-draft input.** Selector does not test visibility or exclude the warning; role branches do not exclude disabled controls. DOM/query failure collapses to false. These are limits of the fallback, not proof of each possible failure. |
| Worker request → takeover | [`Runtime::run_update_adoption`](../../web/crates/runtime/src/runtime.rs#L982-L1016), ID `ts-boardstudio2/web/crates/runtime/src/runtime.rs::Runtime.run_update_adoption`; [`adopt_waiting_registration`](../../web/crates/host/src/host/offline.rs#L204-L308), ID `ts-boardstudio2/web/crates/host/src/host/offline.rs::adopt_waiting_registration`; [`start_offline_worker`](../../web/src/service_worker.rs#L142-L236), ID `ts-boardstudio2/web/src/service_worker.rs::start_offline_worker` | Page attaches `controllerchange` listener before posting the opt-in message. Worker checks its live window clients before skip-waiting, replies accepted/declined; page bounds reply at 5s and accepted takeover at 10s. The worker does not inspect editor drafts. No new draft check occurs between initial gate and posting; work may start during this await. |
| Takeover → final check → reload | [`Runtime::reload_when_safe`](../../web/crates/runtime/src/runtime.rs#L1020-L1040), ID `ts-boardstudio2/web/crates/runtime/src/runtime.rs::Runtime.reload_when_safe`; [`UpdateSignals`/registration reload](../../web/crates/host/src/host/offline.rs#L179-L197) | After accepted takeover and controller change, the **same** safety inputs are re-read. If blocked, report installed and poll every 250ms; if safe, call reload with no intervening await in this function. Production calls `window.location().reload()`. Thus admitted work begun during takeover is guarded, but an unreported local draft is still invisible. Post-takeover deferral automatically resumes; initial deferral requires another Update Now. |

### Event order and remaining race proof

- **Do not assume every Update Now activation blurs the field.** The
  [HTML focus model](https://html.spec.whatwg.org/multipage/interaction.html#focusable-area)
  makes click-focusability browser/platform dependent and explicitly says synthetic
  `.click()` does not focus. Trusted pointer focus transfer or Tab navigation may
  invoke a blur/focusout commit; an `HtmlElement.click()`-only test does not prove it.
- For a blur-committing field, the required timeline is input → actual focus
  transfer → blur/focusout handler → synchronous admission (or an explicit
  outstanding-commit signal) → gate. Save completion need not precede activation:
  pending work/Saving must cause deferral. Record actual pointer/keyboard event and
  admission order instead of asserting a universal browser sequence.
- [Dioxus 0.7.10 event docs](https://github.com/dioxuslabs/dioxus/blob/v0.7.10/packages/html/docs/event_handlers.md)
  (Context7 `/dioxuslabs/dioxus/v0.7.10`; pinned by `web/Cargo.toml:58–62`) say returned
  async handlers are spawned. A blur event is therefore not a general await barrier.
  A future editor callback that awaits **before admission** needs a safety signal
  covering that interval. No such specific runtime race was reproduced here.
- Explicit-Apply owners remain dirty even **if blur runs perfectly**. Likewise,
  error ARIA can be set only after validation/render, and an unfocused or hidden
  retained draft need not participate in the current focus transfer at all.
- Edits can begin while adoption is awaited, including in another control or a
  dialog. The final gate correctly rechecks known owners but shares the draft
  omission. Fixing only `begin_update`, only the focused input, or only invalid
  styling cannot establish safety. Newer text typed while an earlier submission
  settles must keep its own protection.

### Graph evidence and limits

Native `explore(task)` localized the gate; `relations(callers)` connected both
notification and `WorkbenchApp` to `begin_update` (heuristic callback edges).
`trace(call_chain, begin_update, depth=2)` returned AST-resolved calls to
`update_safety_inputs`, `run_update_adoption`, and from that to `reload_when_safe`;
source establishes the awaits above. It reported dynamic dispatch boundaries for
`UpdateSignals::adopt_waiting` and spawned work, not a complete cross-worker trace.
Unrelated name matches such as `EventLog.borrow`/`FakeUpdateSignals.set` were rejected
as execution evidence. `relations(callers, Session.persist_completed)` includes
`Session.complete`; direct source at `session.rs:1271–1283` confirms the Persist arm.

`change(impact)` on `update_safety_inputs`, `UpdateSafetyInputs`, `PendingEditSignals`
and `GeometryScriptsEditor` reported **13 dependents as a lower bound**, including
both update checkpoints and `OwnedEdits`. Its CRITICAL label reflects bounded reach
and dispatch uncertainty, not 13 established bugs. Contract analysis was rebuilding
and not certified current. `change(tests)` reported 0/4 symbol coverage for the two
checkpoints and two real editors despite tests present in source below: graph test
coverage is incomplete, not evidence that those tests do not exist or pass.

## Smallest coherent change map (proposal only)

Prefer **owner-reported unresolved-draft safety**, reusing existing binding/owner
lifecycles where they already exist and adding a narrow adapter for composite forms.
Do not first change blur behavior, invent a second document model, or assume a global
registry is already present. The observable contract should cover a live value that
would be lost on reload, whether valid, invalid or awaiting admission. Exact API and
storage shape remain design work; a scoped registry is one implementation option.

| Slice | Concrete change surface | Required behavior / boundaries |
| --- | --- | --- |
| 1. Add a failing real-owner regression | `web/src/presentation/notifications.rs` production-shell fixtures, `layout_component_inspector_tests.rs`, `ui-shared/src/geometry_scripts.rs` mounted tests | Start settled, change a real explicit-submit value, leave focus, then activate Toast and header. Assert no adoption/reload, exact draft preserved on Cancel. No synthetic invalid-field substitute. Establish RED before product fix. |
| 2. Expose the missing owner state | `Runtime`/`update_safety.rs` (`UpdateSafetyInputs`, `UpdateBlocker`, both update checkpoints); existing `PendingEditSignals`/`OwnedEdits` adapters as applicable | Aggregate **all live reported owners**, not active element only. Keep Core/save/recovery signals unchanged. A valid unsaved form needs truthful “unsubmitted changes” wording, not `InvalidDraft`. Both initial and post-takeover checks consume the same state. |
| 3. Wire proven counterexamples first | `LayoutComponentInspector` constraint tuple; `GeometryScriptsEditor` name/source/enabled tuple | Report differing user draft synchronously on input/change; only explicit existing Save/Add/Apply commits. Merely opening an unchanged form must not block. Preserve dirty state across unrelated accepted revisions; a successful older submission must not clear newer text. Include parse failure, owner identity, explicit reset/cancel and rebinding. |
| 4. Close remaining owner coverage | `NumericTransformField`, board/project-name owners, keymap `NumberDraft`/`KeycodeField`, legacy position fallback; PCB/Keycaps/Case/Parts/setup/settings inventory | Use existing field and scope boundaries; handle malformed native numeric values and pre-admission async work. Register composite/unbound forms explicitly. Retained hidden drafts still count; release handles on genuine owner retirement, not just focus loss. Do not classify search/filter text or reproducible previews as document edits without a product reason. |
| 5. Explain deferral without offering discard | `notifications.rs::blocker_message` / `UpdateWarningDialog`, `Runtime::reload_when_safe` installed message | Say where to Apply/Save, fix or explicitly cancel a draft. Initial warning remains cancel-only and keeps values/focus. Post-takeover text must not imply that the save label or invalid fields exhaust all risks. No force-submit-all or discard-and-reload action. |
| 6. Add end-to-end proof; correct overbroad documentation | `scripts/verify-update-adoption.py` + payloads; 019 evidence, 013 reconciliation; `update_safety.rs` module explanation and dated ADR-0022 amendment only after implementation | Real A→B, actual edited document reopened, real held persistence, both entry points, takeover-time new draft. Preserve ordinary delayed activation/cache policy. Do not close 019/013 on this note or a green unit suite. |

**Tradeoffs:** owner reporting covers valid composite drafts without changing how
Apply works; it adds lifetime/completeness obligations and needs accepted-value
comparison appropriate to each editor. `PendingEditSignals` supplies useful epochs,
unbinding and newer-value protection, but lacks a global inventory/accepted baseline
at bind time. Extending it alone misses unbound Layout constraint signals; Geometry
is a useful test that an existing binding also needs reporting. `OwnedEdits` can
supply scope/generation, not global completeness. A small separate scoped aggregator
may keep save/session policy independent of Dioxus, but a broad all-panel registry
refactor is not justified until the remaining owners and requirements are inventoried.

Marking only invalid inputs is a useful validation/accessibility repair for specific
fields, **not** a fix for valid unsubmitted values. Blanket `blur()`, forced Apply,
or a timeout before reload changes editor semantics or merely moves the race.
Conservatively blocking while every explicit-submit editor is open is a possible
short-term safety policy, but falsely blocks unchanged forms; it requires explicit
product agreement and cannot be presented as the finished dirty-state solution.

### Product decisions still needed

No new decision is needed about discarding unresolved drafts: the user already said
**defer**. Nor should this task re-decide explicit Apply semantics. If a temporary
open-form blockade is chosen, its false-positive UX needs approval. Before claiming
complete coverage, classify which dialog/setup/settings values are user work rather
than disposable navigation/preferences. Retaining the current automatic resume after
takeover needs no new policy; changing it to require a fresh Update Now would need
approval. New draft-deferral copy must describe that existing behavior accurately.
No unresolved product decision blocks reproducing the two proven detection gaps.
These are bounded choices, not permission to implement or reasons to weaken the gate.

## Existing proof versus verification still required

Inspected source, **not rerun**:

- [`session_pending_work.rs:123–278`](../../application/tests/session_pending_work.rs#L123-L278)
  tests queue/active Core and unanswered persistence separately. The lone save is
  not `has_pending_work`, but is `Durability::Saving`.
- [`notifications.rs:962–1020`](../../web/src/presentation/notifications.rs#L962-L1020)
  holds a Core reply after submitting a rename, then releases it and checks one
  adoption/reload after save. It is not a real local field-input regression.
- [`notifications.rs:1028–1066`](../../web/src/presentation/notifications.rs#L1028-L1066)
  appends an input with `aria-invalid`; it tests the selector, not a real invalid editor.
- [`notifications.rs:1127–1162`](../../web/src/presentation/notifications.rs#L1127-L1162)
  gates persistence; `1165–1200` injects save failure; `1247–1282` holds a Core edit
  during scripted takeover. Useful runtime guards, not proof of packaged held-save
  or explicit-submit draft safety. Test support at `runtime.rs:8469–8497` distinguishes
  production browser-store saves from the optional memory port.
- [`leg_draft_safety:997–1047`](../../scripts/verify-update-adoption.py#L997-L1047), ID
  `ts-boardstudio2/scripts/verify-update-adoption.py::leg_draft_safety`, fills a real
  Board name **plus** an injected invalid field. It later checks Saved locally and
  the renamed board restored after actual adoption. This establishes a useful
  reopened-value assertion, but the injected marker may be the only blocker; there
  is no held real pending save in this leg. The header label alone is not draft proof.

Implementation verification plan (not run/authorized here):

1. **Real valid draft RED/GREEN:** existing saved mirror constraint and saved
   geometry script; edit without Save/Apply, transfer focus, activate both entry
   points by trusted pointer and Tab/Enter/Space. RED must fail specifically because
   adoption/reload proceeds or no draft blocker appears; GREEN must defer and Cancel
   preserve exact text/form/owner. Do not “pass” on an unrelated invalid field.
2. **Real invalid owner:** malformed constraint and macro/scalar numeric input that
   the browser can actually retain. Observe DOM value, local validation and draft
   state; do not inject impossible `type=number` text or manually set ARIA. Fix or
   explicitly reset; verify the blocker clears only for the resolved value.
3. **Admission/save boundary:** trace input, blur/focusout, callback, Session
   admission, Core reply, persist entered, transaction acknowledgment and accepted
   revision. Hold the real persistence boundary, activate, Cancel, then release.
   Assert no adopt/reload while held and the intended value/document/revision after
   B reload. Reuse existing persist gates for mounted coverage; a packaged hold
   needs a deterministic test-only seam at that boundary, not sleep or a fake label.
4. **Takeover race:** begin safe, hold takeover, type a new unsubmitted script or
   constraint value, finish takeover. Final check must keep A's page and draft alive
   even though B controls it; resolve explicitly, save and verify the chosen resume
   behavior plus actual loaded B and reopened values. Include invalid/failure case.
5. **Lifetime/completeness:** two owners, hidden retained panel/dialog, old save
   settling after newer typing, same-key unmount/remount, project/scope change,
   revert-to-accepted, reset/cancel and failed submission. No stale or stuck blocker,
   no false clearing of a newer draft, no block for unchanged or purely navigational
   state. Preserve existing stale-waiting, duplicate-action and cancellation-focus tests.
6. **Negative controls:** disable new owner reporting, omit final-check aggregation,
   or clear on old submission/blur; use `scripts/mutation-check.py` under the repo's
   workflow and require each intended real-editor assertion to fail. Later full gate
   and combined spec+standards review remain required for product implementation.

Worker-restart/ADOPTER lifetime and navigating-client cache races remain separate
unproven worker/cache questions, not explanations for the local-draft omission.
The title/display-text Toast matching concern also remains separate. This source
investigation does not resolve them, change ADR policy, or promote old evidence to
fresh proof.

## Investigation validation and transfer

One fresh-context research delegate collected initial editor evidence; the coordinator
independently checked the primary constraint counterexample, added Geometry scripts,
scalar/configuration/dialog samples, and traced Runtime/Session/storage/worker source.
Native Gortex `explore`, `read`, `search`, `relations`, `trace`, `change(impact)` and
`change(tests)` were used. No product source, existing ticket/map/ADR, baseline, build
output or historical log was changed by this investigation. The only owned deliverable
is this new Markdown file; this is an ownership statement about our tool writes,
not an assertion that the already-dirty checkout has no other changes.

Pre-create impact initially reported file-not-indexed; once indexed, artifact impact
reported LOW/0 dependents. Earlier delegate tests/guards/contract calls on an unrelated
Session symbol are **not artifact validation**. Source tests and old GREEN counts are
not a new pass. The coordinator read back all 296 lines of the synthesized note in
bounded native Gortex windows, then made these wording/validation corrections.
Post-edit `change(detect, scope=all)` enumerates the existing 23 tracked modified
files but omits this new untracked note: it is not a complete worktree-status proof.
Checks on the note's own indexed heading report no executable test targets (0/1),
no configured guard rules, and a LOW/0-dependents contract with `allow` and a
no-new-parse-errors stop condition. They do not certify citations, completeness or
product safety. File-target forms of detect/tests/guards/contract were rejected;
checks were corrected to the supported scope/heading-symbol forms. No executable
validation command, build, mutation run or browser test was run.

**Stopping point:** findings/change map only. Next authorized phase should first
produce the real explicit-submit regression, then agree the narrow owner-state seam
and remaining policy choices. Ticket 019 stays open; no commit, push or merge.

