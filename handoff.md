# Handoff (generated from records)

- Branch: codex/rust-v1-ui-parity-20261001 @ 6bb3d520
- Worktree: /home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001
- Parents: 53/60 accepted, 3 implementing, 4 planned
- Criteria: unassessed 15, missing 1, implemented 1, verified 204; 221 total
  - functional: unassessed 3, verified 178
  - release: implemented 1, missing 1, unassessed 10, verified 20
  - visual: unassessed 2, verified 6

## Served candidate
- Build: frontend-layout-canvas-escape-20261005
- Source commit: 6bb3d52024e898f0f4111baac9b84e307224b9e7
- Root: http://127.0.0.1:34826/
- Subpath: http://127.0.0.1:34826/boardstudio/

## Functional readiness
- Ready functional criteria: 2 investigate, 0 implement, 0 qualify (Layout 0 investigate/0 implement/0 qualify; PCB 0 investigate/0 implement/0 qualify; Keymap 0 investigate/0 implement/0 qualify; Keycaps 0 investigate/0 implement/0 qualify; Case 0 investigate/0 implement/0 qualify; Parts+Project 0 investigate/0 implement/0 qualify; Shared 2 investigate/0 implement/0 qualify)
- Functionally verified parents awaiting formal acceptance: 0
- Parents with unmet final joins: 4
  Layout: unassessed 0, missing 0, implemented 0, verified 27; ready investigate 0, implement 0, qualify 0
  PCB: unassessed 0, missing 0, implemented 0, verified 26; ready investigate 0, implement 0, qualify 0
  Keymap: unassessed 0, missing 0, implemented 0, verified 7; ready investigate 0, implement 0, qualify 0
  Keycaps: unassessed 0, missing 0, implemented 0, verified 13; ready investigate 0, implement 0, qualify 0
  Case: unassessed 0, missing 0, implemented 0, verified 39; ready investigate 0, implement 0, qualify 0
  Parts+Project: unassessed 0, missing 0, implemented 0, verified 39; ready investigate 0, implement 0, qualify 0
  Shared: unassessed 3, missing 0, implemented 0, verified 27; ready investigate 2, implement 0, qualify 0
    Investigate: F8.6-C02, F8.6-C03
Functional criteria verified; formal acceptance still outstanding:
  none
Exact unmet final joins:
  F8.6 → F3.7
  F9.2 → F3.7, F8.6
  F9.4 → F8.6
  F9.5 → F9.2

## Parents
- Accepted (53): INT.1, INT.2, BND.1, BND.2, F2.1, F2.2, F2.3, F2.4, F3.1, F3.2, F3.3, F3.4, F3.5, F3.6, F3.8, F4.1, F4.2, F4.3, F4.4, F4.5, F4.6, F5.1, F5.2, F5.3, F5.4, F5.5, F5.6, F5.7, F5.8, F6K.1, F6K.2, F6K.3, F6K.4, F6C.1, F6C.2, F6C.3, F6C.4, F6C.5, F6.6, F7.1, F7.2, F7.3, F7.4, F7.5, F7.6, F7.7, F7.8, F8.1, F8.2, F8.3, F8.4, F8.5, F9.1
- Hold (0): none

## Criteria on hold or blocked
- F8.6-C01 [unassessed]: Join six-workbench project-to-export journey after each required provider/consumer acceptance is ready.
- F8.6-C02 [unassessed]: Combine existing bounded receipts with remaining authored/generated/portable outputs and per-operation snapshot proofs.
- F8.6-C03 [unassessed]: Run focused coordinator races/failure/retry/cleanup and route return on the frozen joined candidate.
- F9.2-C01 [unassessed]: Complete per-workflow and aggregate paired qualification after required workflow joins; reuse only attributable unchanged evidence.
- F9.4-C01 [unassessed]: Run cross-frontend archive round-trip and actual storage-write checks on final routes.
- F9.5-C01 [unassessed]: Map each existing budget to eligible current evidence; preserve ineligible/failed baselines for applicability review.
- F9.5-C02 [unassessed]: Qualify applicable interaction/rendering performance on final integrated source.
- F9.5-C03 [unassessed]: Attribute worker, URL and resource cleanup to final output paths; leave unmeasured/ineligible checks open.
- F9.6-C01 [unassessed]: After F9.1, F9.2, F9.4, F9.5 and F2–F8 exits, reconcile every remaining placeholder/React island against inventory.
- F9.6-C02 [unassessed]: Prepare reproducible candidate provenance plus copied-data cutover and rollback dry-run after qualification gates.
- F9.7-C01 [unassessed]: After the concrete F9.6 patch is reviewed, obtain explicit approval for actual cutover, then qualify launch/compatibility/critical journey.
- F9.7-C02 [unassessed]: Retain React reference and rollback until adoption approval; retire only inventory-covered entrypoints after approved cutover.

