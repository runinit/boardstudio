# Grouped F3.5/F3.6 qualification, 2026-10-04

Candidate: `frontend-review-fixes-20261004`, source `df2abad6`, served at
`http://127.0.0.1:34820/`. Reference: TypeScript `http://127.0.0.1:5175/`.
Browser: Codex in-app browser, fresh candidate Sofle v2 editable demo; reference
Sofle v2. Source files in the TypeScript reference were not changed.

## Mirrored key creation and edit comparison (F3.5-C05)

Both: Add object → Mirror existing half → All unpaired layouts on this board,
axis X 152.32 → Create linked half. Select mirrored keys / Column 1 / Key 1.1.
Both show **Use mirrored components immediately after creation**. This is the
`assembliesLocal=true` UI branch: reference `app/src/ui/MatrixInspector.tsx:84`,
candidate `web/src/presentation/objects/matrix_transform_inspector.rs:875`.
This creation state is shared reference behavior, not a migration defect.

Paired subsequent actions/results:

- Use mirrored components: both replace the button with the follow-paired-half note.
- Target Key Assembly → exact option `ergogen:ceoloide/switch_choc_v1_v2`:
  both retain Choc and expose the local reset button. Source key remains exact
  `assembly-preset-mx-hotswap-south-left-keys-0/definition/switch`.
- Source Remove diode: source attachment disappears; target local diode remains,
  along with target Choc assembly, in both frontends.
- Target Use mirrored components again: diode disappears in both. Both retain
  the target explicit Choc definition. Subsequent source Gateron assembly edit
  also leaves target explicit Choc in both; do not claim reset cleared that field.
- Target Key enabled off then Undo: Enabled returns true in both. Candidate
  loses selected part IDs (see reproduced defect below).

Native operation proof: `migration-deliver.py focused-test -- cargo test
--manifest-path web/Cargo.toml --locked --lib matrix_transform_operation`: 15 passed.
The initial `--bin boardstudio-web` attempt executed zero matching tests and was
correctly rejected; it is NOT passing evidence.

Portable archive capture through the browser download event timed out and reset
the browser connection. No downloaded archive or archive-content proof is claimed.
The creation verdict uses the mounted conditional UI and its source binding.

## Reproduced selection defect (F3.5-C01/C05)

Twice on the candidate: select mirrored Key 1.1, uncheck Key enabled, Undo, then
wait for Saved locally. Enabled is checked, but Inspector says **0 keys selected**,
Fit selection is disabled, and Relations lacks Edit placement relationship.
Reference retains **1 keys selected**, Fit selection and the relation action.
Clicking the same candidate tree key restores all three. Repair packet must begin
with a failing native regression; source/verification recorded separately.

## Inspector and compact keyboard journey (F3.5-C01/C04)

Desktop mirrored-key Properties and Relations enter with Enter. When selection
is live, Edit placement relationship routes to the current switch component in
both frontends. Matrix context shows Rows 4, Columns 6, Pitch 19.05/19.05,
Origin 9.18/-78.048 and Rotation 0 in both.

At 760×800, both compact Inspect drawers close with Escape from a focused tab and
restore focus to the Inspect trigger (`aria-expanded=false`, active label Inspect).
Enter reopens it with its previous Properties/Relations choice. Close inspector →
Objects (Enter) → keys Linked (Enter) opens matrix Inspector in both. Matrix
Relations names keys · mirrored; Enter on Edit placement relationship opens
left-keys-SW1 component properties in both. This does not prove every nested Back
path; earlier accepted Board/Outline Back receipts remain relevant.

## Layout 3D (F3.6-C03/C04, partial)

Candidate 3D reaches ready/updated. Dragging (550,250)→(650,290) visibly rotates
the rendered two halves on selected Left PCB; click (548,255) picks mirrored Key
5.1 and mounts its Inspector. View controls expose Top/Bottom/Isometric, rotation,
zoom and display modes; Top view, Zoom in and Fit layout controls execute.
Reference reaches 178/178 models; Fit and drag (360,520)→(460,565) visibly rotate
its board pair; click (343,550) picks SW9. Screenshots were inspected solely to
verify the rendered camera difference and ground pick coordinates, not cosmetics.

Candidate first settled 3D→2D return preserves picked Key 5.1, X219.41/Y-69.15,
and 138% camera. An immediate click during a subsequent 3D render timed out;
after preview updated, 2D works and preserves those same values. Do not infer
GPU/listener cleanup, context-loss recovery, or performance from this observation.

## Acceptance

No parent accepted by this receipt. Independent review:
`../layout-grouped-review-20261004.md`. F3.6 still depends on F7.3 acceptance;
remaining lifecycle and context coverage must remain open. F8/F9 prerequisites
and unassessed criteria are unchanged.

## Additional settled observations

Three candidate 3D/2D cycles completed with identical Key 5.1, X219.41/Y-69.15,
138% return state. No error-level browser logs were recorded in the bounded check.
A further reference 3D/2D cycle returned to 125%. Selecting Right PCB clears
candidate key context to Right PCB Inspector and disables Fit selection; the next
3D render shows that single 70-part board, replacing Left PCB's linked pair.
Viewport overrides were cleared after compact testing (reference restored
1222×940, candidate 1222×940).

