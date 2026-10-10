# Shipped boards review (phase 1)

Read-only review of the 20 shipped projects in the requested order. Source of project data is `core/tests/fixtures/demo-projects-baseline.json`; measured inputs are pinned in `content/layouts/keyboard-layouts.json` and `content/layouts/sofle-layouts.json`. Recipe behavior is in `core/examples/demo_projects/{recipes,physical,mechanical,review}.rs`. The archive is `content/archives/reviung41-original.boardstudio`; the VIK inputs are `content/layouts/module-review.json` and the catalogue sources.

The fixtures describe BoardStudio’s adapted examples, not upstream board files: the recipes preserve measured switch locations, then add BoardStudio library switches, diodes, nice!nano placements/bays and generated outlines. Upstream routing and most peripherals are not reproduced (`content/demos/README.md`; `recipes.rs`). The recorded fixture source hashes match the layout metadata. I independently compared every stored switch pose in the baseline against the corresponding source JSON position and recipe transform: all 15 measured keyboards and all three Sofle variants matched (zero pose differences). This is a read-only calculation against saved data, not a build or physical check.

## Cross-board themes

- All 10 paired split examples still contain two recipe-authored halves. ADR-0008/0010 call for one authored half with its second half maintained as a Core reflection; every pair item stays linked unless that individual item has an explicit local/omitted exception. Ticket 20 is still `ready-for-agent`; it specifically removes manual mirror arithmetic and makes KLOR’s outline match. Treat current parts as measurement-derived, not yet as proof that the recipe satisfies the final pair contract.
- Matrix switches use `ceoloide/switch_mx` or `ceoloide/switch_choc_v1_v2`; all have `reversible:false`, generator `side:B`, and part `side:front`. Their mirrored tilt is `-r` and position mirrors on X only. Diodes use `ceoloide/diode_tht_sod123`, with `reversible:false`, generator `side:B`, part `side:back`. RGB variants add back-side SK6812 LEDs. The three companion modes are literal mirror, duplicate position, and mirror with a tweak. Other generator definitions generally omit `reversible` and a generator `side`; absence is recorded as “absent”, not interpreted as `false`. Mirroring must not rewrite either parameter or part side (ADR-0010).
- Split recipes generate nice!nano controller, reset and TRRS parts and associated bay/TRRS openings; they are adaptation placeholders, not verified source-board component placements. Sofle also has generated encoder and display parts. Unibodies get a generated nice!nano and reset switch, without TRRS. Generated outlines are live recipe outputs. Unibody examples have no case bodies or authored closure mounts in the baseline. No fixture records explicit keep-out regions; the listed clearance values are post-hoc geometry measurements, not keep-out constraints.
- Seven measured splits, Sofle RGB and Choc store four closure mounts per half. Sofle v2 is gasket construction and has none. These are Core-proposed closure features, not upstream mounting-hole measurements. The parts use `ceoloide/mounting_hole_npth`, front side, `hole_drill:6.6` / `hole_size:6.6`, with no explicit `reversible` parameter; their 6.6 mm hole and the 14 mm switch cutout are assumptions for clearance calculations.
- Recorded exports succeeded, but that does not establish PCB, plate, case, electrical, connector, or hardware fit. The saved keyboard fixtures carry project data rather than exported PCB/plate/case results; their paired mechanical configuration does not itself qualify generated output. VIK alone records a simple plate case body. No build/export was run for this review.

## Ten highest-priority findings

Ranked by likely impact. Severity classes are **1 — build/export blocker**, **2 — physically wrong or unresolved fit**, and **3 — cosmetic/tidy-up**. No class-1 blocker is recorded by the cited Outcomes: ticket 02 reports every split half exported. This is historical evidence, not a fresh export.