## Open RF findings (33)
- RF-001: Shared presentation and Runtime are integration hotspots
- RF-002: Internal browser host types are exposed as crate APIs
- RF-003: CAD engine capabilities and host protocols drift apart
- RF-004: Immutable export tokens do not model export-owned commits
- RF-005: Geometric edit planning lives in frontend helper policy
- RF-006: Canonical, physical-instance and isolated sample scopes are easy to conflate
- RF-007: Runtime observation currently supports one subscriber
- RF-008: Archive packing capability is split from its UI options and asset resolution
- RF-009: Parity accounting and acceptance evidence are scattered
- RF-010: Cancellation has different guarantees at worker and kernel boundaries
- RF-011: CAD revision envelope has a JavaScript safe-integer ceiling
- RF-012: Renderer host relies on reflective method names and partial capability wrappers
- RF-013: Object tree containers have invalid required-child semantics
- RF-014: Host callback ownership must survive browser terminal events
- RF-015: Shared viewer controls can read a different source than the rendered scene
- RF-016: Rapid reference form edits can display values that were not accepted
- RF-017: Import completion can outlive the selected definition in the TypeScript reference
- RF-018: Canvas draft input and outline geometry use implicit competing coordinate frames
- RF-019: Layout 3D scene projection omits mounted module bodies
- RF-020: Imported project definitions leak into reusable Parts choices
- RF-021: Saved outline entity IDs depend on a page-local operation counter
- RF-022: Native frontend test harness duplicates the production presentation module tree
- RF-023: Worker-owned provider failures lack a deterministic public test seam
- RF-024: PCB finding overlay advertises interaction that its pointer policy disables
- RF-025: Case gesture ownership conflates accepted source with provisional render identity
- RF-026: Dioxus delegated pointer handlers cannot assume a DOM currentTarget
- RF-027: Generator validation exposes raw JavaScript stack text in the Parts UI
- RF-028: Selection liveness conflates a present context with a resolved target
- RF-029: Layout keyboard gestures depend on SVG focus rather than workbench scope
- RF-030: Dynamic select options can display defaults over retained accepted values
- RF-031: Mechanical profile targets and family defaults are split across presentation and controller rules
- RF-032: PCB-coupled Case readiness prevents authored-body preview and STEP delivery
- RF-033: 63 plain #[test]s in wasm-only presentation modules never run natively; one pre-existing mounted ...

## Qualification scope
- Scope: f86-integrated-project-export-20261005 - Paired six-workbench project-to-export journey
- Journeys: six-workbench-project-export
- State: qualifying

Last retro checkpoint: 2026-10-04T15:56:00Z

2026-10-05 current checkpoint: HEAD6bb3d520; sourcefixaf915dfc repairs focused canvas Escape editor exit. Published frontend-layout-canvas-escape-20261005 at34826, serverPID3203620, 129.22s package/1409sourceentries0mismatch. Native247/1ignored/page/reachability/headless12 passed and exactreceipts reused atcommit. PCB F5.8 and inventory F9.1 accepted. F3.7 was accepted then explicitly REOPENED on actual fractional saved-document contradiction; correctiondecision retained at evidence/layout-point-precision-20261005/f3-7-correction-decision.json (progress set-status rejects all accepted transitions; coordinator recorded explicit correction/history without changingtool).

PrecisionRED: sameexactc476r15 input candidate-import-only copy byteidentical. Public OutlineEdited1→Editpoints→point2click→standaloneShift onpoint (NOTarrow)→Escape→Save yields candidate r16X-8.238 versus original/reference r15X-8.2375. Only candidate revision+pointX changed. Repeat onalreadyroundedpoint staysr16, so repeated no-op doesn't explain extraF8revision. Actualarchives/comparison/receipt in evidence/layout-point-precision-20261005. Reviewer preset_batch_review confirmed. layout_escape_repair owns ONLY outline_lifecycle.rs + outline_lifecycle_browser_tests.rs, fractional meaningful mountedRED/fix/GREEN authorized; tests nowedited, no rootcompiler. If productionowneroutsidelease needed, askroot exactfile. Do not claim mountedintegernonmutation proves fractionalcase.

