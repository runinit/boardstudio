# 25: Full review of the 20 shipped boards, Sofle first

Status: ready-for-agent
Type: task
Blocked by: [22](22-mirrored-half-integrated-verification.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## Request

User, 2026-10-10: the shipped keyboards were imported and recreated but never cleaned up.
They need a full review covering footprints, positioning and the rest, "lots of them need
tweaks outside of these changes". **Sofle is the focus** and goes first.

## What to do

Per board (20 shipped: 15 measured, 3 Sofle, REVIUNG41 archive, VIK review), starting with
Sofle v2, RGB and Choc:

- **Footprints:** each part's footprint, library source, pad and drill parameters, model
  assets, courtyard, side and `reversible` settings against the real part; import leftovers
  and placeholders (generated bay, reset, TRRS) that should be real components.
- **Positioning:** switch pitch and rotation against the measured source, controller,
  connector, diode, LED and encoder placement, mounting holes and their keep-outs, and the
  tight fits already recorded (closure holes vs controller bay on Lily58, KLOR, Sweep, Chocofi,
  Sofle RGB/Choc; switch-cutout gaps on Cantor, TOTEM, Sofle RGB; Sofle v2 gasket has no
  closure mounts).
- **Structure:** one measured half plus the native mirror; recorded exceptions only; outlines
  and gap repair (the KLOR 6.4 mm non-equivariant pocket); matrices and wiring.
- **Output:** PCB, plate and case results and fit findings; record unresolved hardware
  qualification as findings, since rebuilding does not establish physical fit.

Method: a planning pass producing a per-board checklist and severity list (read-only
research with the real fixtures), an AI review against the recorded outcomes and ADR rules,
then user review of the ranked list. No board fixes start before the user review; afterwards,
fixes proceed board by board with the implement/verify process. Each board's fixes are their
own ticket under this one.

## Acceptance

- [x] A written review for every shipped board with findings ranked by severity.
- [x] AI review completed against tickets 02, 06 and 30 and ADR-0010; corrected current linked-fit values and recorded scope limits.
- [x] User reviews the ranked list and supplies the rebuild direction before any board fix starts.
- [x] Rebuild direction recorded: configurable standard key spacing, native board-specific key assemblies, real component matches, native outlines/mount generation, regenerated closure clearances, and VIK deferred.
- [x] Phase 2 per-board fix tickets drafted with source evidence, ownership, hardware gates and regression/verification plans.
- [ ] Sofle variants fixed and verified first; user sign-off before the rest.
- [x] Decisions needing hardware knowledge are asked of the user, not guessed.

Keyboard [04](04-refresh-included-keyboards.md) is redone on the native model in [23](23-redo-keyboard-04-on-native-halves.md); this review feeds it and may supersede part of it.

## Outcome

Phase 1 research, AI review and human direction: [shipped boards review](../audit/shipped-boards-review.md). The audit now records the user's rebuild rules: repository-default key assembly spacing with a global configurable option; duplicate-position diodes for the current shipped data followed by BoardStudio-native, board-specific assemblies; real component footprints; native outline conversion and mounting-system/clearance generation for splits and unibodies; safe-only footprint flipping and a separate reversible-variant option; and VIK deferred. No recipes, fixtures, footprints or code were changed; no build/export or hardware fit check was run. Exact component matches, Choc cutout geometry, hardware tolerances and actual physical fit remain to be checked in the implementation tickets. The report also records the parts of the footprint/output review that were not performed. This lane stops before implementation; per-board tickets should begin with Sofle after the mirrored-half dependencies are ready.

### Verification (Sonnet 5.5 reviewer, 2026-10-10, `lane-boards-review`)

Read-only: no build, cargo, export or hardware check. Reproduced from `core/tests/fixtures/demo-projects-baseline.json`, the layout JSON, the REVIUNG41 archive and the tickets: all 20 part counts; the closure-to-bay and closure-to-switch values (KLOR 1.42 mm overlap, Cantor 0.031, TOTEM 0.048, Sofle Choc 0.109, and the rest, for both halves); the diode offset; the ticket 30 headings; the Sofle peripheral provenance; the 15 measured and three Sofle switch layouts (distance and rotation test). Wrong in the draft and corrected in the document: Sofle RGB 0.009 mm and the Sofle −3.30 mm bay overlap (both superseded by ticket 06, the baseline gives 1.066 mm and 0.10 mm on both halves); Sweep, Chocofi, TOTEM and Cantor are Choc boards at 18 x 17 mm pitch, not MX at 19.05 mm; key counts for Sweep, Chocofi and Cantor; the Lily58 and Sweep sources; stabiliser counts for Discipline, Mysterium and REVIUNG41; the ticket 30 heading difference applies to all seven measured splits. Decisions 1 (diodes) and 4 (pads) were already answered in ADR-0010 and ticket 31; reworded to the physical question that remains. A "Proposed tickets per board" list (Sofle first) is in the document. Not checked: pads, drills, courtyards, model assets, wiring and pin assignments, exported PCB, plate and case output, any real-hardware fit. Coverage gaps and the full claim table are in the document's Verification section. Status stays `ready-for-human`.

## Phase 2 ticket plan (Codex, Sol 6.1 max, 2026-10-10)

This pass writes tickets only. The user's decisions are preserved in the [phase 1 audit](../audit/shipped-boards-review.md#human-decisions-2026-10-10); no board has been rebuilt or physically qualified here. Ticket 25 remains open and `ready-for-agent` for its remaining board work. The phase 1 Outcome and Sonnet verification above are retained as historical evidence, not rerun or replaced.

### Ticket sequence

The 63 new files are numbered 42–104, after checking all 52 local/remote branch refs plus the boards lane and integration working-tree issue directories; 39–41 were already allocated on integration. Every board has three separate implementation concerns: native key assembly profile, real peripheral identities/placements, then native outline/mounting/clearances. Each is bounded to roughly one implementer hour after its source/hardware inputs and shared capability exist. New library components or shared geometry defects require their own tickets before dispatch rather than enlarging a board slice.

| Shared concern or gate | Ticket | Ownership/dependency |
| --- | --- | --- |
| Global pitch preference storage and control | [42](42-global-key-spacing-preference.md) | Web preferences only; does not rewrite projects |
| New-assembly creation consumes the preference | [43](43-apply-key-spacing-on-creation.md) | After 42; measured per-board spacing remains explicit |
| Native recipe matrix/profile materialisation | [44](44-native-recipe-assembly-profiles.md) | After 20, 31 and 39; explicit measured pitch, independent of 42/43; private recipe helper, no new Core mirror algorithm |
| Optional reversible-variant workflow decision | [45](45-reversible-variant-substitution-decision.md) | Non-blocking human decision; explicit per-board qualified selection works without a general UI feature |
| Sofle exact hardware/profile inputs | [46](46-sofle-hardware-inputs.md) | Agent source/library research first; human approval of proposed installed hardware, mounts/process/clearances remains open |
| Sofle verification packet and user sign-off | [56](56-sofle-sign-off.md) | After all three Sofle mechanical slices and a targeted mounted pass owned/recorded in 56; blocks every remaining board |
| Remaining split inputs | [57](57-measured-split-hardware-inputs.md) | After 56; per-board BOM/profile/clearance answers |
| Unibody inputs | [79](79-unibody-hardware-inputs.md) | After seven splits; measured recipe is REVIUNG41's editable target |
| Original REVIUNG41 archive preservation | [104](104-preserve-reviung41-original-archive.md) | After measured REVIUNG41; preserve source bytes and distinguish its historical gasket |

| Board (execution order) | Native assembly | Real components | Native mechanics | Hardware packet |
| --- | --- | --- | --- | --- |
| Sofle v2 | [47](47-sofle-v2-native-assembly.md) | [48](48-sofle-v2-real-components.md) | [49](49-sofle-v2-native-mechanics.md) | [46](46-sofle-hardware-inputs.md) |
| Sofle RGB | [50](50-sofle-rgb-native-assembly.md) | [51](51-sofle-rgb-real-components.md) | [52](52-sofle-rgb-native-mechanics.md) | [46](46-sofle-hardware-inputs.md) |
| Sofle Choc | [53](53-sofle-choc-native-assembly.md) | [54](54-sofle-choc-real-components.md) | [55](55-sofle-choc-native-mechanics.md) | [46](46-sofle-hardware-inputs.md) |
| Corne | [58](58-corne-native-assembly.md) | [59](59-corne-real-components.md) | [60](60-corne-native-mechanics.md) | [57](57-measured-split-hardware-inputs.md) |
| Lily58 | [61](61-lily58-native-assembly.md) | [62](62-lily58-real-components.md) | [63](63-lily58-native-mechanics.md) | [57](57-measured-split-hardware-inputs.md) |
| KLOR | [64](64-klor-native-assembly.md) | [65](65-klor-real-components.md) | [66](66-klor-native-mechanics.md) | [57](57-measured-split-hardware-inputs.md) |
| Ferris Sweep | [67](67-sweep-native-assembly.md) | [68](68-sweep-real-components.md) | [69](69-sweep-native-mechanics.md) | [57](57-measured-split-hardware-inputs.md) |
| Chocofi | [70](70-chocofi-native-assembly.md) | [71](71-chocofi-real-components.md) | [72](72-chocofi-native-mechanics.md) | [57](57-measured-split-hardware-inputs.md) |
| TOTEM | [73](73-totem-native-assembly.md) | [74](74-totem-real-components.md) | [75](75-totem-native-mechanics.md) | [57](57-measured-split-hardware-inputs.md) |
| Cantor | [76](76-cantor-native-assembly.md) | [77](77-cantor-real-components.md) | [78](78-cantor-native-mechanics.md) | [57](57-measured-split-hardware-inputs.md) |
| GH60 | [80](80-gh60-native-assembly.md) | [81](81-gh60-real-components.md) | [82](82-gh60-native-mechanics.md) | [79](79-unibody-hardware-inputs.md) |
| Discipline | [83](83-discipline-native-assembly.md) | [84](84-discipline-real-components.md) | [85](85-discipline-native-mechanics.md) | [79](79-unibody-hardware-inputs.md) |
| Mysterium | [86](86-mysterium-native-assembly.md) | [87](87-mysterium-real-components.md) | [88](88-mysterium-native-mechanics.md) | [79](79-unibody-hardware-inputs.md) |
| Voyager97 (103 keys) | [89](89-voyager97-native-assembly.md) | [90](90-voyager97-real-components.md) | [91](91-voyager97-native-mechanics.md) | [79](79-unibody-hardware-inputs.md) |
| Voyager104 | [92](92-voyager104-native-assembly.md) | [93](93-voyager104-real-components.md) | [94](94-voyager104-native-mechanics.md) | [79](79-unibody-hardware-inputs.md) |
| Plaid | [95](95-plaid-native-assembly.md) | [96](96-plaid-real-components.md) | [97](97-plaid-native-mechanics.md) | [79](79-unibody-hardware-inputs.md) |
| Lumberjack | [98](98-lumberjack-native-assembly.md) | [99](99-lumberjack-real-components.md) | [100](100-lumberjack-native-mechanics.md) | [79](79-unibody-hardware-inputs.md) |
| REVIUNG41 measured | [101](101-reviung41-native-assembly.md) | [102](102-reviung41-real-components.md) | [103](103-reviung41-native-mechanics.md) | [79](79-unibody-hardware-inputs.md) |

Within a board the three tickets are sequential. The next board waits for the previous board's mechanical ticket, so the shared recipe files have one writer. Corne and all later boards also wait for 56; the user must explicitly approve the concrete Sofle results. Shared preference/helper work and hardware answers can be prepared before that gate, but no later-board implementation starts early. Packets 46/57/79 cannot be marked resolved while required rows remain unknown; split a packet before claim if separately arriving answers should unblock only one board.

VIK is deferred to the backlog until a target host is selected. This plan does not create a host-qualification/rebuild ticket, modify its review project, or ask for a host choice now. No `map.md` or backlog file was edited by this lane; the orchestrator records the deferred item when maintaining tracking.

### Coordination with 20, 23 and the shared mirror tickets

- [20](20-recipes-build-one-half.md) keeps ownership of one-half creation, native correspondence/numbering, recipe mirror removal and KLOR's reflective outline/removal of the 6.5 mm allowance. New board tickets consume that verified result and do not recreate a mirror implementation or a loose outline tolerance.
- [31](31-companion-placement-modes.md), [32](32-mirrored-half-labels.md) and integration tickets 39–41 keep ownership of companion/stabiliser semantics, fallback, mode editing and its inspector. Their newer ADR amendments/interfaces must be read at execution. Tickets 39–41 exist on integration but are absent on this lane, so no broken file links were added. Native board profiles select the approved modes; they do not change the rules.
- [23](23-redo-keyboard-04-on-native-halves.md) retains re-inventory of all 20 outputs, mounted-browser/live editing, Standards and Spec reviews, the full integration check and closure of [04](04-refresh-included-keyboards.md). Before 56 sign-off, a fresh Sol verifier runs the targeted three-variant Sofle mounted pass owned and recorded in 56, independently of final 23. Per-board Outcomes and 56/104 supply concrete assembly/component/mechanical/archive evidence for 23 to reuse when current. None of these acceptance obligations is waived or silently superseded by this plan. Before dispatching final 23, the orchestrator should update its tracking dependencies to the finished board set and Sofle sign-off (this lane is not authorised to amend 23 or the map). Parent 25 remains open until its own board acceptance and unresolved hardware gates are handled.

### Execution and file ownership contract

The orchestrator assigns a concrete worktree/branch and a single writer before each dispatch. Implementers are Luna 6 high; independent Sol 6.1 xhigh sessions verify, mutation-check the important assertions and alone set build tickets `resolved`. Shared `physical.rs`, `recipes.rs`, `mechanical.rs`, `mod.rs` and `included_keyboards.rs` edits are sequential; board tickets own only their named branch/profile/assertions. Ticket 44 owns shared helper changes first. No board implementer expands a public/member API or changes a shared generator/geometry/mirror algorithm under a local board ticket.

The designated golden owner from `parallel-lanes.md` (B, then D) remains the only writer of `core/tests/fixtures/demo-projects-baseline.json` and the boundary transcript. Before dispatch, the orchestrator arranges that owner's generated baseline/output handoff for each behavior-changing slice; board agents never hand-edit goldens or build assets. Record intentional per-board identity/pose/definition changes. Required snapshot-dependent tests/build remain blocked until the owner has regenerated the affected data; no ticket may claim a clean gate while relying on an obsolete baseline. Output handoffs and checks run sequentially in the foreground using the global command slot.

For each bug fix, first reproduce a public-boundary regression that fails for the expected old profile/component/geometry, then verify the fix. Use exact source correspondence and resolved envelopes rather than a golden-only assertion. A verifier disables one relevant site at a time and confirms failure before restoring. Native/example changes need `repo lint test typecheck`; changed generated fixtures add `build`. Mounted presentation changes need `browser` as well. Per-board mechanics require actual targeted PCB/plate/case output and numerical findings; the final mounted-browser/Standards/Spec/full check stays with 23/integration, once per batch. No `release` is authorised. Unobserved physical fit, electrical qualification and manufacturing tolerance stay explicit gates.

The human has selected BoardStudio adaptations, not upstream hardware reproduction. Every measured key position, tilt and width stays fixed for each Sofle variant; assemblies, components and mounts/cases may adapt through sourced proposals and explicit open design gates. The user will specify target installed hardware/BOM; agents do not choose it. Screws/spacers may be added, and gasket, plate and tolerance use BoardStudio’s existing case-generation tool. Measured source JSON/hashes are evidence, not files to silently edit for a standard-spacing change. Default pitch and keycap edge gap are separate; board overrides retain 19.05 mm Sofle/MX/unibody or 18 × 17 mm measured Choc relationships. All current shipped diode profiles select duplicate position until their approved board-specific evidence changes it. Independent headings use 180-r, switches -r, and Core mirroring preserves side/parameters; any qualified reversible variant is selected separately.

### Hardware questions and current source limits

[46](46-sofle-hardware-inputs.md) assigns source revision/component/LED/datum facts and existing library qualification to agent research. The target is a BoardStudio adaptation with every measured Sofle key position, tilt and width fixed. The user will specify target installed hardware; agents qualify matching library entries/assemblies and inventory supported case-generator mount/gasket/plate/tolerance options. Screws/spacers may be added. Only genuine remaining design choices become user questions; do not demand unrelated upstream manufacturing specifications or invent independent geometry. [57](57-measured-split-hardware-inputs.md) and [79](79-unibody-hardware-inputs.md) carry the same per-board gates for later boards. Missing catalogue parts become separately scoped onboarding tickets after identity is known. No target component, mirror-safe footprint, mounting type or tolerance was guessed.

Additional source inspection found `core/src/mechanical.rs::builtin_profile` already distinguishes manufacturer-sourced MX, PG1350 Choc v1 and PG1353 Choc v2 profiles (all currently carry 14 mm nominal square cutouts, with different plate/profile datums). The phase 1 “Choc cutout size not found” records that review's scope, not proof that the current repository has no Choc profile. New mechanics tickets must select and validate the actual fit profile, then recalculate clearances; they must not invent a different size solely from the audit. The v2 baseline fixture is named `sofle` (its new board profile/ticket slug is `sofle-v2`).

The lane's bound GitNexus `query` and `context` calls to `boardstudio-lane-boards-review` returned repository-not-found; paginated `list_repos` completed with 10 indexes and none for this lane. Code evidence therefore uses `rg` and direct reads of the current recipe, matrix setup, typed outline and mechanical-profile source. No other checkout's graph was substituted and no refresh was run. This is a graph coverage limitation, not a clean impact result. Implementation must obtain fresh bound impact evidence before editing symbols.

### Phase 2 Outcome

Drafted tickets 42–104 and the dependency/ownership index above. Read the missing agent-workflow document from the main checkout, the lane’s verification/index/tracker documents plus their current main-checkout supplements, the lane instructions, ticket 25/audit/hardware-decision commits, ADR-0010, tickets 04/20/22/23/31, generator-parameters 02/06 fit/dependency sections, integration ticket 39, and the relevant current recipe/native source. No recipes, fixtures, catalogue/footprint code, archive bytes, ADRs, CONTEXT.md, map, generated goldens or app state were changed. Phase 1 verification remains untouched.

Validation pending: the orchestrator's explicit global command slot is required even for `python3 scripts/check.py repo`; it has not been granted, so that check and the final docs commit are held. No build/cargo/browser/full check has run in this planning pass. After staging 64 explicit issue-document paths, bound `detect_changes(scope: all, repo: boardstudio-lane-boards-review, worktree: /home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-boards-review)` returned repository-not-found, like query/context. No changed-symbol/process/risk result was produced or claimed. The graph gate is unavailable; no refresh or other-checkout substitution was attempted. No final docs commit or parent resolution has occurred yet.

### Independent plan-review disposition (Codex, Sol 6.1 medium coordinator, 2026-10-10)

The root coordinator supplied its independent Sol 6.1 xhigh review and assigned this lane the minimum corrections: remove 43 from 44's blockers because native recipes use explicit measured pitch; separate agent source/library research from human installed-hardware/mount/process/numerical approvals in 46/57/79; assign the pre-sign-off Sofle mounted pass to 56 without waiting for final 23; and add the existing Library Workspace settings mount/import plus mounted assertions to 42's scope. These corrections are drafted and staged. The user's BoardStudio-adaptation direction and fixed Sofle key positions/tilts/widths are explicit. No source or generated data changed.

Independent disposition of these corrections, the docs `repo` check and final commit remain pending under the root coordinator's gates. The root has assigned the global command slot to ticket 37 verification; no check, build, browser or index operation ran here. Ticket 42 is only a proposed independent implementation frontier after root scheduling; ticket 19 has exclusive execution priority after the batch. Ticket 46 read-only Sofle source/library/adaptation research is active, with findings held in the research response until a document output is assigned. Parent 25 remains open.

Human clarification after review: installed target hardware/BOM will be supplied by the user; screws/spacers may be added; gasket, plate and tolerance use the existing BoardStudio case generator. Updated 46 and Sofle mechanics 49/52/55 to preserve these settled directions and keep unsupported capabilities as separately scoped follow-ups. Research may inventory capabilities but cannot select a BOM. These are planning edits only.

### Final documentation gate (root coordinator, 2026-10-10)

The independent Sol 6.1 xhigh reviewer rechecked the four corrections and closed all four P2 findings in the staged drafts. The root coordinator then ran `python3 scripts/check.py repo`: passed in 0.1 s, with 244 Markdown files checked. The final bound `detect_changes` attempt still returned repository-not-found; no graph risk verdict is claimed. The reviewed 64-document plan is committed for integration, with no board implementation or hardware qualification implied. Parent 25 remains open; ticket 19's exclusive batch and the relevant board dependencies still gate implementation.
