# Map: Typed Core edits

Label: wayfinder:map
Spec: [spec.md](spec.md) · Decision: [ADR-0006](../../adr/0006-replace-document-for-import-and-recovery.md)

## Destination

Product edits are typed Core intents that state what they change; Core owns their rules
and reports precise changed IDs. `ReplaceDocument` is used only for import and recovery
and for any bulk action a recorded decision permits.

## Notes

- Domain terms: [GLOSSARY.md](../../../GLOSSARY.md) (accepted document, pending edit,
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

- Refactor tickets (independent, no blocking edges; 185 is resolved, 186 is next):
  [185](issues/185-unify-switch-family-lookup.md),
  [186](issues/186-consolidate-cad-cache-eviction.md),
  [187](issues/187-extract-closure-hardware-edit-handling.md),
  [188](issues/188-isolate-gasket-screw-and-boss-selection.md),
  [189](issues/189-separate-mechanical-validation-from-construction.md),
  [190](issues/190-separate-session-request-construction.md),
  [191](issues/191-share-selection-set-updates.md),
  [192](issues/192-make-constraint-replacement-visibility-explicit.md),
  [193](issues/193-isolate-occupied-volume-transformation.md),
  [194](issues/194-investigate-gasket-resolution-phases.md) (research, no source change).

- **2026-10-19 architecture research and specs:** three parallel source-research
  passes narrowed the initial refactor directions to existing modules/interfaces.
  The user approved all three narrowed scopes and test seams; each ticket is
  `ready-for-agent`, with no blocking sibling dependency. Coordinate shared export
  tests and preserve existing working-tree edits. Static research is not behavioral
  verification or a measured performance claim.
  - [182: Reuse parsed KiCad board source across artifact projections](issues/182-reuse-parsed-kicad-board-source.md) — private parsing reuse, preserving separate preview/exact/import policies; test through the existing artifact request interface.
  - [183: Align generator editor null values with declared defaults](issues/183-align-generator-editor-null-defaults.md) — editor-local default handling; test through the mounted editor plus existing rendering/export parity coverage.
  - [184: Use direct forms for the plated mounting-hole generator](issues/184-use-direct-forms-for-plated-mounting-hole.md) — one generator reuses the existing build module; test through KiCad export and retained live generator goldens.

- **2026-10-19 test-suite speed & bloat audit:** six approved tickets slice the
  audit findings across disjoint crate boundaries (`176`–`180` in
  parallel, `181` blocked by `176`–`180` for the page suite, native baseline
  refresh, and full verification gate). All six are **resolved** Across two
  passes (**−10,518 net lines**: 2,126 insertions, 12,644 deletions; `test` step
  **116.3 s → 47.6 s**, `boardstudio_cad` lib **85.36 s → 7.88 s**, `pcb`
  browser **15.49 s → 6.69 s**, `parts` browser **12.11 s → 3.49 s**, full 56/56
  `check.py` gate **525.0 s → 366.9 s**, receipt
  `.evidence/pass2-full-receipt.json`):
  - [176: Speed up CAD layered, PCB, and STEP validation tests and prune STEP oracle self-tests](issues/176-speed-up-cad-and-step-tests.md) — resolved.
  - [177: Prune duplicate and no-op Core unit tests in mirrored_half and mechanical::edit](issues/177-prune-duplicate-core-unit-tests.md) — resolved.
  - [178: Trim redundant CreateMirroredHalf, ReplaceDocument, and pin-capacity loops in Core integration tests](issues/178-trim-core-integration-test-loops.md) — resolved.
  - [179: Replace remaining fixed timer sleeps and trim duplicate browser tests in PCB, Parts, Case, Runtime, and Host](issues/179-trim-sleeps-and-duplicates-in-web-crates.md) — resolved.
  - [180: Trim duplicate browser mounts, pure unit tests in WASM, and CSS assertions in web/crates/layout and ui-shared](issues/180-trim-layout-and-ui-shared-browser-tests.md) — resolved.
  - [181: Trim non-DOM and duplicate browser tests in the web/src page suite and refresh the native test baseline](issues/181-trim-page-suite-and-update-baseline.md) — resolved.

- **2026-10-18 input-boundary audit:** five researched follow-up tickets were
  created. Each is independently verifiable and has no blocking
  prerequisite. At the user's request, the two WRL findings share one decoder
  ticket. Primary-source findings and retained audit-evidence limits are
  recorded in [the research note](../../research/audit-input-boundaries-2026-10-18.md).
  - [170: Bound WRL decoding and refuse unsupported polygons](issues/170-bound-wrl-traversal-work.md) — resolved.
  - [171: Bound geometry-script emission storage](issues/171-bound-script-emission-storage.md) — resolved.
  - [172: Unsupported WRL polygons](issues/172-refuse-unsupported-wrl-polygons.md)
    is retired as merged into 170, not an additional work item.
  - [173: Mark interrupted native receipts incomplete](issues/173-mark-interrupted-native-receipts-incomplete.md) — resolved.
  - [174: Preserve generator free text in KiCad](issues/174-preserve-generator-free-text-in-kicad.md) — resolved; literal generator text/labels now survive KiCad export (commits `0ea5b26d5`, `3d77c263c`).
  - [175: Accept PCB outline UUID identities](issues/175-accept-pcb-outline-uuid-identities.md) — resolved; nonempty legacy timestamps retain precedence, modern UUIDs retain exact source identity, and fresh packaged Core/KiCad controls pass.
  **2026-10-19 implementation:** DeepSeek V4.1 Flash Free / High subagents
  implemented 170, 171 and 173 on `dev`; 172 remains merged into 170. Independent
  Standards and Spec reviews found no functional blockers; checklist/documentation
  findings were corrected, and a final delta review accepted the real application
  decoder browser regression. Native RED evidence covers the four new WRL refusals;
  the later output-size test guards existing behavior, not another pre-fix failure.
  The full six-step check passed in **525.0 s**: 56/56 commands, 2,077 native tests
  passed, 11 ignored, all 1,992 retained names matched, and 104 fresh page tests
  passed, plus mounted-crate and provider/product checks. Receipt:
  `.evidence/implement-170-173-full.json` and its `.json.md` reviewer summary,
  recorded at `a516109d7` with local documentation changes; subsequent commits
  only clarify documentation. Packaged WASM probes demonstrate shared-instance
  WRL recovery and bounded script refusal without accepted document changes.
  No remote CI, release optimization or extra mutation run was performed.
  These fixes do not reopen resolved predecessor tickets.

- **2026-10-18 retro follow-ups:** the user requested primary-source research and
  one ticket for each of four environment improvements. All four are **resolved**:
  implemented in `b4a4595b`, review corrections `4a49af46`. Four OpenCode Go DeepSeek
  V4.1 Flash implementation lanes and separate Standards/Spec reviews were completed.
  Confirmed review findings were fixed before the full six-gate check passed in
  **526.6 s** at clean code revision `4a49af46`. The new receipt records 56/56 commands,
  all retained native names and 103 fresh page tests passing. Remote CI was not run:
  - [166: Lint the WASM Runtime with Clippy](issues/166-lint-wasm-runtime-with-clippy.md).
  - [167: Guide authors to the complete collision-test fixture](issues/167-guide-complete-collision-test-fixtures.md).
  - [168: Return compact orchestration model lookups](issues/168-filter-orchestration-model-lookups.md).
  - [169: Generate scoped verification receipts for reviews](issues/169-generate-scoped-verification-receipts.md).
  Each ticket records its research, implementation, RED/GREEN evidence and limits.

- **2026-10-18 audit follow-ups:** the user requested a ticket for each finding
  from the recent-change audit at `f955050c3`. All four are **resolved**, integrated
  through `ed7fe9d5f`. Full `scripts/check.py` passed all six gates (1005.7 s), fresh
  occurrence-budget worker smoke passed 7 checks, and Claude Opus medium completed
  the combined Standards/Spec closure review with no blocking findings. The existing
  80-asset cache and bounded same-check refusal policy remain; broader CAD acceptance
  is unchanged. Implementation, RED/GREEN and review limits are recorded per ticket:
  - [162: Bound STEP occurrence expansion before geometry work](issues/162-bound-step-occurrence-expansion.md).
  - [163: Refuse unsupported via spans in physical PCB geometry](issues/163-refuse-unsupported-pcb-via-spans.md).
  - [164: Share tight collision refinement across repeated model placements](issues/164-share-tight-collision-refinement.md).
  - [165: Test STEP invariant rejection with corrupt imports](issues/165-test-step-invariant-rejections.md).

- **Curated layout-only library demos are back in scope** per the user on 2026-10-16; see
  [161](issues/161-curated-library-launchpad-demos.md). They are synthetic editable layout
  examples with no hardware or fabrication claims. The historical removal in
  [138](issues/138-retire-shipped-demos.md) remains accurate for the earlier shipped boards.
  Board-specific refresh, recipes, hardware research and sign-off tickets remain `wontfix`:
  [04](issues/04-refresh-included-keyboards.md) ([handoff](issues/04-handoff.md)),
  [20](issues/20-recipes-build-one-half.md), [23](issues/23-redo-keyboard-04-on-native-halves.md),
  [25](issues/25-shipped-boards-review.md), [42](issues/42-global-key-spacing-preference.md),
  [43](issues/43-apply-key-spacing-on-creation.md), [45](issues/45-reversible-variant-substitution-decision.md),
  [46](issues/46-sofle-hardware-inputs.md), [56](issues/56-sofle-sign-off.md),
  [57](issues/57-measured-split-hardware-inputs.md), [79](issues/79-unibody-hardware-inputs.md),
  [44](issues/44-native-recipe-assembly-profiles.md), [47](issues/47-sofle-v2-native-assembly.md), [48](issues/48-sofle-v2-real-components.md), [49](issues/49-sofle-v2-native-mechanics.md), [50](issues/50-sofle-rgb-native-assembly.md), [51](issues/51-sofle-rgb-real-components.md), [52](issues/52-sofle-rgb-native-mechanics.md), [53](issues/53-sofle-choc-native-assembly.md), [54](issues/54-sofle-choc-real-components.md), [55](issues/55-sofle-choc-native-mechanics.md), [58](issues/58-corne-native-assembly.md), [59](issues/59-corne-real-components.md), [60](issues/60-corne-native-mechanics.md), [61](issues/61-lily58-native-assembly.md), [62](issues/62-lily58-real-components.md), [63](issues/63-lily58-native-mechanics.md), [64](issues/64-klor-native-assembly.md), [65](issues/65-klor-real-components.md), [66](issues/66-klor-native-mechanics.md), [67](issues/67-sweep-native-assembly.md), [68](issues/68-sweep-real-components.md), [69](issues/69-sweep-native-mechanics.md), [70](issues/70-chocofi-native-assembly.md), [71](issues/71-chocofi-real-components.md), [72](issues/72-chocofi-native-mechanics.md), [73](issues/73-totem-native-assembly.md), [74](issues/74-totem-real-components.md), [75](issues/75-totem-native-mechanics.md), [76](issues/76-cantor-native-assembly.md), [77](issues/77-cantor-real-components.md), [78](issues/78-cantor-native-mechanics.md), [80](issues/80-gh60-native-assembly.md), [81](issues/81-gh60-real-components.md), [82](issues/82-gh60-native-mechanics.md), [83](issues/83-discipline-native-assembly.md), [84](issues/84-discipline-real-components.md), [85](issues/85-discipline-native-mechanics.md), [86](issues/86-mysterium-native-assembly.md), [87](issues/87-mysterium-real-components.md), [88](issues/88-mysterium-native-mechanics.md), [89](issues/89-voyager97-native-assembly.md), [90](issues/90-voyager97-real-components.md), [91](issues/91-voyager97-native-mechanics.md), [92](issues/92-voyager104-native-assembly.md), [93](issues/93-voyager104-real-components.md), [94](issues/94-voyager104-native-mechanics.md), [95](issues/95-plaid-native-assembly.md), [96](issues/96-plaid-real-components.md), [97](issues/97-plaid-native-mechanics.md), [98](issues/98-lumberjack-native-assembly.md), [99](issues/99-lumberjack-real-components.md), [100](issues/100-lumberjack-native-mechanics.md), [101](issues/101-reviung41-native-assembly.md), [102](issues/102-reviung41-real-components.md), [103](issues/103-reviung41-native-mechanics.md),
  [104](issues/104-preserve-reviung41-original-archive.md) and [118](issues/118-sofle-adaptation-hardware-contract.md).
  Their source evidence remains historical. Reopen only if the user brings board demos back into scope.
  Generic typed Core work remains active; mirrored-half tickets [21](issues/21-mirrored-half-web-adapter.md),
  [22](issues/22-mirrored-half-integrated-verification.md), [40](issues/40-set-companion-placement-operation.md)
  and [41](issues/41-inspector-companion-mode.md) are resolved against synthetic fixtures. Ticket 22
  records that per-item local exceptions do not apply to a reused physical PCB.

- [151](issues/151-explicit-mirror-only-part-placement.md) is resolved and integrated locally
  at `773aee9ad`: explicit mirror-only placement and bilateral linked catalogue additions share
  the prepared transaction. Native, full build/browser, real-WASM transcript and independent
  review/mutation gates passed. Parent 19 fresh full acceptance remains open.

- [109b](issues/109b-mirror-only-replacement-parts-auto-pins.md) is resolved and integrated at
  `d0783ca2` (source `1ee3e52f`). Candidate repository/tooling/lint/typecheck/build, 1,938 native
  tests, 89 browser page tests, 15 CAD checks, actual Chrome WASM boundary 1/1, and 12 compiled
  mutation kills passed; production restoration is exact. Automatic assignment reuses the existing
  resolver in the activated prepared transaction, with source assignments read-only and retained
  bindings/locks honored. The only approved visibility change is test-support `pub(crate)` access to
  the shared fixture helper. Root confirmed post-merge repo/test/lint session 6073 exited
  successfully with 1,938 tests executed; see
  `.evidence/ticket109b/integration/verification-current.json` and
  `.evidence/ticket109b/integration/post-merge-checks.log`. Parent 19 remains the next fresh full
  acceptance and is open.

- Native mirrored halves (ADR-0008 amendment, user 2026-10-09; accepted in
  [16](issues/16-native-mirrored-halves.md), [ADR-0010](../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)):
  one measured half; Core `CreateMirroredHalf` copies, reflects and links matrices,
  layouts, parts, outlines, modules, wiring and mechanical features through one pair map,
  then reconciles on every edit. Exceptions per item. Build 17-22; board refresh tickets
  [04](issues/04-refresh-included-keyboards.md) and [23](issues/23-redo-keyboard-04-on-native-halves.md)
  are deferred under 138. Recipe reflection arithmetic is retired with the demos.
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
- Included-board refresh [04](issues/04-refresh-included-keyboards.md) is superseded by the
  user-authorized demo retirement in 138; no built-in board rebuild is currently planned.

## Progress

  Tickets [21](issues/21-mirrored-half-web-adapter.md), [22](issues/22-mirrored-half-integrated-verification.md), [40](issues/40-set-companion-placement-operation.md) and [41](issues/41-inspector-companion-mode.md) are resolved. Ticket 22 verifies shared-board mechanical feature exceptions through save/reopen.

  All mirrored-half Core reconciliation tickets ([19](issues/19-reconcile-mirrored-halves.md), [105](issues/105-mirrored-half-derivation-kernel.md), [106](issues/106-reconcile-mirrored-matrices-layouts-parts.md), [107](issues/107-reconcile-mirrored-hosted-electrical-items.md), [108](issues/108-reconcile-mirrored-outlines-assemblies.md), [109](issues/109-mirrored-half-exceptions-unlink.md), [109b](issues/109b-mirror-only-replacement-parts-auto-pins.md), [110](issues/110-inverse-map-mirrored-half-edits.md), [111](issues/111-integrate-mirrored-half-reconciliation.md), [112](issues/112-inverse-map-hosted-electrical-edits.md), [113](issues/113-inverse-map-reviewed-outline-edits.md), [114](issues/114-mirrored-edit-compatibility-constraints.md), [115](issues/115-prepare-mirrored-edit-transaction.md), [116](issues/116-prepare-mirrored-open-repair.md), [117](issues/117-prepare-mirrored-preview-history-performance.md), [139](issues/139-generalized-mirrored-constraint-model.md)) are resolved and merged on `dev`.

Companion placement: a column splay now carries authored companions with their keys (`30903fe4b`). [143](issues/143-part-offset-constraint.md) adds a `part-offset` constraint kind that holds a companion along its key's axes (design settled in [ADR-0011](../../adr/0011-part-offset-is-its-own-constraint-kind.md), ready to build), and [144](issues/144-set-matrix-carries-authored-companions.md) carries authored companions through whole-matrix edits. [143b](issues/143b-part-offset-review-fixes.md) closes the review findings on 143's series.

New matrices on a mirrored board: [145](issues/145-mirror-new-matrices-on-a-mirrored-board.md) (blocked by 110) owes the forward transition that mirrors and links a matrix created after the pair exists; 110's inverse seam already supplies the source request and the planned source/mirror identity pair.

Follow-ups from the 145/112 review, all blocking 111: [146](issues/146-mirror-side-circuit-embeds-land-as-typed.md) accepts mirror-side circuit embeds with off-axis or turned members by carrying per-member source pose overrides; [147](issues/147-mirror-keycap-settings-and-plain-nets.md) mirrors keycap settings and plain nets after creation (omission removes orphaned nets); [148](issues/148-mirror-electrical-configuration-for-added-matrices.md) extends the mirror electrical configuration for added matrices.

Laptop resource cap: [142](issues/142-forward-wasm-optimizer-resource-cap.md) resolved. Optional Binaryen setting now forwards into CAD containers; defaults and Cargo caps are preserved. Source55caa3f2/integration2335fa65f2, fresh Sol medium PASS, focused1/class17 and root repo/tooling4.0s PASS.

- [138](issues/138-retire-shipped-demos.md) and [106](issues/106-reconcile-mirrored-matrices-layouts-parts.md) are resolved and integrated on `dev`.

<!-- one line per resolved build ticket -->

- [137](issues/137-export-blocker-explanation.md) resolved: missing PCB blocker explanation fixed without changing readiness guards; source fa6b1bc8, SHA256 0e193336011ff1650b7b4c70eca29d7687142aedbe44539494bbf75a774571b0, merged no-ff at b618ece511339b89ece793711c68d17fa3a113cc. Fresh Sol 6.1 medium review PASS; independently reviewed compiled mounted WASM RED → GREEN 1/1 and reused repo/lint/typecheck evidence. Review: /home/chris/.cache/boardstudio-baselines/export137/independent-review.json. Synthetic fixture has no shipped/demo dependency.

- [133](issues/133-ec11e15244g1-encoder-qualification.md) resolved: front-only EC11 nominal-pattern/terminal-role qualification, fresh 3/3 focused and 90 crate tests (3 ignored), two restored producer probes; source `4a40faff`, integrated `6eeef607a`, full cohort PASS 510.0 s. Its board-adaptation parent 118 is deferred under 138.
- [134](issues/134-reconcile-fixed-membership-matrix-assemblies.md) resolved: fixed existing assemblies and retained dependencies, fresh 9/9, three restored kernel probes plus non-pose/linked-key guard corrections; source `bcdef94c`, integrated `6eeef607a`, full cohort PASS 510.0 s. Parents 106/19 remain open and unactivated.
- [135](issues/135-sofle-v2-encoder-configuration.md) resolved: persisted the ticket 133-qualified explicit EC11 configuration for Sofle v2 SW25. Source `4689c99b`, integrated `78eb5069`; independent archive/export/preservation verification passed with a restored override mutation. The E12+F135 cohort passed all default checks in 372.4 s. Parent118 is deferred under 138.

- [136](issues/136-saved-project-publication.md) resolved at 4183cc0f1a08b554567faa3f9a01b74232689b7c: independent Sol 6.1 xhigh publication verification, six exact-restored meaningful mutations, native 2/2, Chrome Library 16 and complete source-bound graph MEDIUM (14/14 symbols, four files). Root integrated at 6163b9221; full cohort PASS 382.2 s. Binding: /home/chris/.cache/boardstudio-baselines/sofle-next-unit/publication136-verifier-final-source-output-binding.json. Board delivery and physical/assembly-swap gates are deferred under 138; the accepted publication evidence remains historical.

- [131](issues/131-diode-sod123-opt-in-qualification.md) and [132](issues/132-reconcile-fixed-membership-matrix-geometry.md)
  are independently verified and integrated at `b01c106a4`: opt-in Vishay diode land-pattern qualification and
  fixed-membership multi-key mirrored geometry/preservation. Both have meaningful restored producer/kernel
  mutation evidence. One full sequential cohort batch passed all default steps in 539.1 s, including 70 presentation
  tests. Parent 106/19 remain open and unactivated; physical/system hardware follow-through is deferred under 138.

- [31](issues/31-companion-placement-modes.md) companion placement modes, [32](issues/32-mirrored-half-labels.md) mirrored-half labels and
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
- [19](issues/19-reconcile-mirrored-halves.md) is split into independently reviewed tickets 105–117. The dependency chain starts with
  [105](issues/105-mirrored-half-derivation-kernel.md), ends with atomic activation in
  [111](issues/111-integrate-mirrored-half-reconciliation.md), and preserves the parent's original acceptance.
  [114](issues/114-mirrored-edit-compatibility-constraints.md) is resolved after 113 and 139. On
  2026-10-13 the owner chose mirror support for raw `SetMechanical` and selected an optional
  `MechanicalTarget` on the existing operation; [149](issues/149-targeted-mirrored-set-mechanical.md)
  records the implementation and remains a parent-19 blocker without widening ticket 111.
  [115](issues/115-prepare-mirrored-edit-transaction.md)
  is resolved and merged as `78ec73f0`. [116](issues/116-prepare-mirrored-open-repair.md) is
  resolved and merged as `1ad134f79`; its post-merge `repo`, `test`, and `lint` passed, all 1,776
  baselined native tests ran, and its three mutation checks were killed. [117](issues/117-prepare-mirrored-preview-history-performance.md)
  is resolved and merged as `dc9a2960`; its post-merge `repo`, `test`, and `lint` passed with all
  1,785 native tests, and its three mutation checks were killed. Ticket 149 is resolved and
  merged as `ebbca129`: post-merge `repo test lint` passes with all 1,817 native tests, and its
  14 compiled mutations were killed. [150](issues/150-prepare-matrixless-half-exceptions.md)
  is resolved and merged locally as `e8192350`: matrixless selective exceptions are reviewed;
  post-merge `repo test lint` passed with all 1,821 native tests, and seven distinct compiled
  mutations were killed in the relevant scopes with exact restoration.
  The next serial Core slices are 111 and
  [109b](issues/109b-mirror-only-replacement-parts-auto-pins.md).
  Ticket 149's optional target and single new crate-only mechanical configuration entry point
  are approved. Core children stay serial; the user approved isolated
  A/F streams with exact
  file/API leases. Every command/check remains globally sequential; activation and timing stay
  exclusive.
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

- **Batch A boundary check** (tickets 18, 29, 30, 27, [26](issues/26-mirrored-half-linked-mechanical-and-unchanged-parameters.md), 28; all Core and catalogue, the page compiles the shared
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
- [148](issues/148-mirror-electrical-configuration-for-added-matrices.md) resolved: new matrix
  assignments and locks follow member correspondences, Local/Omitted electrical-board overlays
  preserve their state, and missing controller pins refuse by function identity. Source
  `7efaf94ec`, merged `b9db54166`; focused 26/26, mutations 3/3 killed, native baseline 1695,
  independent Standards/Spec review clear, merged `repo test lint` passed. 111 still has other
  prerequisites.
- [113](issues/113-inverse-map-reviewed-outline-edits.md) and [144](issues/144-set-matrix-carries-authored-companions.md)
  are resolved on `dev` at `f3a808680` and `e644bc341`, respectively. Outline edits now map anchored
  geometry using each attached part's actual matrix membership; authored diode/LED companions
  follow whole-matrix transforms. Full merged `repo test lint` passed after both integrations.

## Ready for implementation

- [141](issues/141-mirrored-outline-gap-controls.md) is resolved: an explicitly authored cross-axis bridge opts one feature into a connected perimeter; default and unrelated splits remain, and the real Inspector independently edits the two recesses. User-authorized Core implementation: `e89366a4d273780279d5daecbf01ef6727300ec5`; full repository/provider/browser gate and both reviews pass.
- Open tickets in this plan: [140](issues/140-ui-design-impeccable.md) (UI design refinement), and (on `dev`) `161-curated-library-launchpad-demos.md`. Historical parallel scheduling notes: [parallel streams](parallel-streams-review.md), [lane registry](lane-registry.md#coordinators).
- Board artifact implementation/reintroduction is deferred under 138. Board-specific tickets
  04, 20, 23, 25, 42, 43, 45, 46, 56, 57, 79, 104 and 118 are `wontfix` for this scope;
  their source evidence remains historical. Generic mirror/Core and saved-user-project behavior
  remains separate and active. Ticket 22 uses authored synthetic fixtures and has no shipped-board
  acceptance dependency.
- [133](issues/133-ec11e15244g1-encoder-qualification.md) and [134](issues/134-reconcile-fixed-membership-matrix-assemblies.md) are integrated after their full cohort PASS. B106 now has explicit wrapper approval and standing continuation authority as above, with its last bounded44/44 evidence retained; full106/19 acceptance remains open. Parent118's historical engineering gates are deferred with board reintroduction under 138.
- [119](issues/119-mx-hotswap-socket-qualification.md) is resolved for the selected nonreversible MX socket
  configuration: 2.55 mm front/back outer pad widths, 1.6 mm PCB and F/B placements at 0/90/300 degrees.
  Fresh independent verification killed a 3.0-to-3.2 mm drill mutation, restored exact production bytes and passed
  focused, repo/lint and all 77 footprint tests. Default 2.6 mm pads, physical fit/model alignment and whole-board
  qualification remains unqualified; [118](issues/118-sofle-adaptation-hardware-contract.md) is deferred under 138.
- [129](issues/129-choc-v1-hotswap-socket-qualification.md) is resolved for the selected Choc v1 socket
  configuration and integrated at `8e8f6bfab`. Independent verification of corrected commit `0c9a1178`
  killed the dimensioned lower-hole drill mutation, restored exact production bytes and passed repo/lint
  and all 79 footprint tests. The upper-hole diameter remains configuration-only; right-land dimensions,
  physical fit/model datum and whole-board qualification remain open. The earlier unsupported both-hole
  manufacturer attribution at `2cff220e` failed review and was explicitly corrected. Ticket 118 is deferred under 138.
- [130](issues/130-reset-switch-smd-side-qualification.md) is resolved for the bounded Panasonic side-reset
  generator configuration and integrated at `bf464be5e`. Fresh independent verification reviewed manufacturer
  drawing pixels and parser/model bindings, killed the lower-only 0.75-to-0.95 mm drill mutation, restored exact
  producer bytes and passed focused, repo/lint and all 81 footprint tests. Upper-hole diameter remains
  configuration-only; actuator datum, physical fit, model alignment and case access remain unqualified. Ticket 118 is deferred under 138.
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
- Browser 37 remains partial; 38 waits for its complete acceptance. Cleanup [35](issues/35-test-maintenance-cuts.md) and closure 24 remain last,
  after mirror work is integrated; generated goldens belong to the orchestrator.
- The outline phase (11-15) is resolved. Remaining matrix tickets 04-11 are tracked in their own
  map. The separate included-board refresh [04](issues/04-refresh-included-keyboards.md) is deferred
  under 138 and is no longer a delivery frontier.

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

- [151](issues/151-explicit-mirror-only-part-placement.md): owner approved dedicated
  AddMirrorOnlyPart and explicit “Only this half” placement; bounded preparation follows 111 and
  precedes 109b. Ordinary additions are required to stay linked; readiness found catalogue and
  source-side addition gaps that 151 must close. 111 continues independently.


- [152](issues/152-reconcile-authored-matrix-cells.md): resolved private authored-cell/local
  companion and sparse-slot preparation; independent reviews, 214 focused tests, nine compiled
  mutation kills and full repo/test/lint pass with 1,826 native tests. Merged locally as
  `39f15a5e` (source `e72f7307`); post-merge repo/test/lint also passes with all 1,826 native tests.
- [153](issues/153-validate-independent-exceptions-with-matrix-owners.md): resolved private
  independent exceptions alongside exact single/multiple Matrix owners; both review axes accept,
  221 focused tests and six final refined compiled mutation kills pass. Full repo/test/lint passes
  with 1,833 native tests. Merged locally as `d897b057` (source `ab697e05`); post-merge
  repo/test/lint also passes with all 1,833 native tests. 111 activation remains unmerged.
- [154](issues/154-reserve-staged-circuit-member-references.md): resolved source/staged reference
  reservation using existing allocators. Both review axes accept, 31 hosted/223 reconciliation
  tests and four compiled mutation kills pass; full repo/test/lint passes with all 1,836 native
  tests. 155's durable links make the preserved bidirectional pose assertions pass. Merged locally
  as `08b469b59` (source `395fcd97`); post-merge repo/test/lint also passes with all 1,836 native
  tests. 111 activation remains unmerged.

- [155](issues/155-admit-hosted-addition-correspondences.md): resolved private durable admission
  of selected hosted Module/Circuit/member/private-Net links. Both review axes accept, 222 focused
  tests and eight final compiled mutation kills pass; full repo/test/lint passes with all 1,834
  native tests. Merged locally as `fb6bdbfb6` (source `1ee69ac54`); post-merge repo/test/lint
  also passes with all 1,834 native tests. 154's propagated-pose proof now passes; its local
  integration also passes; 111 activation remains open. No new policy/schema/visibility.

- [156](issues/156-preserve-validated-matrix-reconciliation-states.md): resolved private tombstone
  continuation and legacy unassociated child Open. Both review axes accept, 230 focused tests
  pass, six meaningful compiled mutations are killed (one inert validation probe survived),
  and full repo/test/lint passes with 1,843 native tests. The original public 111 regressions
  pass in its isolated draft. Merged locally at `96eaeabe0`; post-merge repo/test/lint passes
  with all 1,843 native tests. 111 stays unmerged.

- [157](issues/157-classify-matrix-membership-by-identity.md): resolved private identity membership
  classification, merged locally at e95d8e6e with post-merge repo/test/lint and1,847 native tests
  passing. Both reviews, four mutation kills and full
  repo/test/lint pass with1,847 native tests. Full authored public Splay/Placement now pass (Splay
  oracle sign corrected to existing rotation math). Isolated111 is60/62; two electrical failures
  remain.111 activation remains unmerged.

- [158](issues/158-retain-local-controller-private-net-on-omit.md): resolved existing 147-policy
  Local-controller net-retention correction, merged locally at `2cb402e2`; post-merge repository,
  test and lint checks pass with 1,849 native tests. Both reviews,
  two mutation kills and full repo/test/lint pass with1,849 native tests. Unchanged public
  controller Preview/Commit case passes;111 is61/62 with associated Local circuit projection
  remaining.111 activation stays unmerged.

- [159](issues/159-preserve-associated-local-circuit-owner-state.md): resolved preparation for
  associated Local owner/member/definition/private-net preservation and inherited member
  settings/pins, with independent joined-net states. Both review axes pass, six compiled
  mutation kills and full repo/test/lint pass with 1,856 native tests. All 62 unchanged public
  mirrored-half tests pass in isolated 111. Source `10fc4048` merged locally at `1a1f0fbb`
  with identical trees; post-merge repo/test/lint passes with all 1,856 native tests.
  111 still needs Preview loss visibility and full activation gates.


- [160](issues/160-show-paired-preview-electrical-loss.md): resolved exact electrical-loss
  Preview notices, passive rendered rows and clearing/stale ownership controls. Native/WASM
  transport and mounted Editor proof pass in isolated 111; eight mutations are killed.
  Preparation `aef4fd22` merged locally at `684e8e92`; candidate/post-merge repository/test/lint
  pass all 1,859 native tests (three additions, no removals). 111 activation remains unaccepted.
- [111](issues/111-integrate-mirrored-half-reconciliation.md): resolved coherent activation,
  candidate `4d63abc1` merged locally at `6c2f77eb` with identical trees. Final repository,
  1,904 native tests, lint, build, full browser and separate real WASM replay pass. Reviewed
  test-only rotated-map follow-up `904391c1` passes repository/native/lint with 1,905 native
  names and 65 public mirrored-half cases; production source remains identical to the full
  verified candidate. Both review axes cover all deltas. Runtime mutations:
  fourteen compiled kills, one audited equivalent survivor, zero compile failures and exact
  restoration. Same-workload paired p50/p95 is 10,699/11,927 µs versus 1,522/1,654 µs before
  activation; added cost and run spread are recorded, with no invented budget. Next: 151
  bilateral linked default plus explicit mirror-only placement, then 109b and fresh parent-19
  acceptance. Parent 19 remains open; nothing pushed.

- [19](issues/19-reconcile-mirrored-halves.md): resolved fresh full mirrored-half acceptance.
  Source is integrated locally at `21c5fd1a0`; post-integration repo/test/lint passes all 1,976
  native names, with 38 additions and no removals. Public both-author lifecycle/history, native/
  WASM parity, build/typecheck/browser and eight applicable compiled mutation kills are covered
  by independent reviews. Final paired native p50/p95 is 11,407/14,693 µs versus 1,528/2,273 µs
  before activation; the existing unpaired guard passes. User changes are preserved; no push.
