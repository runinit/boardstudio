# F6K.3 private structured macro editor contract

This contract covers the new `keymap/macro_editor.rs` and
`keymap/macro_controller.rs` only. Root owns module exports, Editor hooks and
panel mount, shared CSS, Runtime composition and public verification. The
existing `KeymapMacro`, `MacroStep`, `MacroChange`, `KeymapChange` and
`EditKeymap` Core operation remain authoritative.

## Reference behavior

Pinned React reference `5a472a9426e6e38993361da402cd4ec730feb369` is
`app/src/ui/KeymapPanel.tsx::MacroEditor`. It renders macros in persisted
order, each as a fieldset with name, tap duration, between-actions wait,
ordered steps, Add step and Remove macro. The add-macro button appends a
`SaveMacro` with a generated stable ID, `Macro N`, 30 ms tap, 0 ms wait and one
tap of key A. Macro edits use `EditMacro`/`MacroChange`; removal uses
`RemoveMacro`. Name, duration, delay and plain keycode inputs commit on blur;
step kind commits on change. Add step appends tap/key-press A. Choosing wait
creates wait 100 ms; choosing tap/press/release creates that kind with
key-press A. A non-wait step exposes a plain keycode input, not F6K.2's general
binding behavior controls. Remove step is disabled at one remaining step;
Add step is disabled at 128.

Names are sent raw, without trimming. Keycode text is trimmed on blur. Numeric
text remains a local draft until it parses as a `u32`; empty input follows the
reference's `Number("") == 0`. Core decides supported ranges, valid names,
keycodes, nested macro references and whole-map limits. Local handling only
keeps unrepresentable numeric text correctable. No UI-side macro/DTS engine,
validation mirror, step reordering or stable step IDs are added.

Core validation is the acceptance oracle: IDs are unique identifiers; names
are nonblank valid text of at most 32 bytes; a keymap has at most 128 macros;
each macro has 1–128 steps and tap/wait/step wait values no greater than
10,000 ms; expanded bindings are at most 256 cells (wait counts four and each
other step counts two); nested macro calls are rejected. Existing firmware
generation remains the only ZMK output implementation.

## Component seam and read-only projection

`use_macro_operations` is called unconditionally for the Editor lifetime, even
when the Keymap panel is hidden. Its result includes a stable Editor lifetime
ID, one monotonic controller-owned request-sequence `Signal<u64>` shared across
component remounts, a read-only macro source, editability, the exact current
feedback, and `EventHandler<MacroEditRequest>`. Root passes those values to
`MacroEditor` from the already accepted document and full current Scope.

The read-only macro source holds only a cloned `Arc<ProjectDoc>` handle and
exposes the existing macro slice; it does not clone the map, macro list, steps,
or document and cannot mutate accepted state. Its equality uses source Arc
identity and accepted token/revision so child props do not deep-compare the
whole project. Draft children receive only their accepted scalar/step value,
target and callback. Missing keymap renders an empty macro list; `Some` with an
empty layer list remains non-editable because Core rejects that map.

`MacroEditRequest` carries full `Scope`, the captured scope-generation stamp,
accepted `SnapshotToken` and revision, Editor lifetime ID, request ID, typed
target identity, and one supported operation. Targets are collection/add,
collection/remove, macro name/tap/wait, or macro step index plus
kind/delay/keycode field. Operations are Add, Remove by stable macro ID, or
one typed `MacroChange`. The component has no Runtime/Core access and never
submits a whole `ProjectDoc` or `KeymapConfiguration`. The controller
validates that target, operation variant, field and index agree before
constructing the Core change.

Draft keys include full Scope, Editor lifetime, macro ID, field identity and
accepted value. Step draft identity includes macro ID, index, accepted step
value, and an `Rc`-shared stamp of the exact accepted ordered step sequence.
The sequence stamp changes on any structural or step-content edit, including
removing one of two identical adjacent steps, while excluding unrelated token,
macro-name and duration changes. Requests carry that shared bounded stamp;
controller admission requires it still matches the accepted sequence before
using an index. This prevents dirty text and delayed acknowledgements from
being attached to a different row when indices shift. It does not invent step
IDs or clone the sequence per field. Token/revision are excluded from draft
keys so unrelated saves do not erase dirty invalid text. A single in-flight
macro operation prevents local add/remove/edit overlap. Pending/result feedback
is accepted only for the exact request ID and target that the root controller
currently admits. Component-local request allocation cannot hide a prior
admitted result when a later callback is ignored.

