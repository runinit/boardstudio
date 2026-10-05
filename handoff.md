# Handoff (generated from records)

- Branch: codex/rust-v1-ui-parity-20261001 @ 71a9a599
- Worktree: /home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001
- Parents: 53/60 accepted, 3 implementing, 4 planned
- Criteria: unassessed 15, missing 1, implemented 1, verified 204; 221 total
  - functional: unassessed 3, verified 178
  - release: implemented 1, missing 1, unassessed 10, verified 20
  - visual: unassessed 2, verified 6

## Served candidate
- Build: frontend-layout-point-precision-20261005
- Source commit: 4c0a2fc46074121b85c7f0e8fb0d80cd5c3f1021
- Root: http://127.0.0.1:34827/
- Subpath: http://127.0.0.1:34827/boardstudio/

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
- F8.6-C01 [unassessed]: Review retained fresh fixed-candidate six-workbench/output/roundtrip/return evidence and unchanged owner/theme/keyboard proofs; acceptance waits newly demonstrated coordinate history repair and reo...
- F8.6-C02 [unassessed]: Review retained fresh fixed-candidate six-workbench/output/roundtrip/return evidence and unchanged owner/theme/keyboard proofs; acceptance waits newly demonstrated coordinate history repair and reo...
- F8.6-C03 [unassessed]: Review retained fresh fixed-candidate six-workbench/output/roundtrip/return evidence and unchanged owner/theme/keyboard proofs; acceptance waits newly demonstrated coordinate history repair and reo...
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

Latest active correction: required rapid coordinate Enter→Blur duplicatehistory repair. Actual source4c0a public candidate current34827 with exact same r17 J1 placement input: rapidX60Enter→Y-40Enter savesr20 vsTSr19; candidate twoUndo actualarchive stillX60/r22 vsTSX66.675/r21; thirdUndo restoresX. Owning mounted rawRED proves threeEdit events X60,X60,Y-40. Author layout_escape_repair fixed only inspector/layout_component_inspector.rs + layout_component_inspector_tests.rs; explicit Enter retry remains, matching followingBlur suppressed. Source hashes inspector6792aeb06e6f2feb8bcf7ed1b7e00b1a02997847140828a5995c36cf98147c76/tests12871c23ebebadf92ed7084a34c3b89cece04e9433b179be3311303a0010f0a4. Same reviewer preset_batch_review CLEAR. Combinednative247/1ignored,page,reachability,headless17passed; source stagedBEFOREgate to preserve receipts. Author/sourceleasesreleased. Commit/package/public rapidXY/twoUndo replay pending; do not repeat wholeF8journey. Explicitmanifest /tmp/layout-coordinate-commit-paths.json. Six unrelatedindexentries remain snapshot /tmp/case-layout-unrelated-index.json.

F8 fresh six-workbench journey COMPLETED at4c0a/34827 vsTS5175: evidence/f86-integrated-20261005/fixed-candidate contains allperworkbenchcheckpoints,currentpairedportablecopies r24/r23,actual20-entrymechanicalZIPsboth,candidateauthoredSTEP,exact ownrevisionmanifestproof,fullgeometryartifactcomparison,ExportBacktoCasecurrentboth,publiccrossfrontendimport/reload/savearchiveswith6assetbytespreserved. Reference→candidateProjectDoc exact; candidate→reference only16~1e-14floats. Fullnormalizedjoineddocdiff19 includes16tinyfloats,knownorphanpla/materialfielddifference,{}vsmissinggeneratorparameters,andrequiredphantomrevisionbug. Source-ownermapUNCHANGEDexport/runtime/session6bb→4c. Sole reviewer explicitlyretainallcompletedF8outputs/roundtrip/reuseC03failure/URL/scope/theme/keyboardownerproof; ONLY coordinatehistoryrepairbranchremains. F3.7explicitlyreopenedwithnewcoordinate/f3-7-correction-decision.json; originalprioroutlineprecisionrecoveryremainsvalid. RestoreF3thenacceptF8fromsamefinalreviewafterpublicGREEN.

Next release prerequisites map evidence/release-join-remaining-20261005.md fromread-onlyauthor: F9.4 root/subpathofflineupdate/stale/lazyassetrecovery and saved-writeattribution; F9.5 applicablepointercaps33/50/100ms p95 for30/100/200keys,5serialsessions10warmups/100samples,existingpointerdriver; sourcefreshness/applicability mustbeassessed, do notinventnewcaps orfaultmatrix. Thismapispreparation,unassesseduntiljoinsaccepted. Actualcrossimport/reload/savefreshF8proofnowexists; do notblindlyrepeatdurabilityproof justbecausemaplabelsUI Save separately—compareexactclause andprioracceptedownerproof. F9.2 aggregatepairedworkflow/theme/keyboard/desktopshortheight/zoomreview stillpending. F9.6 concretecutoverrollbackpatch andF9.7approvalrequiredlast. Accessibility/F9.3,mobile/compact,optionalworkexcluded.