On Right PCB's independent keys matrix, candidate Origin X draft 216 → Escape
restores 212.77; 216 → Enter commits; settled Undo returns 212.77. Pitch X -1
shows “Enter a finite number greater than zero”; Escape restores 19.05.
Reference matches the accepted values and rejects -1 (“Enter a value above 0.”).
Reference Escape additionally returns to board context; reselecting matrix shows
unchanged 212.77. Candidate keeps matrix context. This context difference is
recorded, not silently treated as exact keyboard parity.

Candidate canvas Shift-click on grounded DOM targets
`matrix/right-keys/r0c0` and `matrix/right-keys/r0c1` shows **2 keys selected** and
Key 2.1 Properties. Tree Ctrl/Shift clicks remain single-selection. Reference
keyboard Shift+Enter changes the current key rather than adding one; the attempted
reference multi-selection path therefore supplies no matching two-key proof.

Candidate normal reload after Revision 15 Saved retained the linked 138-part Left PCB. Reopening mirrored Key 1.1 shows exact Choc definition, no diode, no local-reset button, proving these accepted edits survived reopen.

Minimal candidate selection reproduction also confirmed on untouched mirrored Key 1.2 (SW13): off → settled 0 selected; Undo → Enabled true but still 0 selected and Fit disabled. Prior assembly edits are unnecessary.

## Combined gate findings

The first enforced integration attempt passed the wasm32 page compiler, native web
lib/bin suite and test-reachability ratchet, then correctly blocked the commit:
`--files presentation.rs` ran the broad `presentation::` substring on one page,
reported 140 outcomes, two unexpected failures, and one incomplete test. No
commit was made. No new known-failure exclusion was added.

Direct fresh-page module probes on the unchanged Rust diff: `cad_presentation::mounted_tests::`
1/1 passed; `presentation::mechanical_settings::contextual_layer_tests::` 4/4 passed;
`presentation::panels::scroll_tests::` ran 3 with only the already-excluded
`compact_inspector_preserves_the_page_scroll_route` failing (it requires a compact
viewport). The previously unexpected compact Case assertion passed. These results
support shared-page interference in the broad run; they do not waive the existing
viewport-sensitive failure. Runner file selection is being repaired to use listed
module isolation, preserving incomplete/zero-test rejection.

## Settled integration gate

Source commit `f320e6b8`: enforced integration command passed the wasm32 page
compiler, native web lib/bin suite, test-reachability ratchet and the repaired
headless runner. File-selected listing covered 267 tests in 64 isolated modules;
the gate requires terminal outcomes for all listed tests. This is a completed run
under the checked-in 13-entry failure allowlist, **not** 267 passing assertions.
No exclusions were added by this repair. Nine externally added exclusions remain
unreviewed. The tooling suites independently passed 20 runner tests and 26 delivery tests.
See `integration-gates.log`, `../test-gates-review-20261004.md` and
`../layout-grouped-review-20261004.md`.

## Published repair replay — f320e6b8, port 34821

Full build `frontend-layout-grouped-repairs-20261004` completed and was published
before this replay. Detached server PID 2032771 serves root and `/boardstudio/`.
Package proof: `../frontend-layout-grouped-repairs-20261004/package-proof.json`;
source mismatches are empty and both served route checks passed.

Fresh candidate Sofle v2, desktop viewport 1222×940. Add object → Mirror existing
half → All unpaired layouts, axis 152.32. Target Key 1.2 (SW13) immediately shows
Use mirrored components, retaining the earlier paired creation result.

- Disable target Key 1.2 and wait for Saved locally: 0 selected, Enabled off,
  Fit selection disabled. Undo: **1 selected, Enabled on, Fit selection enabled**;
  Relations again exposes **Edit placement relationship**. Redo disables/clears
  generated selection again; a second Undo restores the same selected key.
  This closes the specific public regression reproduced on 34820/5175.
- Target assembly accepts exact `ergogen:ceoloide/switch_choc_v1_v2`. Replace diode
  accepts exact `ergogen:ceoloide/diode_tht_sod123`. Remove diode leaves the target
  attachment list empty. Undo restores that exact diode definition and keeps Choc;
  Redo removes it again. The live selection remains one key.
- After Saved locally and a normal page reload, reopening target Key 1.2 retains
  Enabled=true, Choc, no attached components and the local reset action. Canonical
  Key 1.2 (`left-keys-SW7`) still has the original left-keys switch and diode
  definition IDs; target edits did not rewrite the canonical half.
- Change to Right PCB: Inspector becomes Right PCB and Fit selection is disabled.
  Ordinary Key 1.1 (`right-keys-SW1`) disable → Saved → Undo restores one selected
  enabled key and Fit selection. Returning to Left PCB again clears the old scope.
- Select mirrored Key 1.2, enter 3D, observe `3D preview updated.`, then return to
  2D: one selected enabled key, Fit selection available, Choc/no-diode edits intact.
  Earlier paired orbit/pick and three-cycle evidence is reused; this replay makes
  no additional GPU disposal, context-loss or performance claim.

Catalogue unavailable/retry remains proven by the native production-policy
regressions and source review, with the hook-mount/public fault observation limit
in the author report. No provider fault was injected or claimed. Full Inspector
cardinality/nested routes, locked/driven public fixtures and broader viewer
lifecycle gates remain open. This receipt does not accept F3.5 or F3.6.
