# 015: Complete owner coverage, mutation controls and verification closure

Status: resolved
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: [Packaged A→B draft-safety proof](010-packaged-a-to-b-proof.md), [Layout explicit-submit owners](011-layout-explicit-submit-owners.md), [Parts single-draft owners](012-parts-single-draft-owners.md), [PCB placement, attach, assembly and new-net owners](013-pcb-placement-attach-assembly-net-owners.md), [Creation-form owners](014-creation-form-owners.md)

## What to build

The verification closure for draft-safe Update Now: prove every one of the
fourteen Class A owners from the census is protected, complete the
per-(owner, admission-key) label and remedy table, run the mutation controls
over the safety gate, add the packaged Layout-constraint leg, and close with
the full project gate and the combined spec-and-standards review. Closing this
ticket completes this spec only; it does not close the parent adoption
umbrella or claim its final acceptance.

## Acceptance criteria

- [x] Coverage audit: every Class A owner in the settled census maps to a registered adapter with passing mounted deferral evidence; any owner found unprotected is a blocker, not a waiver.
- [x] The per-(owner, admission-key) label and remedy table is complete and reviewed against the settled copy rule.
- [x] Packaged Layout-constraint leg passes: a real unapplied Layout constraint draft defers packaged Update Now and survives cancellation intact.
- [x] Mutation and rejection controls rerun over the registry polling, pruning and per-key unresolved branches using the project's mutation-check tooling; weakened-gate mutants are caught.
- [x] Class B remains an explicit, revocable risk acceptance with its recorded revocation trigger; no Class B adapter is added and no Class B loss is waived by this closure.
- [x] Full project check gate passes with its receipt recorded; one combined spec-and-standards review is completed and its findings resolved. (Gate PASSED — receipt `015-coordinator-full-gate-fdlock.log`; review PASSED — P6 GREEN, combined Spec PASS / Standards resolved.)
- [x] Evidence is logs-only in the evidence directory, reporting results and relevant failures; the completion record names the diagnostic surface chosen for dead-entry pruning and the cleanup attestations from the packaged legs.

## Proof map