Browserpersistentbrowserbindingvalid, f86Fixedtab89/current34827 andf86RefNowtab90/5175 maydisappearafteragentcompletion; browser.tabs.list/new only,no rebootstrap. f86FreshDir is absolute.../f86-integrated-20261005/fixed-candidate; exactprobe-01-placement.boardstudio r17 input. Candidate nonmutatingpartselection ObjectsExpandkeys→buttonJ1; referenceSVGbuttonJ1pressEnter (notpointerclick). RapidX/Ycontrolsactualspinbuttons X mm,Y mm. Filechoosers catchtimeout; downloadsnormalclick withbefore/newfilesystemnames notwaitForEventdownload. Helpers takingmutablependingstate MUSTacceptit asargument; Nodeclosure maycaptureoldtop-levelbinding afterreassignment. ReferencelegendEntercommitsonblur; waitcorrectacceptedrevision. ReferencePartsPlacecomponentauto-switchesLayout; clickingLayoutagain cancelsplacement. ExistingnormalbrowserstatecurrentafterUndo, inputarchivesretained; do notinferfromstaletabs.

Prior checkpoint (historical notes follow; current correction above supersedes F3status and F8unfinishedclaims):  source milestone4c0a2fc46074121b85c7f0e8fb0d80cd5c3f1021 repairs stationary outline point clicks. Candidate frontend-layout-point-precision-20261005 at http://127.0.0.1:34827/ and /boardstudio/, detached server3257059; keep previous34826/34825 alive. Package128.90s,1409sources/0mismatch,190assets eachroute/0mismatch,9fresh/23inherited commands,0warnings. Donor frontend-case-layout-controls-20261005 is full valid reuse source.

F3.7 recovered/accepted after independent reviewer preset_batch_review. Original acceptance and explicit correction decision remain historical. New recoverydecision precision/public-review-f3-7-decision.json. Source meaningful event RED is a clearly labeled transcription, GREEN1/1/full13/13; native247/1ignored,page,reachability,strictheadless13 allpassed. Fixture Runtime records events without applying state; mounted snapshot equality is not nonmutation proof; synthetic pointer capture shim only. Actual public sameinput outlinepoint2click→standaloneShift→Escape→Save now byteidentical input c476fcaf16a9150b11221427620047c55d4ad11d293315b7c56d45bd7f8b4cbe,r15,X-8.2375. DOMcirclefocus→BODY/0handles. Actual download Sofle v2(16).boardstudio copied precision/candidate-fixed-after-escape.boardstudio. Reference unchangedr15/exactpoint; four1e-14pose serialization deltas retained. Precision/RECEIPT,public-fixed.json,public-fixed-comparison.json,public-review.md own result. Source lease released; no compilers/authors running. Same sole reviewer retained.

Important gate reuse: this source prepare ran before staging, so commit reran because ONLY two @index source entries changed; allcommitgates passed. integration-commit.json/log retain evidence. Next batch stage explicit source paths BEFORE prepare-gates, then stage records without touching source to preserve exact receipt identity. No tooling change needed. Six unrelated indexentries remain exactly /tmp/case-layout-unrelated-index.json; never add-A. Source/review/milestone scope includes F9.1 inventory accepted earlier. Accessibility/F9.3/axe/semantic/AT qualification, mobile/compact and optional work excluded. Functional keyboard/focus/Escape remain.

F8.6 remaining: retained sameproject partial journey in evidence/f86-integrated-20261005. Original input integrated-layout-c1d3/candidate-edited.boardstudio r15c476. Old candidate had precisioninteractionr16 beforeF8; ref sameoriginalr15. Completed Column1X2→3, PartsNPTHJ1placement explicitX60/Y-40, PCBResolve same diode-directionerror/noApply, SW1KeypressA, legendRUN, authoredplate thenConfiguremechanicalstack. Generatedmechanicalreadyboth; refauthoredmaterialmissing remains knownacceptedF8.5difference. Actual globalExport ready and joinedportablecopies candidate r25/ref23 saved (Sofle v2(10),(11)), but NO generatedmechanicalpackage output yet. Oldraw20diffs retained: precisionbug(nowfixedforfreshinput), orphanpla vs missingmaterialId with materials=[]both, generatorParameters{}vsmissing, ~1e-14floats, revision+2. Precision explainsone; remainingrevisionUNATTRIBUTED. Sourceboundedcheck rejects unwitnessed extraConfigurecommit claim; no newdefect asserted. Existingreview-approvedreuse-map covers owner/snapshot/race/URL/theme/keyboard evidence at originalfixtureidentity; no newfaultmatrix or wholejourneythemerepeat. Need freshfixedcandidate integratedcomposition with bounded peractionrevision attribution (actualcandidateSavecopyarchives; ref main[data-revision]DOM), currentjoinedartifactdownloads/portable roundtrip/returnnavigation. F8thenF9.2/F9.4/F9.5/F9.6/F9.7. Final cutover/Reactretirement requires approval of concrete completed patch.

Browser persistent binding browser valid; latest precisionFixed tab88 at34827 contains exactr15input afterGREENsave. Tabs may disappear after agentcompletion; browser.tabs.list/new/goto existingorigins, no rebootstrap/getForUrl. Currentreference5175 savedr15; readonlysource5a472a9 with publicUIeditsallowed. Node escapeFS,precisionEvidence,f86Evidence available. Browser skill/docs alreadyread. Use DOM snapshots filtered/readonlyevaluate only; no screenshots/innerText unlessvisual. Filechooser waiter catchtimeoutimmediately. Download waitForEvent hangs: normalclick, before/newDownloads names, waitactualcompleted.boardstudio andcopy. No privateapp/storage inspection. Browserlocalstate survives reopenedtabs. No goalcomplete/blocked; remaining migration goal active.