1. **[2] KLOR closure/bay overlap persists on both halves by 1.42 mm.** Ticket 06 says linked closure positions carry the overlap to both halves; Core does not report a half’s own closure mount against a Case opening. [Ticket 06](../../generator-parameters/issues/06-paired-half-closure-proposals.md).
2. **[2] Sofle RGB and Choc closure holes overlap the linked controller bay.** Ticket 06 reports 0.10 mm clearance on the left and −3.30 mm on the right. [Ticket 06](../../generator-parameters/issues/06-paired-half-closure-proposals.md).
3. **[3] Ticket 02/06 preserve superseded bay-overlap findings for Lily58, Sweep and Chocofi.** Ticket 02 records overlap on then-local right placements (1.35, 1.10 and 0.54 mm); ticket 06’s later linked-proposal table gives 25.74, 27.68 and 35.48 mm clearance respectively. Keep the history visible, but use ticket 06’s later values for current review. [Tickets 02 and 06](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).
4. **[2] Tight closure-to-switch-cutout clearances:** Sofle RGB 0.009 mm (right), Cantor 0.031 mm (both after linkage), TOTEM 0.048 mm (both), Sofle Choc 0.109 mm (both), KLOR 0.295 mm (both), Sweep 0.495 mm (both). These are geometry calculations with assumed cutout dimensions, not measured manufacturing clearance. [Tickets 02 and 06](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).
5. **[2] KLOR’s generated outline is not reflective:** up to 6.38 mm boundary distance at right x 316–334, y −61 to −69. Ticket 06 attributes this to gap repair filling a 19.36 mm concave pocket on one side only, not to the measured keys, bay, reset, TRRS or diodes. Ticket 20 requires a native mirrored half and removes the 6.5 mm allowance. [Ticket 06 Outline mirror](../../generator-parameters/issues/06-paired-half-closure-proposals.md#outline-mirror), [ticket 20](../issues/20-recipes-build-one-half.md).
6. **[2] Sofle v2 closure/mount decision remains open.** It has a gasket and zero closure mounts; keeping gasket means none, changing to tray means mounts must be proposed. Its gasket geometry was regenerated when the pair construction changed and needs fit review. [Ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md), [ticket 04 Outcome](../issues/04-refresh-included-keyboards.md).
7. **[2] The current diode mirror rule changes diode position by 14.8 mm from every shipped right-half diode.** Shipped recipes keep the same switch-local `(+7.40, -1.50)` offset; ADR-0010 currently specifies literal mirrored position. Three modes exist (literal mirror, duplicate position, mirror plus tweak). The intended mode is a user decision; ticket 29 recommends no hardware guess. [ADR-0010 companion amendment](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md), [ticket 29 Verification](../issues/29-mirror-switch-and-companion-headings.md#verification).
8. **[2] Independent-part heading mismatch is recorded for Corne and all three Sofles.** Ticket 30 reports right-half MCU/reset/TRRS on Corne at 0→180, and Sofle v2/RGB/Choc right MCU, encoder, display, TRRS, reset and seven round mounting holes at the listed headings. RGB/Choc additionally have LED hand placements. Recipe headings still use `-r`/the authored angle; ADR-0010 requires `180-r` over 360° for independent parts. [Ticket 30 Outcome](../issues/30-independent-parts-use-the-full-heading-rule.md).
9. **[2] Correct heading alone does not geometrically mirror off-axis pads.** ADR-0010’s ticket-27 verification says MCU, TRRS, display and encoder pads off their heading axis land with local `y` reversed when a part is only rotated; it proposes a finding or a footprint-aware solution for user decision. [ADR-0010 note and ticket 27 Verification](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md).
10. **[3] All generated peripheral placements need component/fit qualification.** Source metadata expressly says peripherals/routing are not reproduced; bays, reset/TRRS placements, Sofle encoder/display offsets, and controller positions are recipe values. Decide whether to keep these demonstration placeholders, replace them with verified part identities/poses, or leave the example explicitly schematic. [README](../../../../content/demos/README.md), [recipes](../../../../core/examples/demo_projects/recipes.rs).

## Decisions for the user

1. **Diodes on mirrored halves:** (a) retain the ADR literal mirror, moving each diode to the other switch side; (b) add a duplicate-position companion mode matching shipped data; or (c) literal mirror with a per-item tweak. **Recommendation:** choose only after confirming diode-to-switch placement against a target PCB; preserve the three-mode model and do not silently encode one mode as universal. [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md).
2. **Sofle v2 construction:** (a) keep the gasket, with no closure mounts; or (b) move to tray and review generated closure positions. **Recommendation:** decide the mount before changing holes; the current source contains no evidence to choose. [Ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).
3. **Peripheral placeholders:** (a) retain as illustrative generated parts/openings; (b) qualify and replace with exact target components and measured placements; or (c) remove peripherals and leave the example layout-only. **Recommendation:** (b) only with the exact hardware/board revision and fit evidence; otherwise (a) must remain clearly documented as a demo. No hardware facts are inferred here.
4. **Independent-part mirror geometry:** (a) accept pose-only `180-r` and surface pad-asymmetry findings; or (b) define footprint-aware transforms/reflections for the affected footprints. **Recommendation:** make this a separate decision before treating a native mirror as fabrication-ready. [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-the-heading-rule-in-practice-2026-10-10).
5. **Clearance findings:** decide acceptable minimum clearance and whether to move closure mounts or revise bay/outline geometry. **Recommendation:** require target hardware and fabrication/assembly tolerances before accepting sub-millimetre values.

## Part-list and footprint key

The baseline stores `definitionId`, generator source/parameters, part `side`, pose and model assets; generated definitions do not carry a separate KiCad footprint-name string. Therefore the “footprint” identifiers below are the exact BoardStudio library source IDs, not a claim that their fabrication footprint/model is qualified. Generator code is under `footprints/src/generators/ceoloide/`; imported Marbastlib footprint files and manifest are under `catalogue/parts/`. `reversible` and generator-side values below are the stored recipe parameters; a parameter marked absent is not explicit in the definition. All references, counts and positions come from the expected baseline fixture.

| Recipe/profile | Part list and library footprint source | Stored side / settings |
|---|---|---|
| Measured MX matrix | N `ceoloide/switch_mx`; N `ceoloide/diode_tht_sod123`; where present 2 `ceoloide/mcu_nice_nano`, 2 `ceoloide/reset_switch_tht_top`, 2 `ceoloide/trrs_pj320a`; closure features are generated `case-closure/*` assemblies | Switch: front, `side:B`, `reversible:false`, hotswap; diode: back, `side:B`, `reversible:false`, no THT; peripherals front, `include_extra_pins:true` on MCU, other mirror/side/reversible params absent. Closure hole source uses 6.6 mm normalized hole/drill settings. |
| Sofle v2 | 58 `switch_mx`, 58 `diode_tht_sod123`; two each nice!nano, EC11/EC12 encoder, SSD1306 display, PJ-320A TRRS, reset; 14 NPTH measured TH1–TH7 per half | Switch/diode as above; all peripherals front, holes front; no RGB LED. `reversible` absent on peripheral definitions. Gasket, no closure assemblies. |
| Sofle RGB | 58 `switch_mx`, 58 matrix diodes, 58 matrix SK6812 LEDs, 14 measured SK6812 LEDs D31–D37 per half, plus Sofle v2 peripheral/hole profile and eight closure parts | Matrix LED back, `side:B`, `reversible:false`, `reverse_mount:true`; measured D31–D37 back, `side:B`, `reversible` absent, `reverse_mount:false`. Switch/diode as above. |
| Sofle Choc | 58 `switch_choc_v1_v2`, 58 matrix diodes, 58 matrix SK6812 LEDs, plus Sofle v2 peripheral/hole profile and eight closure parts | Switch front, `side:B`, `reversible:false`, hotswap, Choc V1 support, V2 off; matrix LED back with `reverse_mount:true`; diode back. No separate measured D31–D37 set in fixture. |
| Measured unibody common | N `switch_mx`, N `diode_tht_sod123`, one nice!nano and one reset switch; optional Marbastlib MX stabilisers | Switch front, generator `side:B`, `reversible:false`, hotswap; diode back, `side:B`, `reversible:false`; controller/reset front. Stabiliser side front; reversible absent. No authored mounting-hole or closure parts in these fixtures. |
| VIK module review | 13 solder MX switch source `ceoloide/switch_mx`; imported `thqwgd001:rotation-reversible`, `:c-2pin-reversible`, `:c-4pin-reversible`; source horizontal VIK host connectors; embedded Haptic DRV2605L components; mounted VIK splitter, haptic and EC11 modules | Switches/connectors front except J_VIK2 back; haptic passives/components retain their embedded front/back faces; imported encoder definitions have no instance `reversible` parameter (reversible identity is in the definition/source). Not a shipped keyboard. See `review.rs`, `catalogue/modules/imported-modules.json`, `catalogue/parts/imported-parts.json`. |

## Board-by-board record

The profile key above gives each part’s library source and stored side/settings. Counts are fixture part counts, including the generated closure assemblies where present; switches/diodes are matrix instances, not distinct definitions. Switch positions were checked against source JSON as described above. The named controller/reset/TRRS and bay/opening are generated by the recipe and should be treated as placeholders pending the user decisions above.

### 1. Sofle v2

- **Source/positions:** 58 measured switches (4×6 keys + 1×5 thumbs per half), 19.05 mm matrix pitch; all 58 switch poses matched `content/layouts/sofle-layouts.json` `layouts.v2` through `recipes.rs`. Source: `Sofle_v2/PCB/SofleKeyboard.kicad_pcb`, revision `fb294f7c`, hash in the layout JSON. 140 parts, 70 per board.
- **Parts:** v2 profile: MX switches/diodes, nice!nano, EC11 encoder, SSD1306 display, TRRS, reset, and TH1–TH7 NPTH per half. `reversible:false` on switches/diodes; absent on peripherals/holes. No closure mounts.
- **Mounts/keep-outs/placeholders:** 7 authored through-holes each half; no recorded closure holes. Two linked bay openings and two linked TRRS openings; reset has no Case opening. No measured Case window for encoder/display. Shared gasket construction replaces the old one-half legacy gasket and was regenerated.
- **Findings:** gasket fit/construction unresolved; the three boards’ independent parts are in ticket 30’s heading-disagreement set. The independent pads on MCU/TRRS/display/encoder also need the ticket-27 decision. [Ticket 04 Outcome](../issues/04-refresh-included-keyboards.md), [ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md), [ticket 30](../issues/30-independent-parts-use-the-full-heading-rule.md).

### 2. Sofle RGB

- **Source/positions:** 58 switches, 4×6 + 1×5 per half, 19.05 mm matrix pitch; source-matched. Source hash/revision as above, `layouts.rgb`. 220 parts, 110 per board.
- **Parts:** 58 MX switches, 58 diodes, 58 matrix SK6812 LEDs, 7 additional measured LEDs per half, plus v2 peripherals/THs and eight closure parts. Matrix LEDs store back/`side:B`/`reverse_mount:true`; measured LEDs back/`side:B`/`reverse_mount:false`. See RGB profile.
- **Mounts/keep-outs/placeholders:** 7 authored holes per half; 4 closure holes each half. Closure-to-bay gap 0.10 mm left and −3.30 mm right (overlap). Closure-to-switch gap 1.066 mm left, 0.009 mm right. Controller/TRRS bay values are generated placeholders.
- **Findings:** heading mismatches (right U1, SW25, J3, J2, RSW1, TH1–TH7; LED headings as enumerated in ticket 30); right bay overlap and 0.009 mm switch gap; pad geometry/user decision; diode companion-mode decision. [Tickets 02, 06 and 30](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).

### 3. Sofle Choc

- **Source/positions:** 58 switches, 4×6 + 1×5 per half; source-matched to `layouts.choc`. 206 parts, 103 per board.
- **Parts:** 58 Choc switches, 58 diodes, 58 SK6812 matrix LEDs, plus two each nice!nano/encoder/display/TRRS/reset, 14 authored TH holes, eight closure parts. Settings in Sofle Choc profile.
- **Mounts/keep-outs/placeholders:** 7 authored holes per half; 4 closure holes per half. Bay gap 0.10 mm on left and −3.30 mm on right; switch gap 0.109 mm left and 1.028 mm right.
- **Findings:** right bay overlap; tight left switch gap; ticket 30 heading mismatches for independent components; ticket 29 diode mode differs from shipped same-side placement. No separate measured D31–D37 LED bank. [Tickets 02, 06 and 30](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).

### 4. Corne

- **Source/positions:** 21 measured keys (3×6 + 3 thumbs), 19.05 mm pitch; all 42 switch poses matched. Source `foostan/crkbd`, revision `1f56ad19`, `corne-cherry/pcb/corne-cherry.kicad_pcb`. 98 parts, 49 each half.
- **Parts:** 42 MX switches + 42 back diodes; two each MCU/reset/TRRS; eight generated closure-hole parts. Measured MX profile.
- **Mounts/placeholders/findings:** no authored TH holes; four closure holes each half. Closure switch gaps 0.664/1.700 mm, no hole within 0.5 mm of bay. Bay/reset/TRRS generated. Ticket 30 reports right U1/RST/TRRS heading 0→180. Shipped diode positions use same-side placement, unlike current literal-mirror rule. [Tickets 02, 29, 30](../issues/30-independent-parts-use-the-full-heading-rule.md).

### 5. Lily58

- **Source/positions:** 29 measured keys (4×6 + 5 thumbs), 19.05 mm pitch; all 58 switch poses matched. Source `josefadamcik/lily58`, pinned measurement path/hash in `keyboard-layouts.json`. 130 parts, 65 per half.
- **Parts:** 58 MX switches + 58 back diodes; two each MCU/reset/TRRS; eight generated closure parts. Measured MX profile. Recipe input is right-authored; `recipes.rs` records this special source frame.
- **Mounts/placeholders/findings:** no authored TH parts; four closure holes each half. Ticket 02 records the earlier right closure/bay overlap of 1.35 mm; ticket 06’s later linked position is 25.74 mm clear. Switch gap is 1.118 mm on both halves after linkage. Generated bay/reset/TRRS. [Tickets 02 and 06](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).

### 6. KLOR

- **Source/positions:** 21 keys, 3×6 + 4 thumbs, 19.05 mm pitch; all 42 switch poses matched, including rotated columns/thumbs. Source repository/revision/path/hash in `keyboard-layouts.json`. 98 parts, 49 each half; right-authored source frame.
- **Parts:** 42 MX switches + 42 back diodes; two each MCU/reset/TRRS; eight closure parts. Measured MX profile.
- **Mounts/placeholders/findings:** no authored TH holes; four closure holes per half. Closure/bay overlap is 1.42 mm on both; switch gap 0.295 mm on both after linkage. Outline boundary differs by as much as 6.38 mm because the 20 mm pocket-gap repair fills only one concavity. Recipe fixes cannot address that; ticket 20 requires creating the mirrored half from one source. [Ticket 06 Outline mirror](../../generator-parameters/issues/06-paired-half-closure-proposals.md#outline-mirror), [ticket 20](../issues/20-recipes-build-one-half.md).

### 7. Ferris Sweep

- **Source/positions:** 17 keys (3×6 + 5 thumbs), 19.05 mm pitch; all 34 switch poses matched. Source `qmk/…` pinned by path/hash in `keyboard-layouts.json`. 82 parts, 41 each half.
- **Parts:** 34 MX switches + 34 back diodes; two each MCU/reset/TRRS; eight closure parts. Measured MX profile.
- **Mounts/placeholders/findings:** no authored TH holes; four closure holes each half. Ticket 02 records the earlier right closure/bay overlap of 1.10 mm; ticket 06’s later linked position is 27.68 mm clear. Switch gap is 0.495 mm on both halves after linkage. Bay/reset/TRRS generated. [Tickets 02 and 06](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).

### 8. Chocofi

- **Source/positions:** 18 keys (3×6), 19.05 mm pitch; all 36 switch poses matched. Pinned source path/revision/hash in `keyboard-layouts.json`. 86 parts, 43 each half.
- **Parts:** 36 MX switches + 36 back diodes; two each MCU/reset/TRRS; eight closure parts. Measured MX profile.
- **Mounts/placeholders/findings:** no authored TH holes; four closure holes each half. Ticket 02 records the earlier right closure/bay overlap of 0.54 mm; ticket 06’s later linked position is 35.48 mm clear. Switch gap is 1.381 mm on both halves after linkage. Generated bay/reset/TRRS. [Tickets 02 and 06](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).

### 9. TOTEM

- **Source/positions:** 19 keys (15 main, one outer, 3 thumbs), 19.05 mm pitch; all 38 switch poses matched. Pinned source path/revision/hash in `keyboard-layouts.json`. 90 parts, 45 each half.
- **Parts:** 38 MX switches + 38 back diodes; two each MCU/reset/TRRS; eight closure parts. Measured MX profile.
- **Mounts/placeholders/findings:** no authored TH holes; four closure holes each half. Switch gap is 0.048 mm on both halves after linkage, with no closure within 0.5 mm of bay. Generated bay/reset/TRRS. Diode side is a pending companion-mode decision. [Tickets 02 and 29](../issues/29-mirror-switch-and-companion-headings.md).

### 10. Cantor

- **Source/positions:** 21 keys (3×10 + 3 thumbs), 19.05 mm pitch; all 42 switch poses matched. Pinned source path/revision/hash in `keyboard-layouts.json`. 98 parts, 49 each half.
- **Parts:** 42 MX switches + 42 back diodes; two each MCU/reset/TRRS; eight closure parts. Measured MX profile.
- **Mounts/placeholders/findings:** no authored TH holes; four closure holes each half. Switch gap is 0.031 mm on both halves after linkage; no closure within 0.5 mm of bay. Generated bay/reset/TRRS. The 0.031 mm side is a fit concern, not a hardware verdict. [Ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).

### 11. REVIUNG41 measured

- **Source/positions:** 41 measured switches, 19.05 mm pitch, split into left/right key clusters and thumbs on a single unibody main board; all 41 poses matched. Source `gtips/reviung`, revision `110154f5`, `reviung41/pcb/ver1.3/reviung41.kicad_pcb`. 85 parts.
- **Parts:** 41 MX switches + 41 back diodes, one nice!nano and reset. Measured unibody profile; no TRRS.
- **Mounts/placeholders/findings:** no authored mounting holes, closure holes, mechanical setup, or case body in this measured fixture. Controller bay and reset are generated examples, not sourced peripheral placements. [Layout metadata](../../../../content/layouts/keyboard-layouts.json), [fixture baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 12. GH60

- **Source/positions:** 61 measured keys; all 61 matched at 19.05 mm matrix pitch with measured widths retained. Pinned upstream revision/path/hash in `keyboard-layouts.json`. 129 parts.
- **Parts:** 61 MX switches + 61 back diodes; one nice!nano/reset; one 6.25u and four 2u Marbastlib stabilisers. Stabiliser source IDs `kicad:marbastlib/STAB_MX_6.25u` and `.../STAB_MX_2u`, front side.
- **Mounts/placeholders/findings:** no authored TH or closure parts/case body; generated controller/reset, no TRRS. Stabiliser placement follows switch width/reference mapping in `physical.rs`; no hardware validation recorded. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 13. Discipline

- **Source/positions:** 68 measured keys; all 68 matched at 19.05 mm base pitch. Pinned source revision/path/hash in `keyboard-layouts.json`. 142 parts.
- **Parts:** 68 MX switches + 68 back diodes; one nice!nano/reset; four 2u Marbastlib stabilisers (front). No TRRS.
- **Mounts/placeholders/findings:** no authored TH/closure holes or case body. Generated controller/reset placeholders only. No physical-fit findings are recorded for the unibody recipe; no fit is established. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 14. Mysterium

- **Source/positions:** 87 measured keys across function, keys, navigation, arrows and numpad matrices; all 87 matched. Pinned source path/revision/hash in `keyboard-layouts.json`; 19.05 mm base pitch. 181 parts.
- **Parts:** 87 MX switches + 87 back diodes; one nice!nano/reset; five 2u and one 6.25u Marbastlib stabilisers, front.
- **Mounts/placeholders/findings:** no authored TH/closure holes or case body; generated controller/reset, no TRRS. No recorded physical fit qualification. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 15. Voyager97

- **Source/positions:** 103 measured keys across function/navigation/arrows/numpad/keys matrices; all 103 matched, 19.05 mm base pitch. Pinned path/revision/hash in `keyboard-layouts.json`. 215 parts.
- **Parts:** 103 MX switches + 103 back diodes; one nice!nano/reset; six 2u and one 6.25u Marbastlib stabilisers, front.
- **Mounts/placeholders/findings:** no authored TH/closure holes or case body; generated controller/reset only, no TRRS. No physical qualification recorded. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 16. Voyager104

- **Source/positions:** 104 measured ANSI keys across function/navigation/arrows/numpad/keys matrices; all 104 matched, 19.05 mm base pitch. Pinned path/revision/hash in `keyboard-layouts.json`. 218 parts.
- **Parts:** 104 MX switches + 104 back diodes; one nice!nano/reset; seven 2u and one 6.25u Marbastlib stabilisers, front.
- **Mounts/placeholders/findings:** no authored TH/closure holes or case body; generated controller/reset only, no TRRS. No physical qualification recorded. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 17. Plaid

- **Source/positions:** 48 measured keys in 4×12 matrix, all matched at 19.05 mm pitch. Pinned path/revision/hash in `keyboard-layouts.json`. 98 parts.
- **Parts:** 48 MX switches + 48 back diodes; one nice!nano/reset. No stabilisers in the fixture.
- **Mounts/placeholders/findings:** no authored TH/closure holes or case body; generated controller/reset, no TRRS. No physical qualification recorded. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 18. Lumberjack

- **Source/positions:** 60 measured keys split into left/right 5×6 matrix clusters; all matched at 19.05 mm pitch. Pinned path/revision/hash in `keyboard-layouts.json`. 122 parts.
- **Parts:** 60 MX switches + 60 back diodes; one nice!nano/reset. No stabilisers recorded.
- **Mounts/placeholders/findings:** no authored TH/closure holes or case body; generated controller/reset, no TRRS. No physical qualification recorded. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 19. REVIUNG41 original archive

- **Source/positions/parts:** the packed archive contains project JSON `REVIUNG41`, revision 3, 85 parts on one main board. Its source URL/revision/hash match the measured layout entry (`gtips/reviung`, `110154f5`, `reviung41/pcb/ver1.3/reviung41.kicad_pcb`); the 41 switch positions match the same 41-key measured input. This inventory was read from the ZIP member in memory; the archive was not modified or extracted.
- **Footprints:** 41 `ceoloide/switch_mx` (front, generator `side:B`, `reversible:false`, hotswap); 41 `ceoloide/diode_tht_sod123` (back, `side:B`, `reversible:false`, no THT); one front `kicad:marbastlib/STAB_MX_2u`; one front nice!nano (`include_extra_pins:true`, no `side`/`reversible` setting); one front reset switch (no side/reversible setting). No TRRS or authored mounting-hole parts.
- **Mounts/keep-outs/placeholders:** one generated outline, no explicit keep-outs, no case body. The single main instance carries legacy internal-gasket settings: 4 auto supports, 2 mm minimum wall, 0.05 mm tolerance, 80×3×2 mm gasket, 0.15 compression, 0.1 travel, and an empty closure-mount list. The controller bay and reset remain generated placeholders. No physical fit is established. Archive and measured recipe share parts/layout but differ in the archive’s stored mechanical configuration. [Archive](../../../../content/archives/reviung41-original.boardstudio), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 20. VIK module review

- **Source/positions:** synthetic 3×5, 19.05 mm MX host-switch matrix in `review.rs`; this is a module demonstration, not a measured keyboard. 33 parts and three mounted module examples: VIK splitter above and below, EC11 rotary; plus an embedded Haptic DRV2605L circuit. Matrix switch pose follows a regular grid, not board measurements.
- **Parts/footprints:** 13 solder `ceoloide/switch_mx` (front, `hotswap:false`, `solder:true`), three imported THQWGD001 encoder identities (rotation-only, 2-pin tactile, 4-pin tactile), horizontal VIK host connectors and embedded haptic components from the catalogue. The review includes front/back instances and source STEP model assets in the fixture. Exact imported component footprint references are in `catalogue/modules/imported-modules.json` and `catalogue/parts/imported-parts.json`, with source PCB at `catalogue/modules/sources/sadekbaroudi-vik/pcb/haptic-drv2605l/`.
- **Mounts/keep-outs/placeholders/findings:** no mounting holes or keep-outs are recorded; a 3 mm plate case body with 0.5 mm clearance is in the review fixture. The mounted module definitions contain real imported circuit/component data, but their placement is illustrative (`x=29/57/85`, `y=20`) and review fixture metadata does not establish host connector compatibility, case clearance, fastening or electrical fit. Ask the user to choose a target host and intended module variant before treating these positions as qualified. [Review recipe](../../../../core/examples/demo_projects/review.rs), [module review metadata](../../../../content/layouts/module-review.json), [module catalogue](../../../../catalogue/modules/imported-modules.json).

## Findings by severity and traceability

### 1 — Blocks a build or export

- **None recorded in the reviewed outcomes.** Generator-parameters 02 says all 20 split halves exported to KiCad after restoring closure proposals. This review did not run export/build, so it does not certify the present branch.

### 2 — Physically wrong or fit unresolved

- KLOR closure/bay overlap and non-reflective outline gap repair: [ticket 06](../../generator-parameters/issues/06-paired-half-closure-proposals.md).
- Lily58, Sweep, Chocofi, Sofle RGB/Choc closure-to-bay overlaps in ticket 02; note ticket 06’s later linked-proposal corrections for Lily58, Sweep, Chocofi: [ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md), [ticket 06](../../generator-parameters/issues/06-paired-half-closure-proposals.md).
- Closure-to-switch clearances below 0.5 mm on KLOR, Sweep, TOTEM, Cantor, Sofle RGB/Choc: [ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).
- Sofle v2 gasket geometry and no closure mounts: [ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md), [ticket 04](../issues/04-refresh-included-keyboards.md).
- Shipped independent part headings versus ADR-0010: [ticket 30](../issues/30-independent-parts-use-the-full-heading-rule.md).
- Diode position differs between shipped same-side layout and current literal mirror mode: [ticket 29 Verification](../issues/29-mirror-switch-and-companion-headings.md#verification).
- Pose-only reflection does not reflect off-axis pad geometry: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md), [ticket 27 Verification](../issues/27-mirror-modules.md#pad-layout-is-rotated-not-mirrored-the-on-grid-claim-is-wrong).
- All generated peripheral placeholders and unibody mechanical/hardware fit are unqualified: [content/demos README](../../../../content/demos/README.md), [ticket 04 Outcome](../issues/04-refresh-included-keyboards.md).
- VIK review mounting/electrical/mechanical compatibility remains unknown without selecting target host hardware: [VIK review recipe](../../../../core/examples/demo_projects/review.rs), catalogue sources linked above.

### 3 — Cosmetic or tidy-up

- Replace or clearly label demo-only controller/reset/TRRS/encoder/display placements if the library is intended to communicate actual buildable hardware; do not present generated examples as upstream component placements. [recipes.rs](../../../../core/examples/demo_projects/recipes.rs), [content/demos README](../../../../content/demos/README.md).
- KLOR’s gap repair is a data/geometry parity issue with physical consequences; it is tracked as severity 2 above, not merely cosmetic. [ticket 06](../../generator-parameters/issues/06-paired-half-closure-proposals.md).

## Scope and verification record

Read: `AGENTS.md`; `docs/agents/verification-process.md`; `docs/plans/typed-core-edits/parallel-lanes.md`; ticket 25 and `docs/agents/issue-tracker.md`; ADR-0008 and ADR-0010; generator-parameters tickets 02 and 06; typed-core-edits tickets 04, 20, 27, 29 and 30; `content/demos/README.md`; both measured layout JSON files and `module-review.json`; `core/tests/fixtures/demo-projects-baseline.json`; `content/archives/reviung41-original.boardstudio` (ZIP member `project.json`, inspected in memory); `core/examples/demo_projects/recipes.rs`, `physical.rs`, `review.rs`, and excerpts of `mod.rs`. `core/examples/demo_projects/mechanical.rs`, catalogue imported definitions and footprint generator files were located/searched but not read in full; source paths are linked as provenance, not independently qualified.

Could not determine from repository sources alone: whether generated MCU/reset/TRRS/encoder/display placements correspond to target hardware; whether the closure and switch clearances are acceptable for actual parts/processes; intended diode mode; whether Sofle v2 should remain gasket-mounted; whether pose-only reflection is acceptable for footprints with asymmetric off-axis pads; and whether the archived gasket/placeholder placements fit real hardware. No hardware decision is guessed.

No build, cargo command, export, browser run, recipe/fixture/footprint/code edit, GitNexus refresh, or release check was run. The only verification command for this document is the requested `python3 scripts/check.py repo` run, recorded by the lane agent at completion.
