# Map: Typed Core edits

Label: wayfinder:map
Spec: [spec.md](spec.md) · Decision: [ADR-0006](../../adr/0006-replace-document-for-import-and-recovery.md)

## Destination

Product edits are typed Core intents that state what they change; Core owns their rules
and reports precise changed IDs. `ReplaceDocument` is used only for import and recovery
and for any bulk action a recorded decision permits.

## Notes

- Domain terms: [CONTEXT.md](../../../CONTEXT.md) (accepted document, pending edit,
  field edit, one-shot action).
- Skills: `tdd` for build tickets, `codebase-design` for operation and seam shape,
  `grilling` + `domain-modeling` for decisions, `code-review` before resolving.
- Tracker conventions: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
- For implementation handoff, use [handoff.md](handoff.md): complete matrix Core
  02/03 first, then the approved mechanical ticket 02.
- Starts after edit settlement's
  [ticket 20](../edit-settlement/issues/20-preview-only-direct-edit-event.md), which
  also edits Session's event types.

## Decisions so far

- Native mirrored halves (ADR-0008 amendment, user 2026-10-09; accepted in
  [16](issues/16-native-mirrored-halves.md), [ADR-0010](../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)):
  one measured half; Core `CreateMirroredHalf` copies, reflects and links matrices,
  layouts, parts, outlines, modules, wiring and mechanical features through one pair map,
  then reconciles on every edit. Exceptions per item. Build 17-22; 23 redoes keyboard
  [04](issues/04-refresh-included-keyboards.md) on it. Generator-parameters 06 lands first;
  its recipe reflection arithmetic is temporary.
- Generator parameters are typed end to end (option B, user 2026-10-09):
  [generator-parameters](../generator-parameters/map.md) 01-05. Closure projection uses
  generator normalization; saved strings are repaired by a v3 to v4 migration; Core
  validates parameters at its boundary. Its 02 restores closure mounts in the shipped
  splits and should land before 04's reviews.
- `ReplaceDocument` is reserved for import and recovery as the direction, no
  enforcement date; archive open/import and recovery reopen, footprint import and
  routed-board reference upload are permitted by name:
  [ADR-0006](../../adr/0006-replace-document-for-import-and-recovery.md).
