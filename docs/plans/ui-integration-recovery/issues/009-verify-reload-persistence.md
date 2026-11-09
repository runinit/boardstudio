# 009: Verify persistence across reload

Status: resolved
Type: build
Blocked by: [008: Restore real drag/save/Undo proof](008-restore-drag-save-undo-proof.md)
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Audit A5](../../../research/2026-11-06-dev-ui-integration-audit.md#a5--what-the-existing-green-results-actually-cover)

## What to build

Extend the repaired packaged-app workflow to prove that saved edited geometry
and a saved Undo result survive an actual page reload/reopen. This adds the
missing persistence boundary without inventing a requirement to preserve the
in-memory Undo stack across reload.

## Acceptance criteria

- [x] Use the real gesture and durable-save setup established by 008. Capture
  the expected document and rendered geometry from the accepted edit, then reload
  or reopen the same saved project and compare the restored result.
- [x] In a separate scenario where needed, perform an edit, Undo it before
  reloading, verify the Undo result was saved, then reload/reopen and verify the
  restored document and geometry match that saved Undo result.
- [x] Prove project identity and persistence across the boundary; do not replace
  loading with a direct fixture rewrite or rely only on the save indicator.
  The unrelated Case setup reload is not evidence for these scenarios.
- [x] Preserve 008's exact document/scene restoration assertions and semantic
  comparison rules. Do not relax numeric tolerances or normalize away an actual
  edit discrepancy to make reload pass.
- [x] Make the new assertions red-capable and demonstrate that they detect a
  wrong persisted/restored result before recording GREEN (A4). Use the repository
  mutation runner if mutation checks are needed for that evidence.
- [x] If reload exposes a product defect, diagnose the actual workflow before
  correcting it. Report a separate blocker if the required repair exceeds this
  bounded persistence behavior; do not substitute a weaker assertion.
- [x] Record current source/build/browser identity, both scenario results and
  the distinction between pre-reload Undo and post-reload restoration (A8).
  Keep the original audit and parent spec unchanged.

## Verification and completion

Run both packaged-app round trips through the existing supported runner and
rerun the affected 008 scenarios after extending the harness. Record exact
invocations, failing controls and passing results. Keep focused round-trip proof
separate from the full reconciled-candidate gate in 010.

## Comments

- 2026-11-06: Published as the accepted follow-on to 008, providing independently
  verifiable reload coverage once the existing gesture/save/Undo path works.

## Answer

**Scope:** the existing Rust drag harness (`tools/browser-harness/src/bin/drag_smoke.rs`)
now proves, on a current packaged build, that a saved accepted edit and a saved
Undo result each survive an actual page reload of the same saved project, with
exact document, revision, project-identity and rendered-scene comparisons. No
product defect was exposed, so no product code changed. The complete runner
still **fails** later, in the unrelated Case collision scenario (the known
008 follow-up), so no success receipt exists and this is **not** a full smoke
pass. Logs are local to this worktree under `.evidence/ui-integration-recovery/009-*`
(the directory is git-ignored); run directories are under
`web/target/test-results/drag-browser/`.

### What was added

- `accepted_part_drag`: one accepted Part drag through the existing trusted
  pointer gesture (trusted pointerdown/moves, pointer capture, live preview),
  the save-cycle tracker and durable IndexedDB reads, without 008's Undo:
  `cellOffset` must change, the other captured fields must be preserved, the
  saved document must differ and the rendered scene must change.
- `reload_and_verify`: captures the saved project's identity (`id` and `name`
  of the one local project), calls `page.reload()`, waits for the reopened
  rendered scene, then requires the **exact rendered scene signature**, the
  **mounted project identity** (the header project name, which the reopened
  application renders from its accepted snapshot, not from storage), the
  **same stored project identity**, the **exact page-canonical saved
  document**, and the **same saved revision**. No tolerance or normalization
  was added; 008's comparisons are untouched. The reload wait polls for that
  same exact scene signature (up to 20 s, through the shared
  `scene_settles_within` helper also used by the 2 s cancel check) rather
  than accepting the first mounted frame. The drag release/save-cycle
  sequence is likewise shared (`commit_drag`) between `run_drag` and the new
  accepted-drag helper.
- `mounted_document_proof`: after a reload, one more known Part drag makes
  the reopened application re-save its **mounted** document; that re-saved
  document must differ from the pre-reload saved document **only** in that
  drag's exact geometry: the dragged cell's `offset` in each matrix carrying
  it, and `pose.at` of the parts whose rendered transform actually moved
  (`DragGeometry`, by saved-document array position), compared uncapped
  (`diff_paths_all`). Undo of the proof drag then restores the mounted
  pre-drag document and saves it as a new revision, and that save must equal
  the expected document at **every** path — including the drag's own
  geometry — so no restored field, rendered or not, is proven only in
  storage or only by the scene, and no discrepancy is normalized away.
  (Scenario B's edit/Undo is this proof run against scenario A's result.)
- Scenario A ("Reload saved edit"): a Part drag is accepted and saved
  (revision 12 to 13), then reloaded and verified as above. Scenario B's drag
  is scenario A's mounted re-save proof.
- Scenario B ("Reload saved Undo"): a second Part drag is accepted and saved
  (revision 14), Undo is clicked **before** reload, the Undo result is verified
  as the exact pre-drag document and scene saved as a new revision (15), then
  reloaded and verified against that saved Undo result, followed by its own
  mounted re-save proof. This proves the saved
  Undo *result* persists; it does not require or claim that the in-memory Undo
  stack/history survives reload.