Follows the [proof map protocol](../../../agents/issue-tracker.md#proof-map); the rows restate the
criteria above and add none. They cite 008–014 evidence and do not repeat any owner matrix. Status (coordinator, final review recorded): P1 GREEN, P2 GREEN, P3 `reused`, P4 GREEN, P5 GREEN, P6 GREEN (combined Spec PASS / Standards resolved — see completion record).

- P1 All 14 — the settled census of 14 Class A owners ([006 handoff](006-agree-verification-and-implementation-handoff.md#answer))
  against the adapters registered at the closing `HEAD` → one audit row per owner naming its passing
  mounted deferral evidence from 008–014. Oracle: the census, not the registry's own list; an owner
  without passing evidence is a blocker. Native source counts and test-graph reach are not behavioural
  proof. Class B: no adapter added, risk acceptance and revocation trigger intact. Tier `native` audit
  over `mounted` evidence. Status: GREEN — coverage audit table below + counts independently
  re-grepped 2026-11-08 confirming every 008–014 count (checkpoint-cited logs unchanged).
- P2 Copy remedies — the per-(owner, admission key) label and remedy table → compared with the
  settled copy rule → every key names its consuming action, initial and installed copy stay
  distinct. Oracle: the copy rule and the table, reviewed. Tier `native`. Status: GREEN —
  coordinator dispositions close both flags: (a) assembly origin remedy
  vs 004 example: actual copy ("Place on selected board, restore the origin, or close") is CORRECT
  under the user-approved 013 policy that Matrix consumes the AssemblyDefinition tree and NEVER
  origin, origin is resolved only by Place; the 004 example ("Place, Apply to a matrix, or close")
  predates and is superseded by that decision. Flag closed, no copy change. (b) assembly/matrix row:
  no separate registry entry for the update gate — `AssemblyKey::Matrix` tracks its own submission,
  the tree draft is registered under `save`, and Matrix never resolves or exempts the Place-only
  origin (013 user decision). Flag closed.
- P3 Packaged Layout constraint — a real unapplied Layout constraint draft in the packaged build →
  trusted Update Now, then Cancel → the update defers and the draft stays intact. Oracle: packaged
  deferral and unchanged draft. Tier `packaged`. Status: `reused` FORMALIZED by coordinator after
  reading the final 010 Answer (accepted 010 layout-constraint leg, both Update Now entries, Cancel
  preserves 7.25, cleanup attestation) — no duplicate leg added. Additionally proven fresh on the
  optimized build in P5 (`015-p5s-optimized-layout-leg.log`: 57 PASS / 0 FAIL incl. both Cancel 7.25
  lines and adopted-B on hashed wasm).
- P4 Mutation — weakened-gate mutants over registry polling, pruning and the per-key unresolved
  branches → `scripts/mutation-check.py` → each mutant fails a test. Oracle: the tool's killed and
  survived report; a survivor is not caught and blocks. Tier `native`/`mounted`. Status: GREEN —
  original 7/7 killed (`015-mutation*/`, specs outside repo) AND independent rerun 6+1 killed,
  0 survived, `restored_exact` (`015-mutation-rerun{,-wasm}/`, console `015-p5s-mutation*.log`).
  Scope caveat (honest): sample, not an exhaustive per-owner matrix — final review judged the 6+1
  sampling sufficient for the criterion as written (extra-matrix option DECLINED, see P6).
- P5 Optimized and full gate — coordinator-owned: rebuild the disposable packages deleted after 010 →
  optimized validation, then the full `python3 scripts/check.py` gate → receipt recorded. Oracle: the
  gate's own result. Status: GREEN — full gate `015-coordinator-full-gate-fdlock.log`: "check: all
  steps passed: repo, tooling, build, lint, test, browser" (506.2s), run with the inherited-FD lock
  `BOARDSTUDIO_BUILD_LOCK_FD` per `check.py` header (note: earlier flock-wrapped invocations
  deadlocked at the browser stage and their logs `015-coordinator-full-gate-resume*.log` are NOT gate
  receipts; the original lint failure log `015-coordinator-full-gate.log` stands as the pre-fix lint
  receipt). Driver tooling re-receipt after P5 driver slices: `015-p5s-tooling.log` (34 tests).
  Optimized build `015-p5s-optimized-build.log` + fresh layout leg (see P3). Cleanup attestations
  `015-p5r-cleanup.log` + `015-p5s-cleanup.log` (`site-015-optimized{-ab}` absent, `site-002-ab`
  retained, port free, no sessions). Full gate ran before the scripts-only delta; the tooling step
  was re-receipted on the final tree (`015-p5s-tooling.log`, 34 tests) and doc-links re-checked
  (719 files) during review; the production Rust delta was inside the full gate.
- P6 Final review — one combined Standards and Spec review per the
  [review-report protocol](../../../agents/issue-tracker.md#review-reports) on fixed endpoints with every
  changed file read. Acceptance is `READY` only when P1–P5 and the evidence criteria hold, including
  the dead-entry pruning diagnostic surface and the packaged-leg cleanup attestations in a small
  completion record. Status: GREEN — combined Spec PASS / Standards CHANGES REQUIRED with sole
  Required finding (RED/GREEN mis-attribution) fixed in this edit; endpoints `240b0634..working
  tree`; reviewer checkpoint `/01a1bcbb-1ee6-74b3-ace2-5abda2159b63/notes/reviews/015-review-checkpoint.md`.
  Findings 2–6 fixed in this edit; optional extra-matrix finding DECLINED by coordinator (6+1
  sampling sufficient, no distinct failure rationale under test economy); optional fixture-helper
  refactor DECLINED (acceptable as-is, avoids post-gate tooling churn). Status `resolved`, READY.

## Progress (slice: P1 audit + P2 copy table + P4 mutations + P3 reuse evaluation, 2026-11-08; slice-time NOT resolved — final state resolved, see completion record + Answer)

Base `240b0634a` (014 committed; 013 `b99997d46`). Slice-time ownership honored:
only this ticket changed; `scripts/verify-update-adoption.py` and payloads read-only
(no new leg added, no packaged builds/browser started). No production, execution
or other-doc edits at slice close. No commits. Slice-time: acceptance checkboxes
unchecked; proof-map rows `planned` except where this slice recommends a transition
for the coordinator (P3 reuse is the coordinator's decision). FINAL pre-review inventory below is historical; final state (review recorded): Acceptance criteria
7/7 checked, Proof map all GREEN (P1 GREEN, P2 GREEN, P3 `reused`, P4 GREEN, P5 GREEN,
P6 GREEN), Status `resolved` — completion record + Answer below are authoritative.

### P1 — coverage audit: all 14 Class A owners protected (recommend GREEN)

Oracle: the settled census ([001 census](../inventory/001-draft-owner-census.md),
14 Class A incl. two 007 promotions), not the registry's own list. Each row names
the actual registered adapter (verified registration call site) and its passing
mounted evidence (accepted 008–014 coordinator reruns, re-grepped this slice).

| # | Census owner | Registered adapter (owner / key @ call site) | Passing mounted evidence |
| --- | --- | --- | --- |
| 1 | L1 Layout constraint composite | `layout-constraint` / `save` @ `web/src/presentation/inspector/layout_component_inspector.rs:1665` (`LayoutComponentInspector`) | `011-coordinator-corrected-browser.log`: 106 executed, 0 failed; packaged `010-layout-final.log`: 0 failures |
| 2 | L10 Geometry scripts | `geometry-scripts` / `apply` @ `web/crates/ui-shared/src/geometry_scripts.rs:226` (`GeometryScriptsEditor`) | `008-coordinator-retry-geometry.log` 6/6, `-helpers.log` 16/16, `-notifications.log` 30/0; packaged tracer legs |
| 3 | L8 Legacy position | `legacy-position` / `apply` @ `web/src/presentation/inspector.rs:151` (`Inspector`) | `011-coordinator-corrected-browser.log`: 106/0 |
| 4 | L15 Companion placement | `companion-placement` / `apply` @ `web/src/presentation/inspector/companion_placement.rs:355` (`CompanionPlacementInspector`) | `011-coordinator-corrected-browser.log`: 106/0 |
| 5 | A1 Parts generator parameters | `generator-settings` / `apply` + `upload` @ `web/crates/parts/src/parts/generator_settings.rs:1130,1194` (`GeneratorSettingsEditor`) | `012-coordinator-fit-p4-full-parts.log`: 97/97; P2 both key directions GREEN |
| 6 | A2 Module profile | `module-profile` / `save` + `save-rotary` @ `web/crates/parts/src/parts/module_profile_editor.rs:526,618` (`ModuleProfileEditor`) | `012-coordinator-fit-p4-full-parts.log`: 97/97 |
| 7 | A3 Mechanical fit profile | `fit-profile` / `save` @ `web/crates/parts/src/parts/mechanical_profile_editor.rs:228` (`ManualProfileEditor`) | `012-coordinator-fit-p4-full-parts.log`: 97/97 |
| 8 | A4 Assembly draft | `assembly` / `save` (tree) @ `web/crates/parts/src/parts/assembly_editor.rs:372` (`SavedAssembliesEditor`) + `assembly` / `place` (origin) @ `:1193` (`AssemblyDraftFields`); Matrix shares the tree draft per the 013 user decision (never consumes origin) | `013-coordinator-browser-final.log`: parts 106/106 |
| 9 | A5 Mounted-module placement | `mounted-module` / `placement` @ `web/crates/pcb/src/pcb_module_inspector.rs:150` (`PcbMountedModuleInspector`) | `013-coordinator-browser-final.log`: pcb 64/64 |
| 10 | A6 Module-attach options | `module-attachment` / `attach` @ `web/crates/parts/src/parts/modules_catalogue/module_attachment.rs:120` (`ModuleAttachment`) | `013-coordinator-browser-final.log`: pcb 64/64 |
| 11 | A7 New-net name | `new-net-name` / `create` @ `web/crates/pcb/src/pcb_wiring.rs:280` (`PcbWiringInspector`) | `013-coordinator-browser-final.log`: pcb 64/64 |
| 12 | S2 New matrix form | `matrix-setup` / `create` @ `web/crates/layout/src/objects/matrix_setup.rs:146` (`MatrixSetup`) | `014-coordinator-layout-final.log`: 266+2=268, 0 failed |
| 13 | S2b Mirrored-pair form | `mirrored-pair-setup` / `create` @ `web/crates/layout/src/objects/mirrored_pair.rs:181` (`MirroredPairForm`) | `014-coordinator-layout-final.log`: 268, 0 failed |
| 14 | L20 Existing-half setup | `existing-half-setup` / `create` @ `web/crates/layout/src/objects/existing_half.rs:608` (`ExistingHalfSetup`) | `014-coordinator-layout-final.log`: 268, 0 failed |

Result: 14/14 map to a registered adapter with passing mounted deferral
evidence; zero owners unprotected, zero waivers. Class B: no adapter added
(17 registration call sites cover exactly the 14 Class A owners above; multi-key
owners hold one registration per key), risk acceptance and revocation trigger
(006: observed loss, new B owner, or blur-ordering counterexample) intact and
untriggered in this slice. No scope expansion.

### P2 — per-(owner, admission-key) label and remedy table (recommend GREEN — review recorded, see P2 row)

Copy rule (004, as amended Pair-only): area label + remedy per admission key;
initial deferral names area + remedy; installed-phase wording is distinct;
cancel-only; values/focus preserved. Verified in source: initial template
`"{area} has an unapplied edit. {remedy}, then choose Update Now again."`
(`notifications.rs::blocker_message`); installed text `"The update is
installed. This page reloads once your work is saved and any invalid fields
are fixed."` (`runtime.rs::reload_when_safe`) — distinct from initial. Every
key below names its consuming action.

| Owner | Key | Consuming action | Area | Remedy | Rule check |
| --- | --- | --- | --- | --- | --- |
| geometry-scripts | apply | Apply script | Geometry scripts | Apply the script edit, or cancel the edit in the form | pass |
| layout-constraint | save | Save/Add/Convert/Remove constraint | Layout constraint | Save or remove the constraint, or restore its saved values | pass |
| legacy-position | apply | Enter / Apply position, Escape | Legacy position | Press Enter or Apply position to save, or press Escape to cancel | pass |
| companion-placement | apply | Apply placement | Companion placement | Apply the placement, or restore the saved values in the form | pass |
| generator-settings | apply | Apply generator settings | Generator settings | Apply the generator settings, or restore its saved values | pass |
| generator-settings | upload | Upload generator model | Generator settings | Upload the generator model, or restore its saved values | pass (same area, distinct per-key remedy) |
| module-profile | save | Save project module profile | Module profile | Save the module profile, or restore its saved values | pass |
| module-profile | save-rotary | Save rotary profile | Module profile | Save the rotary profile, or restore its saved values | pass |
| fit-profile | save | Save fit profile | Fit profile | Save the fit profile, or restore its saved values | pass |
| assembly | save | Save assembly | Assembly tree | Save assembly or close | pass |
| assembly | place | Place on selected board | Assembly origin | Place on selected board, restore the origin, or close | pass; note 004's example remedy (`Place, Apply to a matrix, or close`) predates the 013 user decision that Matrix never consumes origin — actual copy matches settled behavior. Flag CLOSED by coordinator (see P2 disposition): actual copy CORRECT under user-approved 013 policy, 004 example superseded, no copy change |
| assembly | matrix | Apply to selected matrix | (tree draft registered under `save`; no separate registry entry for the update gate) | Origin untouched per 013 user decision; tree still defers with Save remedy after Matrix lands | recorded. Flag CLOSED by coordinator (see P2 disposition): `AssemblyKey::Matrix` tracks its own submission, Matrix never resolves or exempts the Place-only origin (013 user decision) |
| mounted-module | placement | Save placement | Mounted module placement | Save placement (add any pending support first), restore the fields, or close | pass |
| module-attachment | attach | Attach module | Module attach options | Attach the module, restore the options, or close | pass |
| new-net-name | create | Form submit (Add net) | New net name | Add the net or clear its name | pass |
| matrix-setup | create | Create matrix | New matrix | Create the matrix or cancel the setup | pass |
| mirrored-pair-setup | create | Preview placement | New mirrored pair | Preview the pair or cancel the setup | pass |
| existing-half-setup | create | Create linked half | Mirror existing half | Create the linked half or cancel the setup | pass |

Coordinator disposition (pre-review, closes both P2 flags — see Proof map P2 for full text): (a) assembly origin copy CORRECT under user-approved 013 policy, 004 example superseded, no copy change; (b) assembly/matrix: no separate registry entry for the update gate, tree draft registered under `save`, Matrix never resolves or exempts the Place-only origin (013 user decision). Both flags closed.

### P3 — packaged Layout-constraint leg: recommend REUSED, no new leg added

The accepted 010 `layout-constraint` leg (`leg_layout_constraint` in
`scripts/verify-update-adoption.py`) already proves the P3 oracle and more: a
real imported Layout constraint draft (`7.25` surviving blur, `invalid == 0`)
→ trusted pointer Update Now at toast AND header → alertdialog deferral naming
`Layout constraint has an unapplied edit` + `Save or remove the constraint, or
restore its saved values` → Cancel → waiting state with exact draft (`x ==
7.25`) and focus on Update Now → then Add constraint, held-save deferral,
adoption to B, and exact reopened project/value/revision. Evidence:
`010-layout-final.log` (`layout toast/header: Cancel preserves exact draft and
focus` PASS, `0 failures`); accepted in the 010 Answer with cleanup attested
(`010-cleanup-attestation.log`). Adding a duplicate packaged leg would be a
redundant matrix with no distinct failure story. Coordinator formalized `reused` (see P3 row);
this slice started no packaged builds or browser runs. Note: 010
used diagnostic fast builds and its disposable outputs were deleted after
acceptance — any rerun needs fresh assets (coordinator P5 scope; fulfilled in P5 — optimized build +
fresh layout leg, see P5 row).

### P4 — mutation controls with project tooling: executed, 7/7 killed (recommend GREEN)

Ran `scripts/mutation-check.py` (the project's tool, not a manual substitute).
Specs live outside the repo at `/home/chris/.cache/tmp/boardstudio-015-mutation-spec{,-wasm}.json`.

- Native registry/gate (`cargo test --locked -p boardstudio-web-runtime --lib`):
  6 weakened-gate mutants, **6 killed, 0 survived, 0 compile-failure**.
  `m1-dead-prune-removed` (dead entries block) → killed by
  `dropped_guards_prune_with_a_one_time_diagnostic_and_never_block`;
  `m2-prune-diagnostic-dropped` (prune records nothing) → killed by the same;
  `m3-stale-scope-prune-removed` → killed by `scope_switches_retire_stale_entries_silently`
  + `closing_the_project_retires_every_entry`; `m4-unresolved-never-reported`
  → killed by `unresolved_entries_block_with_area_and_remedy`;
  `m5-gate-drops-unresolved` (`unresolved_drafts[..0]`) → killed by
  `an_unresolved_draft_key_blocks_with_its_area_and_remedy` +
  `independent_blockers_collect_together`; `m6-diagnostics-never-drained`
  (`clone` instead of `take`) → killed by the one-time-diagnostic test.
  Evidence: `.evidence/update-draft-safety/015-mutation/` (`results.json`,
  `_baseline.log`, per-mutant logs) + console `015-mutation.log`.
- Mounted owner per-key branch (`python3 scripts/run-wasm-tests.py --files
  web/src/presentation/notifications.rs`): `w1-geometry-apply-never-unresolved`
  (`if live == baseline` → `if true`) → **killed** (baseline 30/0; mutant fails
  the geometry deferral tests, exit 1). Evidence: `015-mutation-wasm/` +
  `015-mutation-wasm.log`.
- Every mutant file restored byte-exact (`restored_exact` true; `git status`
clean under `web/` and `scripts/` after each run).
- Diagnostic surface for dead-entry pruning (named for the completion record):
  `web_sys::console::warn_1` via `emit_draft_owner_diagnostic`
  (`web/crates/runtime/src/runtime.rs:285-287`), drained once per poll from
  `DraftOwnerRegistry::take_diagnostics` in `poll_draft_owners`
  (`runtime.rs:962-973`). Message: `draft owner retired without unregistering:
  {workspace}/{owner}/{admission_key}; dropped, not blocking` — one-time per
  entry, never a blocker. m2/m6 prove both record-once and drain-once are
  test-guarded; stale-scope pruning stays silent as normal lifecycle.
- Scope caveat (honest, not a waiver): per-owner predicate branches beyond the
geometry sample are covered by each owner's RED/GREEN deferral proof and 011's
historical 8 manual mutants, not by an exhaustive per-owner mutation matrix —
deliberately avoided as a redundant matrix under test economy. Final review
judged the 6+1 sampling sufficient for the criterion as written (see P4 row).

### Slice validation, remaining gates, handoff

- Validation executed (slice-time, all serialized via `flock ~/.cache/tmp/boardstudio-build.lock`,
  supported flags/default config, no overlapping cargo): native runtime baseline
  `015-mutation/_baseline.log` 138 passed; mutation-check native 6/6 killed;
  mutation-check mounted 1/1 killed; evidence-log greps confirming 008 (6/16/30),
  011 (106/0), 012 (97/97), 013 (106/106 + 64/64), 014 (268/0), 010-layout
  (0 failures). Lint proof (FINAL): full-gate lint step PASSED per
  `015-coordinator-full-gate-fdlock.log`; `015-clippy-parts-wasm.log` is wasm clippy
  (no counts); `015-clippy-parts-browser.log` is the parts subset 106 passed. Slice-time
  only this ticket modified among owned paths — FINAL inventory below supersedes.
- Changed files + ownership proof (FINAL pre-review, supersedes slice-time sole-file note):
  working tree at base `240b0634` (uncommitted): 9 files for this ticket — the coordinator
  commit will contain exactly these 9: the ticket itself
  (`docs/plans/update-draft-safety/issues/015-coverage-mutation-and-closure.md`), 4 parts lint files
  (`web/crates/parts/src/parts/assembly_editor.rs`, `generator_settings.rs`,
  `mechanical_profile_ui.rs`, `module_profile_editor.rs`), `scripts/verify-update-adoption.py`,
  `scripts/test-verify-update-adoption.py`,
  `scripts/verify-update-adoption-payloads/loaded-binary.js`,
  `docs/plans/update-draft-safety/map.md` (resolution pointer per issue-tracker convention). Slice-time `web/` and `scripts/`
  were clean (mutation runs restore byte-exact); P5 added the lint fixes + driver slices above
  (no duplicate packaged leg added — P3 `reused`). Explicitly excluded (separate/external,
  untouched in this ticket's working tree): `AGENTS.md`, `.gortex.yaml`,
  `docs/agents/ai-documentation.md` (Gortex audit), `CODING_STANDARDS.md`,
  `docs/research/2026-11-07-ai-agent-test-economy.md`.
- Genuine RED/GREEN: no real code change in this slice, so no RED/GREEN cycle
  was owed; mutation kills above are control evidence, reported separately.
- Remaining gates (FINAL, review recorded): none for 015 — P6 combined spec-and-standards review
  on fixed endpoints EXECUTED, findings resolved (see P6 row + completion record; P5 optimized +
  full gate EXECUTED; both copy flags CLOSED by coordinator — see P2 row; P4 6+1 sampling judged
  sufficient — see P4 row; independent mutation rerun 6+1 killed; fresh-leg cleanup attested
  `015-p5r-cleanup.log` + `015-p5s-cleanup.log`). Slice-time note preserved: at slice close P5
  was unexecuted (disposable packages needed rebuild), P6 pending, reruns + fresh-leg cleanup
  outstanding (none added in slice; 010 attestations reused). No 019 closure, Class B adapters,
  geometry conversion, framework, or new CI.
- Virtual-notes checkpoint: `015-slice-checkpoint.md` (this agent's notes).

## Completion record (coordinator, final — review recorded)

Base `240b0634a15ec325f509cf3946896c1eec3fd685`, endpoints `240b0634..working tree`.
Status `resolved`; P6 GREEN, READY. Combined review: Spec PASS / Standards CHANGES REQUIRED —
sole Required finding (RED/GREEN mis-attribution of the P5 driver logs) fixed in this edit;
reviewer checkpoint `/01a1bcbb-1ee6-74b3-ace2-5abda2159b63/notes/reviews/015-review-checkpoint.md`.
Findings 2–6 fixed in this edit; optional extra assembly-origin mutant DECLINED by coordinator
(reviewer judged the 6+1 sampling sufficient for the criterion as written; additional per-owner
matrix lacks a distinct failure rationale under test economy); optional fixture-helper refactor
DECLINED (acceptable as-is, avoids post-gate tooling churn).

Newly executed (015): full gate `015-coordinator-full-gate-fdlock.log` (all steps passed, 506.2s,
inherited-FD lock); optimized build `015-p5s-optimized-build.log`; fresh optimized layout leg
`015-p5s-optimized-layout-leg.log` (57 PASS / 0 FAIL); mutation rerun 6+1 killed
(`015-mutation-rerun{,-wasm}/`, `015-p5s-mutation*.log`); driver tooling re-receipt
`015-p5s-tooling.log` (34 tests); driver GREEN run logs `015-p5-driver-tests.log` (32 tests),
`015-p5-loaded-binary-tests.log` (33 tests), `015-p5-packaged-diff-tests.log` (34 tests);
genuine driver RED receipts `015-p5-optimized-layout-leg.log` (FileNotFoundError in stamp_build —
resolver slice RED) and `015-p5r-optimized-layout-leg.log` (`=== post: 2 failures ===`
packaged-diff order — packaged-diff slice RED), GREEN receipt `015-p5s-optimized-layout-leg.log`
(57 PASS / 0 FAIL); loaded-binary slice GREEN-only, recorded as proof gap (no packaged RED
observed — the optimized run had aborted in stamp_build before the payload executed); cleanup
`015-p5r-cleanup.log` + `015-p5s-cleanup.log`; lint parts subsets
`015-clippy-parts-{wasm,browser}.log` (see validation note).

Reused (008–014, checkpoint-cited logs unchanged): P1 mounted counts 008 (6/16/30), 011 (106/0),
012 (97/97), 013 (106/106 + 64/64), 014 (268/0); P3 accepted 010 layout-constraint leg
(`010-layout-final.log`, `010-cleanup-attestation.log`); P4 original 7/7 killed
(`015-mutation*/`, console `015-mutation.log` + `015-mutation-wasm.log`).

P5 driver tooling scope (not production scope creep): `scripts/verify-update-adoption.py`
`page_wasm()` layout resolver + stamp/`packaged_diff`/swap routing; loaded-binary payload
driver-side path rendering (`loaded_binary_script`); `packaged_diff` sorted-expectation one-line
fix; three new driver tests (suite 31→34), each GREEN run log above. Genuine RED/GREEN:
resolver slice RED in `015-p5-optimized-layout-leg.log`, packaged-diff slice RED in
`015-p5r-optimized-layout-leg.log`, both GREEN in `015-p5s-optimized-layout-leg.log`;
loaded-binary slice GREEN-only proof gap (no packaged RED observed). Justification: 010 left
release/wasm-opt validation DUE to 015; driver hardcoded fast layout.

Diagnostic surface (dead-entry pruning): `console.warn_1` via `emit_draft_owner_diagnostic`
(`runtime.rs`), once-per-poll drain `take_diagnostics`, never blocks.

Unmet gates: none for 015. Retained cleanup: none (attestations above;
mutation specs remain outside repo by design). Next action: coordinator commit, then parent
019 — umbrella NOT closed by 015.

## Answer

015 closes draft-safe Update Now verification: 14/14 Class A owners mapped to registered
adapters with passing mounted deferral evidence (P1 GREEN), the per-(owner, admission-key)
label/remedy table complete with both copy flags closed under the 013 user policy (P2 GREEN),
the packaged Layout-constraint leg reused from accepted 010 proof plus a fresh 57 PASS / 0 FAIL
optimized leg (P3 `reused`), 7/7 + 6+1 mutation kills with `restored_exact` (P4 GREEN), and the
full project gate passed with receipt (P5 GREEN). Final combined review: Spec PASS / Standards
resolved (sole Required finding fixed in this edit); P6 GREEN, Status `resolved`. Verdicts and
receipts: completion record above; proof rows: proof map. Next: coordinator commit, then parent
019 — umbrella not closed.