F8 sameproject journey recorded in evidence/f86-integrated-20261005: identicalcandidateeditedr15 importedreference; candidatehadpriorpointinteractionr16. Column1X2→3, PartsNPTHJ1placed+explicitX60/Y-40, PCBResolve matching diode-directionerror (noApply), MainSW1 KeypressA, KeycapslegendRUN, authoredplate thenConfiguremechanicalstack. Candidate exact/current; reference authoredmaterialmissing is knownacceptedF8.5difference, generatedmechanical4parts currentr23. ActualExportready rowsonboth, candidateportedarchive r25 vsreference23; no generatedpackage output yet. Bothjoinedarchives downloaded/copied and6assetsidentical, newJ1/bodyidmapping leaves20rawdiffs in archive-comparison.json: realpointprecisionbug, orphanPLA vsmissingmaterialId preserved, emptygeneratorParameters vsmissing, ~1e-14floats, revision+2. Oneextra revisionattributedpoint; case_local_export read-only tracingremaining1 in revision-attribution.md only. Do not inventcause. Snapshot/layout/selectionbranches retained; F8 criteria remainunassessed/holduntilF3fix and currentoutputs/portable-roundtrip/returnnavigation. Existing reuse-map independentlyCLEAR for owner/snapshot/race/URL/theme/keyboard boundaries; no newfaultmatrix. Export afterbrowserreload correctlyrequiresCasecurrentgeneration again.

Browserbinding browser valid, often tabs disappear after unrelated agentcompletion despite markHandoff; use browser.tabs.list/new only, nevergetForUrl again for stalestabs. Actualsaved state survives. CurrentprecisionTab34826lastusedr16roundedproject, reference5175r15 aftersameaction; fullF8states safelydownloaded. Nodevars escapeFS, f86Evidence, precisionEvidence, f86Steps hold evidencepaths/checkpoints; no hiddenstate. Download waiter avoided because hangs; actualnormalSaveclick and newattributed Downloads files are retained. Latest files Sofle v2(10)candidateF8,(11)referenceF8,(12)import-only,(13)candidateEscape,(14)referenceEscape,(15)candidateRepeat. Allcopiedinevidence. Preserve six unrelatedindexentries snapshot /tmp/case-layout-unrelated-index.json. Do notadd-A. Milestonepostpublication/status/inventory/F8/precision evidenceuncommitted; combine nextrequiredsourcecommit afterreview/gates. Accessibility/F9.3/mobile/optionalworkexcluded.

Latest author checkpoint: meaningful fractional mounted RED nowCONFIRMED via existing Runtime take_layout_component_inspector_test_events observer. The InspectorProbe mock records events thenreturns (runtime.rs1859); snapshotdoesnotmutate bydesign, so earlierpointerharnesspasses were invalid fornonmutation. Author nowstores initialclientcoords andskips finalpointer sampleonlyforzero-motionclick; preserve actual pointerupdisplacement evenwithoutintermediatemove and pendingdragsettlement. One targeteddisplacedpointerup assertion approvedfornewrisk. NoRootcompiler; authorGREEN/affectedmodulepending. Publicbug actualpoint2 -8.2375→-8.238; syntheticfixture RED endpointdiffers dueviewport but proves samezero-motioneditcontract, labelhonestly. Rootreviewer preset_batch_review awaitssettledsource; earlierF3correctionendorsed. F8extra1revision stillunattributed afterboundedsourcecheck (revision-attribution.md); bothConfigurepathsoneedit. Afterrepair freshsameinputcomposition shouldcapture peractioncandidateSavecopyrevision andreference main[data-revision], withoutnewtesthooks. ExistingF8partialflow/actions preservedincheckpointJSON andjoinedarchives; no generatedpackage wasdownloaded yet.

Precision source frozen/review CLEAR; lifecycleSHA d88b54f350a4966c2a3197ed742e846982458055d77647c8478181dba6c7a6cd, browserSHA2580227086b0e73d61e8bc8d2c5a66287b070eed19b365ddd5123a37686b130c. Author meaningful event RED is explicitly a retained output transcription; GREEN1/1 and affected13/13. Combined coordinator gate terminal0 native247/1ignored,page,reachability,strictheadless13/13, exactreceipts retained precision/combined-gates.json. Source no-motion pointerup skips resample; displaced pointerup and pendingrealdrag preserved. F3.7 stays open until packaged actualsameinput savedcopy exactr15/X-8.2375. Commit explicit manifest /tmp/layout-point-precision-commit-paths.json includes owned milestone records and pendingF8/F91 evidence; sixunrelated indexentries preserved.