- The two new checks are recorded in the smoke checks, and
  `scripts/run-drag-browser.py`'s receipt verifier (plus its byte-fixture test)
  now requires both names for a future success receipt. Receipt, cancellation
  and save-revision checks from 008 are unchanged.

### Identities

- **Tested source:** HEAD `89812eaef84a7b73ef71a6e131b557a6ecc8e0ca` plus the
  uncommitted 009 working-tree diff as it stood at the final run
  (`git diff --binary HEAD` sha256
  `e2d55ba375affac86efbe359471b99479b65d3fd13d4cc94ac238ed2c5c66593`;
  harness source sha256 `8b5d080318087ea1c1f3825824287de5b585562875b37b4ee4bd74bde519e631`,
  which still matches the delivered harness). Wording edits to this Answer
  after the final run change the uncommitted diff hash by design; the
  committed tree is the definitive delivered-source identity. HEAD did not
  move during this work.
- **Package:** `python3 scripts/check.py build`: pass, 56.2 s
  (`009-build.log`). Site digest (sha256 over sorted `site-subpath/boardstudio`
  file hashes) `6e585a9ea876db409e4bef346f68636cf4201f344f95119015178976d592e36e`;
  `index.html` sha256 `0e5aa7de898049d85ed8bb40a260ea6e84c4e8d1cc4e130d7cd938b93c83d4ae`
  (unchanged from the 008-era build; the rebuilt digest differs from 008's
  recorded digest, so 008's build identity is not reused as 009 evidence).
- **Browser:** system Chromium 156.0.8078.79, chromiumoxide 0.9.1, driver
  `python3 scripts/run-drag-browser.py`, dedicated per-run profile, local
  server on an ephemeral port (not the user's preview).

### RED controls, then GREEN

1. First packaged run `20261106T224122Z-de7af0701f5c` (`009-run1.log`) already
   passed both reload scenarios, then failed at the known collision control.
2. Repository mutation runner (`009-mutations-spec.json`, evidence
   `009-mutations/` and `009-mutations2/`, `--skip-baseline` because the
   unmutated complete runner already fails later at the collision step, so
   "killed" is read from the failure message, not the exit code): nine
   mutants, each killed by its **intended message** — reload document equality
   inverted (`reload restored the exact saved document`), reload scene
   compared with a null scene (`reload restored the exact canvas scene`),
   reload stored project-identity inverted and mounted project-name inverted
   (both `reload reopened the same saved project`), reload revision inverted
   (`reload preserved the saved revision`), the pre-reload saved-Undo document
   equality inverted (`Undo restored the complete saved document`), and — the
   wrong-result controls added on review — reload verified against the
   **pre-edit document** and against the **pre-edit scene** instead of the
   saved edited result, both detected with the exact-document/exact-scene
   messages above (the document control reports the differing cell-offset and
   pose paths), and the mounted re-save proof compared with itself
   (`reopened document re-saved with only the accepted drag change`). The
   mutated file was restored byte for byte after every run.
3. **GREEN for the 009 scope**, final rerun after the mutants,
   `20261106T225930Z-beca6a839ca0` (`009-final-run5.log`; earlier final runs
   `20261106T224639Z-0f1f4e7e2400`, `20261106T225309Z-87a91d493401`,
   `20261106T225520Z-ab36ab373fb2` and `20261106T225733Z-08ee0433c3f1` passed
   the same reload scenarios before the mounted-document proof, its
   tightening, and the Undo-restored exactness step): all 008 drag scenarios
   rerun and pass (Part, Column body, Stagger cancel as in 008, Stagger handle,
   Ctrl Offset, Matrix move, each with exact Undo), then "Reload saved edit
   passed at revision 13" and "Reload saved Undo passed at revision 15".

### Terminal results of the complete runner

- **Passed:** setup, Part drag, Column body drag, Stagger cancel, Stagger
  handle, Ctrl Offset handle, Matrix move (with their Undo checks), Reload
  saved edit, Reload saved Undo.
- **Failed (unrelated, not repaired here; separate follow-up):**
  `collision_scenario` at `select_combobox_option("Case mounting style",
  "Rigid mount")`: *combobox Case mounting style is not a native select* —
  the same failure recorded by 008.
- **Not reached:** Default collision PCB pair, Case viewer with overflowing
  collision findings, Qualified collision product fixture, Unavailable
  physical PCB stays uncertain, the success receipt and its provenance
  validation. No full runner success or receipt is claimed.
- A WebSocket reset handler error at shutdown appears in the failed runs
  (known noise per 026/008).

### Other checks (rerun for 009, not inherited from 008 logs)

- Harness `cargo fmt --check`, `clippy -D warnings`, 8 unit tests
  (`009-harness-checks.log`, `009-harness-recheck4.log`): pass.
- `python3 scripts/check.py build`: pass (above).
- `python3 scripts/check.py repo tooling test lint`: pass, 129.0 s
  (`009-check-repo-tooling-test-lint.log`) — documentation links, tooling
  tests, the full native test suite and workspace/harness lint. The `browser`
  step was not run through `check.py`; the packaged runner above is the 009
  browser evidence, and the full browser gate remains 010's work.
- `test-drag-receipt-bytes.py`, `test-browser-tools.py` (13 tests) and the 10
  consumer controls in `test-drag-browser-consumer-controls.py` (no receipt on
  every failure; run with `CHROME_BIN=/usr/bin/chromium`): pass.

### Limits

- The collision/provider smoke is **red** at the unrelated Case mounting-style
  control; a follow-up repair is required before 010's full browser gate.
- Reload proof covers the one-project local-storage workflow the harness
  creates; multi-project selection after reload, Undo history across reload
  (explicitly not required), 019 and 021 are not covered.