## Controller admission and acknowledgement

The Editor-lifetime controller owns one pending request and its existing
Runtime operation outcome slot. Before admission and immediately before
`Runtime::submit`, it re-reads and requires: Keymap workspace, live full Scope,
captured scope generation, the root instance-selection guard, exact accepted
token/revision, `Ready` and `Saved` at that revision, no display preview, no
gesture, an editable current keymap, no other macro operation, exact Editor
lifetime/request sequence, and a current unique macro ID/step index. All
`RefCell` borrows end before `Runtime::submit`.

Add allocates a stable entity ID separately from the Runtime operation ID,
checks collision against the current map, derives `Macro N` from the current
macro count, and submits the reference defaults. It rejects at 128 macros.
Remove requires exactly one matching stable ID. Edit requires the macro ID and
field/step target to exist in the freshly accepted macro; Add/Remove step enforce
the 128/last-step UI limits before submitting. Step changes are reconstructed
from the current accepted step: delay changes only `Wait.ms`; keycode changes
replace the binding with `KeyPress` while preserving the current Tap/Press/
Release kind; choosing a kind uses the React defaults (Wait 100 ms or that
kind with KeyPress A). This prevents stale component values from replacing
other fields. Core remains responsible for domain validation. Each accepted
UI request registers `observe_operation` before one `Event::Edit` containing
the existing `EditKeymap` change and a fresh transaction ID.

Completed outcomes remain pending through Busy/Unsaved and settle only after
the exact source is Ready/Saved at a later accepted token/revision. The
controller confirms the stable macro ID and requested value before showing
Saved; step acknowledgements also confirm the same expected ordered step
sequence so an index shift cannot settle against another row. For this bounded
acknowledgement it may retain the one target macro's at-most-128 steps, never
the map or document. A terminal error stays attached to its macro/field only
while the admission field value remains current; step-field relevance also
requires the original step sequence. A synthetic mismatch is tied to its exact
acknowledgement token and clears when the token advances. Scope/generation
changes drop stale local feedback without cancellation or replay into another
document. Persistence failure preserves an editable draft and its exact Core
message.

The controller does not require CAD readiness and does not generate firmware.
Workspace hiding keeps the Editor hook and pending observer alive. The root
caller must preserve that lifetime and supply fresh accepted data and
instance-selection admission.

## Ownership, evidence and open gates

`CONSTRAINTS.md` and ADR-0003 keep accepted documents, edit/history semantics,
validation and generated firmware owned by existing Rust services. No
`CONTEXT-MAP.md` exists in this worktree; the relevant domain references are
`docs/keycaps-and-keymap.md`, `docs/design/keymap-workspace.md`,
`docs/design/keymap-verification.md`, and `docs/architecture.md`.

The historical boundary/error log
`docs/design/evidence/keymap/macro-boundary-red.log` records an earlier Core
test failure. A current parent-owned rerun passed the exact 256-cell boundary
test: `cargo test --locked --manifest-path core/Cargo.toml --test
keymap_workflow macro_expansion_respects_zmk_binding_limit_at_the_boundary`
(1 passed, 9 filtered; 64 waits accepted and 65 rejected). Its raw log is
`evidence/macro-editor/current-boundary-native.log`. The tested
`core/src/keymap.rs` SHA-256 was
`a2a0764e90efeacf74482f7a399350914e9f049511af6b4e0478ec1e9ba5538b`. Keep
the original red artifact unchanged; the current run clears only this focused
Core boundary gate, not the macro UI acceptance joins.

Source implementation, Astra review, native/WASM checks and public browser
evidence remain open. Required public evidence includes every macro and step
operation, validation recovery, stable macro binding through rename, stale
scope/feedback, Undo/Redo, save/reopen, workspace switch and actual firmware
provider output. INT.2 remains a parent acceptance join. RF-009 remains the
applicable source/parity accounting item; this source pass found no additional
refactoring takeaway.
