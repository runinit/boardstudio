# 008: Restore real drag/save/Undo proof

Status: resolved
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Audit A5](../../../research/2026-11-06-dev-ui-integration-audit.md#a5--what-the-existing-green-results-actually-cover), Migration gate record

## What to build

Make the existing packaged-app drag harness exercise current controls and prove
real gesture preview/commit, durable save, cancellation and exact Undo. Restore
its existing behavioral coverage before adding the separate reload scenarios
owned by 009.

## Acceptance criteria

- [x] Reproduce and diagnose the actual setup/focus and obsolete save-status
  observation failures against an identified current packaged build. Distinguish
  harness assumptions from product defects; do not guess an application race.
- [x] The harness creates/selects the intended project and uses the current
  accessible controls through the real workflow. Required preconditions fail
  explicitly rather than accepting absent controls or skipping assertions.
- [x] Existing Part, Column, Stagger, Ctrl+Offset and Matrix drag scenarios reach
  their trusted gestures, visible previews and intended accepted changes. Keep
  pointer/capture and changed-field observations meaningful.
- [x] Durable document reads establish that committed edits are saved. Preserve
  meaningful save-state observations; a badge alone is not durable-save proof.
- [x] Undo restores the exact prior document content and canvas scene and saves
  the Undo result as the appropriate new revision. Cancellation restores the
  scene without creating an accepted edit or saved revision.
- [x] Restore the existing assertions with RED/GREEN evidence (A4). Keep unrelated
  collision/provider smoke scenarios and source/build/browser receipt checks;
  do not delete or bypass them to obtain a passing result.
- [x] Record which drag scenarios completed and the complete runner's actual
  outcome. A later non-drag failure remains visible and cannot be reported as a
  full smoke pass or silently expanded into unrelated repair work.
- [x] Update the drag evidence and historical acceptance records with current
  selector/workflow facts, tested source and remaining reload/full-gate limits
  (A8). Do not overwrite historical run results.

## Verification and completion

Use the existing supported Rust browser runner and checked packaged app. Record
actual commands, RED/GREEN evidence and source/build/browser identity. Run focused
runner controls when changing their behavior; preserve original semantic
assertions and receipt integrity. All existing drag/cancellation/Undo scenarios
must have terminal results. Full combined acceptance remains with 010.

## Comments

- 2026-11-06: The user accepted splitting the former drag/save/Undo/reload slice.
  This ticket restores the existing end-to-end proof; 009 adds a distinct reload
  boundary after this workflow is reliable. Parent spec and audit stay unchanged.

## Answer

**Scope:** the existing Rust drag harness (`tools/browser-harness/src/bin/drag_smoke.rs`)
now reaches, and passes, every Part, Column body, Stagger cancel, Stagger handle,
Ctrl Offset and Matrix move scenario with exact Undo on a current packaged build.
The complete runner still **fails** later, in the unrelated Case collision
scenario, so no success receipt exists and this is **not** a full smoke pass.
Logs are local to this worktree under `.evidence/ui-integration-recovery/008-*`
(the directory is git-ignored); run directories are under
`web/target/test-results/drag-browser/`.

### Diagnosis (harness assumptions, not product defects)

| Finding | Evidence |
| --- | --- |
| The Rows slider focus failure recorded in 026 **did not reproduce**: setup, the Rows/Columns sliders and "Create matrix" passed on both builds below. Not a defect on a current build. | `008-red-drag-baseline.log`, `008-r2-red-head-harness.log` |
| `.m1-save-state` no longer exists. The save status is the header `PopoverTrigger` `.header-save` with `data-state` (`ready` with no accepted document, then `saved`, `saving`, `failed` or `pending` from the durability state); its accessible name is "Saved locally". The wait timed out at the first `saved()` call. | RED runs; DOM probed in a private browser |
| `.m1-transform-handle` is now `[data-transform-handle]`, and the canvas `svg.m1-canvas` is `svg[role="group"][aria-label^="Keyboard layout"]`. Both were used for the trusted-gesture label and pointer-capture observation, so the old selectors could never match. | source search; green run observes pointer capture |
| Direct "Select Part/Column" buttons exist in the probed DOM and `select_scope` passed in every run; the harness does not log whether it used the direct button or the fallback scope menu, so the fallback's role/name lookup is unverified. | runs reach Column body drag |
| After Escape the scene equals the pre-drag scene about 160 ms later; the harness compared it immediately. A temporary tagged probe (since removed) reported `settled=true ms=161` and no differing scene entry (its single reported diff is its own length line, `400 400`). That identifies a read-before-restore race in the harness. It does not establish that 160 ms is acceptable app latency; no latency requirement exists in the specification. | `008-r2-debug-cancel.log` |

### Identities

- **First (contaminated) baseline.** HEAD `7688074cc` plus uncommitted work from tickets 018/020 in the shared tree
  (non-doc diff `1e8d8c1c2665…`). It is retained as a historical observation of the same
  `.m1-save-state` failure and is **not** isolated provenance.
- **Pinned run.** HEAD `ca065e679a84455f963434d62e5477e42e6d5453` with only the ticket claim and the harness edits in
  the working tree (HEAD later moved to a docs-only commit `b8bbcb3dd`). Checked build
  `python3 scripts/check.py build`: pass, 110.4 s (`008-r2-build.log`). Site digest (sha256 over sorted
  `site-subpath/boardstudio` file hashes) `2d333d9b52c069aaffabd45391dcfb3a3f467d2c0db12c86541e760f7f284d6a`,
  unchanged across every run below. Browser: system Chromium 156.0.8078.79, chromiumoxide 0.9.1,
  driver `python3 scripts/run-drag-browser.py`, dedicated per-run profile, local server on an ephemeral port (not the user's preview).
- Harness source: HEAD blob sha256 `3f77c52c…` (RED), final edit sha256 `97aef825c9195182b572c8c4c04f46e71da8195269603803ba934d47fc0d9706`.

### RED, then GREEN

1. **RED (HEAD harness, pinned build):** run `20261106T215232Z-ae631e5e66df` fails at
   `timed out waiting for a saved document: document.querySelector('.m1-save-state')…` after "Part drag starts".
2. Selector repair plus an explicit precondition (`require_save_status`: a missing `.header-save[data-state]` now fails
   with a named error instead of waiting): run `20261106T215304Z-18325f5320b8` passes Part drag and Column body drag,
   including exact Undo, and fails at `Stagger cancel restored the exact canvas scene` (the race above).
3. Only the cancel comparison now waits, up to 2 s, for the **same exact signature** (`scene_settles_to`); it still fails
   if the scene never equals the pre-drag scene. This does relax timing for cancel: a restore that lands within 2 s now
   passes where an immediate read would fail, and the specification sets no tighter bound. The Undo scene comparison
   stays immediate as before (a first draft also waited there, but the evidence did not require it, so it was reverted).
   No tolerance, signature or document comparison changed.
4. **GREEN for the drag scope**, final harness, run `20261106T215951Z-c6819311f672` (an earlier 5 s-wait draft passed the same
   scenarios in `20261106T215400Z-81d0a4d82c5b`): Part drag (revision 2 to 3, Undo saved as
   a later revision), Column body drag (4 to 5), Stagger cancel (no revision change, exact document and scene),
   Stagger handle (6 to 7), Ctrl Offset handle (8 to 9) and Matrix move (10 to 11) all passed, each Undo restoring the exact
   document and scene and saving a new revision. Trusted pointer events, movement count, capture, the gesture label,
   changed-field and preserved-field checks, and the timing bounds are unchanged.

### Terminal results of the complete runner

Stagger cancel prints no pass line of its own; its document, revision and scene checks run before the Stagger handle scenario, so
the run reaching "Stagger handle starts" at revision 6 is its pass signal, and the cancel mutants show those checks are live.

- **Passed:** setup, Part drag, Column body drag, Stagger cancel, Stagger handle, Ctrl Offset handle, Matrix move
  (with their Undo checks).
- **Failed (unrelated, not repaired here):** `collision_scenario` at
  `select_combobox_option("Case mounting style", "Rigid mount")`: *combobox Case mounting style is not a native select*.
  The harness assumes a native `<select>` that the current Select component no longer renders.
- **Not reached:** Default collision PCB pair, Case viewer with overflowing collision findings, Qualified collision
  product fixture, Unavailable physical PCB stays uncertain, the success receipt and its provenance validation.
  These checks and the receipt validators in `scripts/run-drag-browser.py` are untouched.
- A WebSocket reset handler error at shutdown appears in every failed run (known noise per 026); Chrome 145 pinned
  provisioning (ticket 005) was not used.

### Controls

- Repository mutation runner on the final harness (`008-mutation-run-final.log`, `008-mutations-final/`): six mutants killed,
  each by its **intended message**: absent save status, Undo scene, Undo document, Undo new-revision, cancel scene, cancel
  revision. The mutated file was restored byte for byte. The unmutated run already fails later at the collision step, so
  "killed" is read from the message, not the exit code.
- Harness `cargo fmt --check`, `clippy -D warnings`, 7 unit tests (`008-harness-checks-final.log`), `test-drag-receipt-bytes.py`, `test-browser-tools.py`
  and the 10 consumer controls in `test-drag-browser-consumer-controls.py` (no receipt on every failure) all passed.

### Limits

- The collision/provider smoke is **red** at an unrelated control; a follow-up repair is required before 010's full browser gate.
- Other consumers still use the dead selectors and were not changed: `.m1-save-state` in
  `tools/browser-harness/src/cad_u1.rs:44` and `scripts/qualification-page-fixture.cjs:27`, and `svg.m1-canvas,.m1-transform-handle`
  in `tools/browser-harness/src/bin/graphite_feedback.rs` (lines 170 and 197).
- Reload persistence (009) and the combined candidate gate (010) are not covered. The linked Left/Right half behaviour of
  the setup guide and ticket 021 (repeated save Toasts) were not examined.