- The first slice is the smallest bounded demonstration, `SetWiringMode`, to settle
  the pattern before larger concepts copy it:
  [edit settlement ticket 19](../edit-settlement/issues/19-decide-first-typed-core-edits.md#answer).
- Ticket 01 is resolved and establishes the pattern for later typed edits.
- Mechanical settings and outline versions come next, from
  [module deepening](../module-deepening/map.md) (user, 2026-10-07):
  [02](issues/02-mechanical-settings-patch.md#answer) has a confirmed design (user,
  2026-10-08): atomic
  settings/clearance projection, bidirectional mechanical mirror pairs for reused
  or distinct PCBs, and per-feature exceptions. The catalogue adapter stays;
  existing projects link explicitly, Make local is explicit, and fit conflicts remain
  accepted with findings. Linked deletion removes both features, omissions are
  explicit, rejoining selects source geometry with a preview, and new construction
  sharing is pair-scoped. Its shared Core edits
  integrate after matrix-layout-consistency 02/03. [03](issues/03-decide-outline-version-intents.md)
  is resolved: the user confirmed one typed outline-edit interface **including linked
  refinements** on 2026-10-09. See its [Answer](issues/03-decide-outline-version-intents.md#answer)
  and the dependency-ordered outline tickets 11-15. 01 and the outline planner split have landed.
- After mechanical 02 and matrix integrated verification, [04](issues/04-refresh-included-keyboards.md)
  redoes all included keyboards using the settled model (user follow-up, 2026-10-08).

## Progress

Accepted bounded106 membership unit: source0fc21ca5e, integrated465137ee5; fresh Sol medium PASS, shifted-compaction expectedRED→45GREEN, affected95.1sPASS and ONEnew-wrapper cleanup mutationRED→restored45GREEN. Full106/19OPEN; independent Parts/Layout, whole-matrix removal and combined acceptance remain.

Approved extension: [139](issues/139-generalized-mirrored-constraint-model.md) separately scopes the general constraint model for slanted axes and linked/Local endpoint relations; planning first, consumed by114, full19 scope retained.

User-reported mirrored outline gap controls: [141](issues/141-mirrored-outline-gap-controls.md), L3/E diagnosis and minimal synthetic reproduction; coordinate any confirmed reconciliation defect with B108. Two toggles are reported, but duplicate rendering, propagation and generated geometry are not yet distinguished by runtime evidence.

Current progress: [107](issues/107-reconcile-mirrored-hosted-electrical-items.md) resolved after SAME independent Sol medium acceptance, all3intended assertion mutations/restoredGREEN and final70/70. Source1a3c377a merged0bcc292412; root exact indexed/staged/reviewed hash, complete70symbol1fileCRITICAL48flow delta. Initial139 private math source4e55187d merged5f6e9a1298; later139solver/lifecycle/111 remain OPEN. Root coherent107/139 integration batchPASS859.9s plus explicittypecheck25.5s,72presentationtests/0failures;108fivecase source draft ready after safe B sync. E141 connected-perimeter probes preserve candidate-suppression counterexamples, actual user reproduction/root cause unproved. L7 formal acceptance next.106/138/142 resolved; full19/browser delivery OPEN.

Laptop resource cap: [142](issues/142-forward-wasm-optimizer-resource-cap.md) resolved. Optional Binaryen setting now forwards into CAD containers; defaults and Cargo caps are preserved. Source55caa3f2/integration2335fa65f2, fresh Sol medium PASS, focused1/class17 and root repo/tooling4.0s PASS.

Integration checkpoint:138resolved. Its cfg(test) nonempty-gap correction6bad928a was accepted by SAMEreviewer and integratede3eeb1be5; exact sourcee776d169 and complete2/2symbol MEDIUM graph. Original repo/tooling/lint/build plus resumed test/typecheck/browser424.2s and final affected Core Clippy all PASS;72main-app presentation tests/0failures. Rebuilt subpath output excludes51retired asset identities. Full binding: `demo-retirement-plan/root-final-capture/integration-cohort-result.json` in the baseline cache.106resolved;107 and isolated139source work proceed. No full19 or UI completion claim.

Progress2026-10-11: [106](issues/106-reconcile-mirrored-matrices-layouts-parts.md) resolved on the SAME independent Sol6.1 medium final crosswalk: six behavioral rows and all three original mutation guarantees satisfied. Final mode/tweak producer mutation compiled intended RED, exact817040restoration passed focused1/1. Source16b3234d/integration6c2fd1a525; root combined B106/F138 batch running.107 source implementation advances; full19/111/browser acceptance remains open.

Current app progress:137delivered; bounded106membership and independentPart/fixedLayout units accepted/integrated465137ee5/9d6439eeb. Combined106source16b3234d independentlyaccepted/integrated6c2fd1a525 after3compiledcollisionREDs and samewriterfix;106row/guaranteeclosure beingrecorded from existingevidence. Full19/111OPEN. F138acceptedsourcecbe6c792/integratede6ce90b389;51exacthash generatedassets removed. Historicalretiredlinkcleanup107links/61files complete; rootrepo/fullbatch/outputabsence/trackingcommit pending.139designaccepted, implementationfollows106. E21/41wait acceptedCoreoperations; L7focusedUIacceptanceHEAVYactive, latermerge. Laptop4build/4test/1suite; rootintegration/index/goldens; pollingPAUSED.

<!-- one line per resolved build ticket -->

- [137](issues/137-export-blocker-explanation.md) resolved: missing PCB blocker explanation fixed without changing readiness guards; source fa6b1bc8, SHA256 0e193336011ff1650b7b4c70eca29d7687142aedbe44539494bbf75a774571b0, merged no-ff at b618ece511339b89ece793711c68d17fa3a113cc. Fresh Sol 6.1 medium review PASS; independently reviewed compiled mounted WASM RED → GREEN 1/1 and reused repo/lint/typecheck evidence. Review: /home/chris/.cache/boardstudio-baselines/export137/independent-review.json. Synthetic fixture has no shipped/demo dependency.

- [133](issues/133-ec11e15244g1-encoder-qualification.md) resolved: front-only EC11 nominal-pattern/terminal-role qualification, fresh 3/3 focused and 90 crate tests (3 ignored), two restored producer probes; source `4a40faff`, integrated `6eeef607a`, full cohort PASS 510.0 s. Parent 118 remains open.
- [134](issues/134-reconcile-fixed-membership-matrix-assemblies.md) resolved: fixed existing assemblies and retained dependencies, fresh 9/9, three restored kernel probes plus non-pose/linked-key guard corrections; source `bcdef94c`, integrated `6eeef607a`, full cohort PASS 510.0 s. Parents 106/19 remain open and unactivated.
- [135](issues/135-sofle-v2-encoder-configuration.md) resolved: persisted the ticket 133-qualified explicit EC11 configuration for Sofle v2 SW25. Source `4689c99b`, integrated `78eb5069`; independent archive/export/preservation verification passed with a restored override mutation. The E12+F135 cohort passed all default checks in 372.4 s. Parent118 remains open.

- [136](issues/136-saved-project-publication.md) resolved at 4183cc0f1a08b554567faa3f9a01b74232689b7c: independent Sol 6.1 xhigh publication verification, six exact-restored meaningful mutations, native 2/2, Chrome Library 16 and complete source-bound graph MEDIUM (14/14 symbols, four files). Root integrated at 6163b9221; full cohort PASS 382.2 s. Binding: /home/chris/.cache/boardstudio-baselines/sofle-next-unit/publication136-verifier-final-source-output-binding.json. Ticket25/56/118 and physical/assembly-swap gates remain open.

- [131](issues/131-diode-sod123-opt-in-qualification.md) and [132](issues/132-reconcile-fixed-membership-matrix-geometry.md)
  are independently verified and integrated at `b01c106a4`: opt-in Vishay diode land-pattern qualification and
  fixed-membership multi-key mirrored geometry/preservation. Both have meaningful restored producer/kernel
  mutation evidence. One full sequential cohort batch passed all default steps in 539.1 s, including 70 presentation
  tests. Parent 106/19 remain open and unactivated; physical/system hardware ticket 118 remains open.

- 31 companion placement modes, 32 mirrored-half labels and
  [39](issues/39-stabilisers-and-companion-fallback.md) stabiliser/companion fallback rules are independently
  verified and merged from lane B. Ticket 39 verification is `4227567ae`; its eight mutations failed meaningfully.
  Placement modes apply to switch companions only; independent parts keep the general mirror rule.
- 25 phase 1 shipped-board audit and recorded hardware direction are merged from lane F. Phase 2 planning has
  an independently reviewed packet, tickets 42–104 (`a7857b49`): Sofle v2, RGB and Choc first, then the seven
  measured splits and remaining boards. Parent 25 stays open until the board work is verified. Preserve measured
  layouts; use BoardStudio adaptations and existing case generation for plate, gasket and tolerances.
  Screws and spacers may be added. The user subsequently delegated best-fit hardware selection and finding/adding
  missing footprints (2026-10-10); the boards team owns evidence-based BOM selection and component qualification.
- 37's delivered library/layout/page slice has independent verification at `fb2fe7bb7`, including a reproduced
  and fixed weak library-delete predicate, six bounded mutation failures, unchanged test lists and two sequential
  full browser passes (251.3 s and 248.3 s). The entire ticket remains `implemented (awaiting verification)`:
  carried suites, negative-idle proof, PCB recovery regression and speed evidence remain. Ticket 38 waits.
- 19 is split into independently reviewed tickets 105–117. The dependency chain starts with
  [105](issues/105-mirrored-half-derivation-kernel.md), ends with atomic activation in
  [111](issues/111-integrate-mirrored-half-reconciliation.md), and preserves the parent's original acceptance.
  [114](issues/114-mirrored-edit-compatibility-constraints.md) remains `needs-info`; it cannot silently pass through
  unsupported rotated/mixed constraints. Core children stay serial; the user approved isolated A/F streams with
  exact file/API leases. Every command/check remains globally sequential; activation and timing stay exclusive.
- [105](issues/105-mirrored-half-derivation-kernel.md) is resolved and integrated: private value-only part and companion
  derivation accepts existing identities and per-item settings; creation retains allocation and insertion. Fresh verification
  `09e07a7a` confirmed full acceptance, independently caught four companion/mode/tweak/stabiliser/fallback mutations and
  passed restored repo/lint/test/typecheck. Parent 19 remains open; next Core slice is 106, without production activation.
- **Integration batch check**, 2026-10-10 at `12a6784d`: one sequential foreground
  `BOARDSTUDIO_CHECK_JOBS=1 CARGO_BUILD_JOBS=12 python3 scripts/check.py` passed all default steps.
  Repo 0.1 s, tooling 23.4 s, lint 17.9 s, build 327.1 s, test 94.7 s, browser 350.1 s; total 813.3 s.
  All 70 presentation tests and 15 CAD browser checks passed. Log: `/tmp/boardstudio-tce-batch-12a6784d.log`.
  Batch includes the verified 37 slice, lane B 31/32/39, lane F planning and workflow tooling integration.

- 24 test audit, phase 1 (proposal delivered; the user subsequently approved its recommendations): about 2,150 tests, `test` 74.5 s and `browser` 159.1 s;
  the time sits in two debug fixture builds and fixed wasm sleeps rather than in the number of tests, so the audit
  proposes a short deletion/merge list plus speed-only changes: [Audit](audit/test-audit.md),
  [Outcome](issues/24-test-audit.md#outcome-phase-1-audit-and-proposal-only-nothing-deleted-edited-or-merged).

- 33 native test speed (audit phase 2, part 1): resolved. The shipped splits build once per test binary, `prepare` builds
  recipes in parallel (release fixtures byte-identical to the parent's), the 36 generators export through one KiCad
  run, the closure loop is 2 projects, the source-right test is merged, `kicad_parses` fails without KiCad. `test`
  69.9 s (implementer baseline) to 32-36 s with `opt-level = 1`, 39-41 s after the verifier reverted that profile line
  because the first `browser` run took 312.9 s (cold, confounded). Next: 34 (browser sleeps, now resolved for its pilot scope); re-test the opt-level
  with warm browser runs. [Outcome](issues/33-test-speed-rust.md#outcome).

- 34 browser test speed (audit phase 2, part 2), delivered scope: resolved. A bounded `wait_until` family
  (`web/crates/ui-shared/src/test_wait.rs`, test-support and wasm only) and the keymap `queued_edit_tests` conversion: keymap
  suite 37.0 s to 9.7 s, `browser` step 176.1 s to 126.8-149.9 s, 15 test names and 59 assertions unchanged. The verifier made the
  held-edit gates bounded and fixed three predicates that were weaker than the next assertion. The remaining suites and the audit
  merges are tickets [36](issues/36-sleeps-case-pcb-parts.md), [37](issues/37-sleeps-layout-library-ui-page.md) and
  [38](issues/38-merge-duplicate-browser-tests.md). [Outcome](issues/34-test-speed-browser.md#outcome).

- 36 browser test sleeps in case, pcb and parts, delivered scope: resolved (verified by Sonnet 5.5, 2026-10-10). Converted: case mounted
  settlement tests (13 of 16 settle sites), pcb `queued_edit_tests` (17) and 5 of 11 physical-setup sites; suites case 30.8 s to
  15.1 s, pcb 26.5 s to 18.3 s, test names identical, `browser` 144.1-164.9 s under heavy concurrent load. The verifier found and fixed
  waits weaker than the next assertion (case 2, pcb 4) and a pre-existing vacuous step (an "unrelated" edit blocked by save recovery
  never lands). Everything else, including all of `parts`, moves to [37](issues/37-sleeps-layout-library-ui-page.md) under "Carried from 36".
  [Verification](issues/36-sleeps-case-pcb-parts.md#verification).

- **Batch A boundary check** (tickets 18, 29, 30, 27, 26, 28; all Core and catalogue, the page compiles the shared
  model), 2026-10-10 at `92adcd611` in the nested `tce-matrix` worktree, sandbox lifted, one sequential foreground
  `python3 scripts/check.py` (default steps, not `release`), started 04:41:09, finished 04:50:28: all passed.
  Step times: repo 0.1 s, tooling 7.9 s, lint 9.7 s, build 206.8 s, test 90.4 s, browser 243.9 s, total 558.8 s.
  Browser: 70 presentation tests executed, 0 failed. No failure to report. This is historical evidence;
  the current implementation frontier is below.
- 28 embedded circuits mirror in `CreateMirroredHalf`: the circuit id takes the board rule, members, private nets and
  copied definitions move under `embedded/<mirror id>/`, an `EmbeddedCircuit` record and `embedded-circuit`
  correspondences are written, members take the independent-part pose, and a member reference a host part already
  holds takes the next free one (the verifier found the implementer's tests missed a duplicate, and added a test);
  collisions refuse by name; reuse-board copies no circuit (no instance pin). Two mutations redone:
  [Verification](issues/28-mirror-embedded-circuits.md#verification).
- 27 mirror modules in `CreateMirroredHalf`: pose through the pair map, heading by the independent-part rule
  (module not flipped), ids, `module` correspondences and one Warning `mirror/<id>/handed-module` per mirrored
  module (kept: no symmetry mark exists); a reference the mirror cannot follow refuses by name; reuse-board copies
  no module. Three mutations redone. Finding for the user: the heading rule turns a part rather than mirroring it,
  so pads off the heading axis (MCU, TRRS, display, encoder) are not an exact mirror even on the 90-degree grid:
  [Verification](issues/27-mirror-modules.md#verification).

- 30 independent parts take the full heading rule `r_map + 180 - r` over 360 degrees (a TRRS facing left
  faces right on the mirror), with anchored outline geometry reflected as `(x, -y)` in their frame; keys
  and companions keep the switch rule. Mutation-checked, and the world positions of anchors hold at five
  headings under an axis and a rotated map. The shipped halves differ for Corne 3, Sofle RGB 19 and Sofle
  v2 and choc 12 each (tickets 20 and 25 decide):
  [Verification](issues/30-independent-parts-use-the-full-heading-rule.md#verification).

- 29 key switches mirror their tilt with no half turn (the existing code already did; now pinned
  with 15 and -20 degree tilts and every shipped split, mutation-checked against `180 - r` and the
  ticket 18 `forward_angle` mutant), and Core adds a Warning finding `mirror/<part>/heading-uncertain`
  for a handed mirror part off the 90-degree grid (not for switches, diodes, holes or MX stabilisers).
  The diode side on the mirrored half (mirror puts it on the other side, the shipped right halves on
  the same side) is an open user decision: [Outcome](issues/29-mirror-switch-and-companion-headings.md#verification).
- 18 `CreateMirroredHalf` lands in Core (verified, one mutant left to 29 and now closed):
  [Verification](issues/18-core-create-mirrored-half.md#verification).
- 04 (partial, ticket still claimed) the ten split included keyboards open as explicit mechanical
  mirror pairs through the typed operation (Sofle linked, measured splits with local right-half
  bays, Sofle v2 gasket as the pair construction), unibody and review projects untouched, no key
  moved, a reused flipped-PCB demonstration and six Core request tests landed; repo, lint, build
  and native test pass, while mounted/live checks and the Standards/Spec reviews remain:
  [Outcome](issues/04-refresh-included-keyboards.md#outcome).
- 17 mirrored-half schema and format version 5 land: the mechanical pair gains optional
  `constructionSource` (recorded by `link-mirror-pair` and used by the closure rule, first member
  when absent) and `halves` (boards, correspondences, exceptions) with located validation that
  also runs on Open; the v4 to v5 step changes only `formatVersion`; the boundary transcript moves
  by `formatVersion` only. Verification removed an implementer change that could delete a half's
  own closure mounts and added the module kind: [Outcome](issues/17-mirrored-half-schema-and-format-5.md#outcome).
- 01 `SetWiringMode` lands through a typed Core intent with precise board IDs and ordered
  Undo: [Outcome](issues/01-set-wiring-mode.md#outcome).
- 15 integrated outline verification closes the outline phase: the typed outline transcript is
  appended (60 core entries, native and WASM parity), matrix, mechanical and outline actions hold
  their queue contract in both orders with linked refinements following and fixed copies staying,
  the full check passes at `77b9548b8`, and independent Standards and Spec reviews are dispositioned
  with Core settings validation, base-contour protection, linked-choice cleanup on every removal
  path and linked-refinement settings editing fixed and the rest recorded as accepted deviations
  or backlog: [Outcome](issues/15-outline-integrated-verification.md#outcome).
- 14 the outline web adapter lands: the planner is replaced by the typed adapter
  (zero `ReplaceDocument` consumers in the outline module, Generate through typed
  `CreateGenerated`), ownership is presented distinctly with located findings and
  blocked exports, one-shot Freeze retains the link, and hover/focus create no history:
  [Outcome](issues/14-outline-web-adapter.md#outcome).
- 13 typed Core outline edits land: one `ApplyOutlineEdit` operation with the
  kebab-case change vocabulary, Core-owned identity allocation and non-duplicating
  default names, atomic composite lifecycle, fingerprint-scoped retirement, and
  malformed-input errors without mutation:
  [Outcome](issues/13-core-outline-edits.md#outcome).
- 12 linked outline resolution lands: linked refinements rebuild from the board's
  current included components with version-local choices, source cleanup rides the
  same accepted edit/Undo, failed resolution keeps the link with a located last-valid
  finding, inactive alternatives cannot block, and the expected region count follows
  the current structure: [Outcome](issues/12-linked-outline-resolution.md#outcome).
- 11 Core outline ownership lands: explicit `OutlineOwnership` per version with
  absence-equals-fixed defaults (no migration), retired-identity reservation,
  cross-namespace collision refusal, version-namespaced derived identities, archive
  and browser-persistence parity, and no inferred links on legacy reopen:
  [Outcome](issues/11-core-outline-ownership.md#outcome).
- 02 typed mechanical edits land complete: Core-owned rules/inputs/findings, mirror pairs
  with per-feature sources, atomic lifecycle (removal, Q14 paired disable, legacy
  conversion), durable identity reservations, author-contour gasket resolution,
  per-target admission/export gating, Case adapters with reviewed fingerprints and a
  read-only projection port — integrated, fully checked, boundary transcript unchanged,
  independently reviewed with all blockers fixed through `2a8d5e076`:
  [Outcome](issues/02-mechanical-settings-patch.md#outcome). Children 05-10:
  [05](issues/05-core-effective-inputs.md#outcome),
  [06](issues/06-core-apply-settings.md#outcome),
  [07](issues/07-core-closure-projection.md#outcome),
  [08](issues/08-core-mirror-pairs.md#outcome),
  [09](issues/09-case-adapter-and-setup.md#outcome),
  [10](issues/10-integrated-verification.md#outcome).

## Ready for implementation

Parallel scheduling decision: [parallel streams](parallel-streams-review.md). The user approved B+A+F with audited
leases and sequential commands. B remains serial; A/F have isolated active slices after exact audits and the original 105
verification window ended. Full parent acceptance is not waived by partial slices.

- Native mirrored halves:17,18,26–32,39,105,132 and134 integrated. Bounded106membership accepted/integrated465137ee5 with45GREEN/affected95.1s and independentwrappermutationproof; full106/19OPEN. Next independentParts/Layout unit released; generalizedremoval/wholematrix/combinedacceptance remain. Selective independent-part/companion exceptions leave the rest linked;40placementcontrols companion-only.111activation and full19acceptance remain;139approvedgeneralconstraintmodeldesign is separate. Currentownership/evidence: [lane registry](lane-registry.md#coordinators).
- Board artifact implementation/reintroduction is DEFERRED: user-authorized retirement [138](issues/138-retire-shipped-demos.md) supersedes current board-delivery readiness. Historical [25](issues/25-shipped-boards-review.md) contract: one ordinary saved project per board through existing request examples and common operations. [136](issues/136-saved-project-publication.md) is resolved and integrated at 6163b9221 after fresh independent verification; publication closes its bounded saved-archive scope. It does not wait for all25 or shared19/22. Ticket25/56/118 and physical/assembly-swap gates remain open. Sofle v2/RGB/Choc are Inspector assembly choices in one project; [56](issues/56-sofle-sign-off.md) gates its review and assembly-swap evidence before remaining boards. Shared mirror/link 19/22 remains required separately, but board creation does not wait on 19/22/44. Tickets 44, 47–55, 58–78 and 80–103 are wontfix with supersession answers; their evidence remains historical. [46](issues/46-sofle-hardware-inputs.md) reflects delegated BOM selection with evidence gaps only; [57](issues/57-measured-split-hardware-inputs.md) and [79](issues/79-unibody-hardware-inputs.md) preserve genuine human hardware/material/process/tolerance gates.
- [133](issues/133-ec11e15244g1-encoder-qualification.md) and [134](issues/134-reconcile-fixed-membership-matrix-assemblies.md) are integrated after their full cohort PASS. B106 now has explicit wrapper approval and standing continuation authority as above, with its last bounded44/44 evidence retained; full106/19 acceptance remains open. Parent118 historical engineering gates remain open; board reintroduction is deferred.
- [119](issues/119-mx-hotswap-socket-qualification.md) is resolved for the selected nonreversible MX socket
  configuration: 2.55 mm front/back outer pad widths, 1.6 mm PCB and F/B placements at 0/90/300 degrees.
  Fresh independent verification killed a 3.0-to-3.2 mm drill mutation, restored exact production bytes and passed
  focused, repo/lint and all 77 footprint tests. Default 2.6 mm pads, physical fit/model alignment and whole-board
  qualification remain unqualified; [118](issues/118-sofle-adaptation-hardware-contract.md) stays open.
- [129](issues/129-choc-v1-hotswap-socket-qualification.md) is resolved for the selected Choc v1 socket
  configuration and integrated at `8e8f6bfab`. Independent verification of corrected commit `0c9a1178`
  killed the dimensioned lower-hole drill mutation, restored exact production bytes and passed repo/lint
  and all 79 footprint tests. The upper-hole diameter remains configuration-only; right-land dimensions,
  physical fit/model datum and whole-board qualification remain open. The earlier unsupported both-hole
  manufacturer attribution at `2cff220e` failed review and was explicitly corrected. Ticket 118 stays open.
- [130](issues/130-reset-switch-smd-side-qualification.md) is resolved for the bounded Panasonic side-reset
  generator configuration and integrated at `bf464be5e`. Fresh independent verification reviewed manufacturer
  drawing pixels and parser/model bindings, killed the lower-only 0.75-to-0.95 mm drill mutation, restored exact
  producer bytes and passed focused, repo/lint and all 81 footprint tests. Upper-hole diameter remains
  configuration-only; actuator datum, physical fit, model alignment, case access and ticket 118 remain open.
  The shared post-merge integration batch at `1ad6b95c8` passed in 401.0 s, including all 70 presentation tests.
- [128](issues/128-keycaps-queued-settings-waits.md) is resolved for its bounded keycaps queued-settings slice,
  independently verified and integrated at `1ad6b95c8`. Nine restored mutation variants and the original
  downstream assertion provide the recorded sensitivity evidence. The vacuous-wait variant observed `None`,
  not a completed wrong-depth edit; no completion claim follows from it. The full default batch passed.
- [132](issues/132-reconcile-fixed-membership-matrix-geometry.md) is an independently reviewed bounded child
  of 106 for fixed active membership, multi-key geometry and retained-key preservation. Its independent 2x3
  numeric oracle passed planning review. Fresh independent bounded verification passed focused 7/7, both
  intended kernel mutations with exact restores, and final repo/lint/test/typecheck (83.6 s). Integrated `b01c106a4`; full cohort batch passed 539.1 s. Ticket 132 is resolved; parent 106/19 remain open and unactivated.
- [131](issues/131-diode-sod123-opt-in-qualification.md) baseline preparation passed independent verification
  and is checkpointed on lane F at `4607c266`. The frozen 43-case current Rust baseline records 40 render/export
  successes and 3 render errors. Luna 6 high producer implementation passed fresh Sol 6.1 xhigh verification,
  including strict schema, 43 absent/false compatibility cases and separate width/polarity producer mutations
  with exact restore and green checks. Generated fixture is unchanged; integrated `42df5e642` and full cohort batch passed at `b01c106a4`.
  HIGH registry / CRITICAL schema consumer warnings are retained. Ticket 131 is resolved; physical 118 stays open.
- The next nine-test browser inspector proposal is deferred: 1.08 s of scheduled waits is not a measured gain,
  and negative-case causal proof remains unresolved. Shared helper impacts are HIGH; no implementation lease.
- Browser 37 remains partial; 38 waits for its complete acceptance. Cleanup 35 and closure 24 remain last,
  after mirror work is integrated; generated goldens belong to the orchestrator.
- The outline phase (11-15) is resolved. The remaining work is separate: matrix tickets 04-11
  (their own map) and [04: Refresh included keyboards](issues/04-refresh-included-keyboards.md),
  which is blocked only by matrix 11.

## Outline implementation frontier

Decision [03](issues/03-decide-outline-version-intents.md#answer) is resolved and its
build (11-15) is complete. The original proposal is retained as history, but deferring
linked refinements is superseded by the confirmed scope.

```text
03 approved
└── 11 ownership / identity / persistence
    └── 12 linked resolution / source cleanup
        └── 13 typed edits / atomic lifecycle
            └── 14 web adapter / linked presentation
                └── 15 integrated verification / reviews
```

- [12: Linked resolution](issues/12-linked-outline-resolution.md),
  [13: Typed edits](issues/13-core-outline-edits.md),
  [14: Web adapter](issues/14-outline-web-adapter.md) and
  [15: Integrated verification](issues/15-outline-integrated-verification.md)
  followed the dependency chain and are resolved with their recorded deviations.
- Mechanical tickets 02 and 05-10 are resolved; outline tickets own their own gates
  from here.
- Work proceeds in separate worktrees. The parent
  coordinates leases for shared Core model/migration/dispatch/geometry files and
  the boundary transcript; append migrations in actual integrated version order.
  Keep one Chrome owner and one transcript blessing owner.
- The keyboard refresh remains a separate follow-up with its recorded mechanical
  and matrix dependencies. Preserve the original implementation handoff's scope.

## Not yet specified

From the [resolver inventory](../edit-settlement/replace-document-resolvers.md). Each
becomes a typed intent, a typed batch intent, or a decision to add it to ADR-0006's
permitted list.

- Project and board renames (setup guide, Library, board inspector): likely next;
  two small operations replacing three resolvers.
- Other outline behavior beyond the confirmed ticket 03 contract requires its own
  decision; outline versions, settings and linked refinements are now specified in 11-15.
- Wiring/net: part-net assignment and the reviewed wiring plan, likely one typed batch
  intent with atomic identity.
- Layout membership and placement: atomic assign-to-layout, part/controller placement.
- Keycap resize plan: check whether existing keycap operations already express it.
- Parts definitions: rename, field patch, new component, models, assemblies, generator
  apply and upload.
- PCB part input settings, pin locks and board-reference edits.
- Bulk actions needing a typed batch intent or a permitted-list decision: physical
  setup, geometry-script apply, generator apply and model upload, assembly placement,
  reviewed wiring plan, matrix and mirrored-half creation, board creation.

## Out of scope

- Enforcing the import-and-recovery restriction (a later decision once the bulk actions
  are classified).
- Smaller or finer-grained Undo history.

Progress2026-10-11: accepted independent Part/fixed Layout106 unit source52ee10b2 integrated9d6439eeb9 after fresh SolMEDIUM PASS, two intendedRED sensitivity probes and exact-restored47GREEN. Full106/19 remains open; next coherent topology/removal completion precedes139 preparation. Evidence: ticket106 Outcome and ticket106-independent-parts-layouts/independent-medium/independent-assessment.md.
